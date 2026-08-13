// ============================================================================
// control.rs 客户端内置控制接口（TCP JSON-RPC，供 MCP/agent 控制玩家）
// 监听 127.0.0.1:9000，每行一条 JSON-RPC。
//   move {dx,dy,run}     相对玩家瓦片偏移移动（dx/dy 为瓦片数）
//   screenshot {path}    保存当前帧截图
//   state {}             返回玩家位置/朝向
// ============================================================================

use std::io::{BufRead, BufReader, Write};
use std::net::TcpListener;

use bevy::prelude::*;
use bevy::render::view::screenshot::{save_to_disk, Screenshot};
use crossbeam_channel::{bounded, Receiver, Sender};
use serde_json::{json, Value};

use crate::actor::{
    ActorAnim, LocalPlayer, Monster, MonsterName, NetObjectId, Npc, NpcName, Player, PlayerName,
};
use crate::game::movement::{world_to_tile, LocalMove};
use crate::game::pathfinding;
use crate::map_renderer::{GameData, GameLibraries};
use crate::scenes::AppState;

/// 控制命令（控制线程 → Bevy 主循环）
enum ControlCommand {
    Move { dx: i32, dy: i32, run: bool },
    Screenshot { path: String },
    GetState { reply: Sender<String> },
    Nearby { reply: Sender<String> },
}

#[derive(Resource)]
struct ControlRx(Receiver<ControlCommand>);

pub struct ControlPlugin;

impl Plugin for ControlPlugin {
    fn build(&self, app: &mut App) {
        let (tx, rx) = bounded::<ControlCommand>(64);
        app.insert_resource(ControlRx(rx));
        std::thread::spawn(move || control_listener(tx));
        app.add_systems(
            Update,
            apply_control_commands.run_if(in_state(AppState::Game)),
        );
    }
}

fn control_listener(tx: Sender<ControlCommand>) {
    let listener = match TcpListener::bind("127.0.0.1:9000") {
        Ok(l) => l,
        Err(e) => {
            tracing::warn!("[control] 绑定 9000 失败: {e}");
            return;
        }
    };
    tracing::info!("[control] 监听 127.0.0.1:9000");
    for stream in listener.incoming() {
        let Ok(stream) = stream else { continue };
        let tx = tx.clone();
        std::thread::spawn(move || handle_conn(stream, tx));
    }
}

fn handle_conn(mut stream: std::net::TcpStream, tx: Sender<ControlCommand>) {
    let reader = BufReader::new(match stream.try_clone() {
        Ok(s) => s,
        Err(_) => return,
    });
    for line in reader.lines() {
        let Ok(line) = line else { break };
        let line = line.trim();
        if line.is_empty() {
            continue;
        }
        let Ok(req) = serde_json::from_str::<Value>(line) else {
            let _ = writeln!(stream, "{{\"jsonrpc\":\"2.0\",\"error\":\"parse\"}}");
            continue;
        };
        let id = req.get("id").cloned().unwrap_or(Value::Null);
        let method = req.get("method").and_then(|v| v.as_str()).unwrap_or("");
        let params = req.get("params").cloned().unwrap_or_else(|| json!({}));

        let result: Value = match method {
            "move" => {
                let dx = params.get("dx").and_then(|v| v.as_i64()).unwrap_or(0) as i32;
                let dy = params.get("dy").and_then(|v| v.as_i64()).unwrap_or(0) as i32;
                let run = params.get("run").and_then(|v| v.as_bool()).unwrap_or(true);
                let _ = tx.send(ControlCommand::Move { dx, dy, run });
                json!({"ok": true})
            }
            "screenshot" => {
                let path = params
                    .get("path")
                    .and_then(|v| v.as_str())
                    .unwrap_or("../tools/ctrl_shot.png")
                    .to_string();
                let _ = tx.send(ControlCommand::Screenshot { path });
                json!({"ok": true})
            }
            "nearby" => {
                let (reply_tx, reply_rx) = bounded::<String>(1);
                if tx.send(ControlCommand::Nearby { reply: reply_tx }).is_ok() {
                    let s = reply_rx
                        .recv_timeout(std::time::Duration::from_secs(2))
                        .unwrap_or_else(|_| "{}".to_string());
                    serde_json::from_str::<Value>(&s).unwrap_or_else(|_| json!({}))
                } else {
                    json!({"error": "control channel closed"})
                }
            }
            "state" => {
                let (reply_tx, reply_rx) = bounded::<String>(1);
                if tx
                    .send(ControlCommand::GetState { reply: reply_tx })
                    .is_ok()
                {
                    let s = reply_rx
                        .recv_timeout(std::time::Duration::from_secs(2))
                        .unwrap_or_else(|_| "{}".to_string());
                    serde_json::from_str::<Value>(&s).unwrap_or_else(|_| json!({}))
                } else {
                    json!({"error": "control channel closed"})
                }
            }
            _ => json!({"error": format!("unknown method: {method}")}),
        };

        let resp = json!({"jsonrpc": "2.0", "id": id, "result": result});
        let _ = writeln!(stream, "{resp}");
        let _ = stream.flush();
    }
}

fn apply_control_commands(
    mut commands: Commands,
    control: Res<ControlRx>,
    game_data: Res<GameData>,
    mut libs: ResMut<GameLibraries>,
    players: Query<(Entity, &Transform, &ActorAnim), (With<LocalPlayer>, With<NetObjectId>)>,
    monsters: Query<(&Transform, &MonsterName), (With<Monster>, Without<LocalPlayer>)>,
    npcs: Query<(&Transform, &NpcName), (With<Npc>, Without<LocalPlayer>)>,
    others: Query<(&Transform, &PlayerName), (With<Player>, Without<LocalPlayer>)>,
) {
    while let Ok(cmd) = control.0.try_recv() {
        match cmd {
            ControlCommand::Move { dx, dy, run } => {
                let Ok((pe, ptf, _)) = players.single() else {
                    continue;
                };
                let Some(map) = &game_data.map else { continue };
                let from = world_to_tile(ptf.translation.x, ptf.translation.y);
                let target = (from.0 + dx, from.1 + dy);
                if target == from {
                    continue;
                }
                libs.0.ensure_initialized();
                match pathfinding::find_path(map, from, target) {
                    Some(p) if !p.is_empty() => {
                        commands.entity(pe).insert(LocalMove {
                            path: p.into(),
                            step_timer_ms: 0.0,
                            run,
                            last: None,
                            step_origin: None,
                            turn_acc: 0.0,
                        });
                        tracing::info!(
                            "🎮 control move: ({},{}) -> ({},{}) run={run}",
                            from.0,
                            from.1,
                            target.0,
                            target.1
                        );
                    }
                    _ => tracing::debug!(
                        "🎮 control move unreachable: ({},{}) -> ({},{})",
                        from.0,
                        from.1,
                        target.0,
                        target.1
                    ),
                }
            }
            ControlCommand::Screenshot { path } => {
                tracing::info!("🎮 control screenshot: {path}");
                commands
                    .spawn(Screenshot::primary_window())
                    .observe(save_to_disk(path));
            }
            ControlCommand::Nearby { reply } => {
                let Ok((_, ptf, _)) = players.single() else {
                    let _ = reply.send("{}".to_string());
                    continue;
                };
                let px = ptf.translation.x;
                let py = ptf.translation.y;
                let mut arr = Vec::new();
                for (tf, name) in monsters.iter() {
                    let d =
                        ((tf.translation.x - px).powi(2) + (tf.translation.y - py).powi(2)).sqrt();
                    if d < 600.0 {
                        arr.push(json!({"kind": "monster", "name": name.0, "x": tf.translation.x, "y": tf.translation.y, "dist": (d as i32)}));
                    }
                }
                for (tf, name) in npcs.iter() {
                    let d =
                        ((tf.translation.x - px).powi(2) + (tf.translation.y - py).powi(2)).sqrt();
                    if d < 600.0 {
                        arr.push(json!({"kind": "npc", "name": name.0, "x": tf.translation.x, "y": tf.translation.y, "dist": (d as i32)}));
                    }
                }
                for (tf, name) in others.iter() {
                    let d =
                        ((tf.translation.x - px).powi(2) + (tf.translation.y - py).powi(2)).sqrt();
                    if d < 600.0 {
                        arr.push(json!({"kind": "player", "name": name.0, "x": tf.translation.x, "y": tf.translation.y, "dist": (d as i32)}));
                    }
                }
                arr.sort_by_key(|v| v.get("dist").and_then(|d| d.as_i64()).unwrap_or(0));
                let _ = reply.send(json!({"count": arr.len(), "entities": arr}).to_string());
            }
            ControlCommand::GetState { reply } => {
                let Ok((_, ptf, anim)) = players.single() else {
                    let _ = reply.send("{}".to_string());
                    continue;
                };
                let tile = world_to_tile(ptf.translation.x, ptf.translation.y);
                let s = json!({
                    "x": ptf.translation.x,
                    "y": ptf.translation.y,
                    "tile_x": tile.0,
                    "tile_y": tile.1,
                    "direction": anim.direction,
                })
                .to_string();
                let _ = reply.send(s);
            }
        }
    }
}
