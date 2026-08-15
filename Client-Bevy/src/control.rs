// ============================================================================
// control.rs 客户端内置控制接口（TCP JSON-RPC，供 MCP/agent 控制玩家）
// 监听 127.0.0.1:9000，每行一条 JSON-RPC。
//   move {dx,dy,run}     相对玩家瓦片偏移移动（dx/dy 为瓦片数）
//   screenshot {path}    保存当前帧截图
//   state {}             返回玩家位置/朝向
//   nearby {}            返回周围实体（含 object_id）
//   attack {object_id}   攻击指定对象
//   interact {object_id} 与指定 NPC 对话
//   pickup {object_id}  拾取指定地面物品
//   chat {message}    发送聊天/GM 命令（@MAKE 等）
//   dialog {kind,action?}  打开/关闭/切换对话框（默认 toggle；验收截图巡回用，#2586）
// ============================================================================

use std::io::{BufRead, BufReader, Write};
use std::net::TcpListener;

use bevy::prelude::*;
use bevy::render::view::screenshot::{save_to_disk, Screenshot};
use crossbeam_channel::{bounded, Receiver, Sender};
use serde_json::{json, Value};

use crate::actor::{
    ActorAnim, GroundItem, LocalPlayer, Monster, MonsterName, NetObjectId, Npc, NpcName, Player,
    PlayerName,
};
use crate::game::dialogs::{DialogKind, DialogManager};
use crate::game::movement::{world_to_tile, LocalMove};
use crate::game::pathfinding;
use crate::game::player_control::ControlState;
use crate::map_renderer::{GameData, GameLibraries};
use crate::network::NetConnection;
use crate::scenes::AppState;

/// 控制命令（控制线程 → Bevy 主循环）
enum ControlCommand {
    Move {
        dx: i32,
        dy: i32,
        run: bool,
    },
    Screenshot {
        path: String,
    },
    GetState {
        reply: Sender<String>,
    },
    Nearby {
        reply: Sender<String>,
    },
    Attack {
        object_id: u32,
    },
    Interact {
        object_id: u32,
    },
    Pickup {
        object_id: u32,
    },
    Chat {
        message: String,
    },
    Dialog {
        kind: DialogKind,
        action: DialogAction,
    },
}

/// dialog 命令的动作（#2586）
enum DialogAction {
    Open,
    Close,
    Toggle,
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
            "chat" => {
                let message = params
                    .get("message")
                    .and_then(|v| v.as_str())
                    .unwrap_or("")
                    .to_string();
                if !message.is_empty() {
                    let _ = tx.send(ControlCommand::Chat { message });
                    json!({"ok": true})
                } else {
                    json!({"error": "missing message"})
                }
            }
            "pickup" => {
                let object_id = params
                    .get("object_id")
                    .and_then(|v| v.as_u64())
                    .unwrap_or(0) as u32;
                if object_id > 0 {
                    let _ = tx.send(ControlCommand::Pickup { object_id });
                    json!({"ok": true})
                } else {
                    json!({"error": "missing object_id"})
                }
            }
            "attack" => {
                let object_id = params
                    .get("object_id")
                    .and_then(|v| v.as_u64())
                    .unwrap_or(0) as u32;
                if object_id > 0 {
                    let _ = tx.send(ControlCommand::Attack { object_id });
                    json!({"ok": true})
                } else {
                    json!({"error": "missing object_id"})
                }
            }
            "interact" => {
                let object_id = params
                    .get("object_id")
                    .and_then(|v| v.as_u64())
                    .unwrap_or(0) as u32;
                if object_id > 0 {
                    let _ = tx.send(ControlCommand::Interact { object_id });
                    json!({"ok": true})
                } else {
                    json!({"error": "missing object_id"})
                }
            }
            "dialog" => {
                let kind = params.get("kind").and_then(|v| v.as_str()).unwrap_or("");
                let action = params
                    .get("action")
                    .and_then(|v| v.as_str())
                    .unwrap_or("toggle")
                    .to_ascii_lowercase();
                match (parse_dialog_kind(kind), action.as_str()) {
                    (Some(k), "open") => {
                        let _ = tx.send(ControlCommand::Dialog {
                            kind: k,
                            action: DialogAction::Open,
                        });
                        json!({"ok": true, "kind": kind, "action": "open"})
                    }
                    (Some(k), "close") => {
                        let _ = tx.send(ControlCommand::Dialog {
                            kind: k,
                            action: DialogAction::Close,
                        });
                        json!({"ok": true, "kind": kind, "action": "close"})
                    }
                    (Some(k), "toggle") => {
                        let _ = tx.send(ControlCommand::Dialog {
                            kind: k,
                            action: DialogAction::Toggle,
                        });
                        json!({"ok": true, "kind": kind, "action": "toggle"})
                    }
                    (Some(_), a) => {
                        json!({"error": format!("unknown action: {a} (open/close/toggle)")})
                    }
                    (None, _) if kind.is_empty() => {
                        json!({"error": "missing kind (snake_case, e.g. inventory)"})
                    }
                    (None, _) => json!({"error": format!("unknown dialog kind: {kind}")}),
                }
            }
            _ => json!({"error": format!("unknown method: {method}")}),
        };

        let resp = json!({"jsonrpc": "2.0", "id": id, "result": result});
        let _ = writeln!(stream, "{resp}");
        let _ = stream.flush();
    }
}

/// snake_case 对话框名 → DialogKind（#2586）。
///
/// 覆盖除 `GuestTrade` 外的全部 46 个变体（`DialogKind` 共 47 个）——
/// `GuestTrade` 由网络 trade 会话与 Trade 成对驱动（dialogs/trade.rs），无独立开关语义，
/// 故不做 RPC 映射（调用会回 unknown dialog kind）。
/// **新增 DialogKind 变体时必须同步本函数、[`has_rpc_mapping`] 与测试名单**
/// （[`has_rpc_mapping`] 的穷尽 match 会让漏改编译失败）。
fn parse_dialog_kind(s: &str) -> Option<DialogKind> {
    use DialogKind as D;
    Some(match s {
        "inventory" => D::Inventory,
        "character" => D::Character,
        "quest_log" => D::QuestLog,
        "settings" => D::Settings,
        "menu" => D::Menu,
        "game_shop" => D::GameShop,
        "minimap" => D::Minimap,
        "npc" => D::Npc,
        "group" => D::Group,
        "friend" => D::Friend,
        "trade" => D::Trade,
        "inspect" => D::Inspect,
        "npc_goods" => D::NpcGoods,
        "guild" => D::Guild,
        "mail" => D::Mail,
        "ranking" => D::Ranking,
        "mentor" => D::Mentor,
        "relationship" => D::Relationship,
        "mount" => D::Mount,
        "report" => D::Report,
        "hero" => D::Hero,
        "hero_inventory" => D::HeroInventory,
        "hero_equipment" => D::HeroEquipment,
        "hero_skill" => D::HeroSkill,
        "creature" => D::Creature,
        "trust_merchant" => D::TrustMerchant,
        "item_rental" => D::ItemRental,
        "guild_territory" => D::GuildTerritory,
        "help" => D::Help,
        "notice" => D::Notice,
        "buff" => D::Buff,
        "fishing" => D::Fishing,
        "socket" => D::Socket,
        "refine" => D::Refine,
        "craft" => D::Craft,
        "dura_status" => D::DuraStatus,
        "npc_drop" => D::NpcDrop,
        "roll" => D::Roll,
        "npc_awake" => D::NpcAwake,
        "timer" => D::Timer,
        "keyboard_layout" => D::KeyboardLayout,
        "big_map" => D::BigMap,
        "chat_notice" => D::ChatNotice,
        "market" => D::Market,
        "storage" => D::Storage,
        "skills" => D::Skills,
        _ => return None,
    })
}

/// 该 DialogKind 是否有 RPC 映射（= parse_dialog_kind 可达）。
///
/// **无通配臂的穷尽 match**：新增 DialogKind 变体而漏改这里会编译失败，堵住
/// 「测试名单自证互异、测不出枚举遗漏」的盲区（批M 审查发现 GuestTrade 即因此漏掉）。
fn has_rpc_mapping(kind: DialogKind) -> bool {
    use DialogKind as D;
    match kind {
        D::Inventory
        | D::Character
        | D::QuestLog
        | D::Settings
        | D::Menu
        | D::GameShop
        | D::Minimap
        | D::Npc
        | D::Group
        | D::Friend
        | D::Trade
        | D::Inspect
        | D::NpcGoods
        | D::Guild
        | D::Mail
        | D::Ranking
        | D::Mentor
        | D::Relationship
        | D::Mount
        | D::Report
        | D::Hero
        | D::HeroInventory
        | D::HeroEquipment
        | D::HeroSkill
        | D::Creature
        | D::TrustMerchant
        | D::ItemRental
        | D::GuildTerritory
        | D::Help
        | D::Notice
        | D::Buff
        | D::Fishing
        | D::Socket
        | D::Refine
        | D::Craft
        | D::DuraStatus
        | D::NpcDrop
        | D::Roll
        | D::NpcAwake
        | D::Timer
        | D::KeyboardLayout
        | D::BigMap
        | D::ChatNotice
        | D::Market
        | D::Storage
        | D::Skills => true,
        // GuestTrade 刻意排除：网络 trade 会话驱动，无独立开关（见 parse_dialog_kind 文档）
        D::GuestTrade => false,
    }
}

fn apply_control_commands(
    mut commands: Commands,
    control: Res<ControlRx>,
    mut control_state: ResMut<ControlState>,
    mut mgr: ResMut<DialogManager>,
    net: Res<NetConnection>,
    time: Res<Time>,
    game_data: Res<GameData>,
    mut libs: ResMut<GameLibraries>,
    players: Query<(Entity, &Transform, &ActorAnim), (With<LocalPlayer>, With<NetObjectId>)>,
    monsters: Query<
        (&Transform, &MonsterName, &NetObjectId),
        (With<Monster>, Without<LocalPlayer>),
    >,
    npcs: Query<(&Transform, &NpcName, &NetObjectId), (With<Npc>, Without<LocalPlayer>)>,
    others: Query<(&Transform, &PlayerName, &NetObjectId), (With<Player>, Without<LocalPlayer>)>,
    items: Query<(&Transform, &GroundItem, &NetObjectId), (With<GroundItem>, Without<LocalPlayer>)>,
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
            ControlCommand::Dialog { kind, action } => {
                match action {
                    DialogAction::Open => mgr.open(kind),
                    DialogAction::Close => mgr.close(kind),
                    DialogAction::Toggle => mgr.toggle(kind),
                }
                tracing::info!("🎮 control dialog: {kind:?} -> open={}", mgr.is_open(kind));
            }
            ControlCommand::Nearby { reply } => {
                let Ok((_, ptf, _)) = players.single() else {
                    let _ = reply.send("{}".to_string());
                    continue;
                };
                let px = ptf.translation.x;
                let py = ptf.translation.y;
                let mut arr = Vec::new();
                for (tf, name, oid) in monsters.iter() {
                    let d =
                        ((tf.translation.x - px).powi(2) + (tf.translation.y - py).powi(2)).sqrt();
                    if d < 600.0 {
                        arr.push(json!({"kind": "monster", "name": name.0, "object_id": oid.0, "x": tf.translation.x, "y": tf.translation.y, "dist": (d as i32)}));
                    }
                }
                for (tf, name, oid) in npcs.iter() {
                    let d =
                        ((tf.translation.x - px).powi(2) + (tf.translation.y - py).powi(2)).sqrt();
                    if d < 600.0 {
                        arr.push(json!({"kind": "npc", "name": name.0, "object_id": oid.0, "x": tf.translation.x, "y": tf.translation.y, "dist": (d as i32)}));
                    }
                }
                for (tf, name, oid) in others.iter() {
                    let d =
                        ((tf.translation.x - px).powi(2) + (tf.translation.y - py).powi(2)).sqrt();
                    if d < 600.0 {
                        arr.push(json!({"kind": "player", "name": name.0, "object_id": oid.0, "x": tf.translation.x, "y": tf.translation.y, "dist": (d as i32)}));
                    }
                }
                for (tf, item, oid) in items.iter() {
                    let d =
                        ((tf.translation.x - px).powi(2) + (tf.translation.y - py).powi(2)).sqrt();
                    if d < 600.0 {
                        arr.push(json!({"kind": "item", "name": item.name, "object_id": oid.0, "x": tf.translation.x, "y": tf.translation.y, "dist": (d as i32)}));
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
            ControlCommand::Attack { object_id } => {
                control_state.attack_target = Some(object_id);
                control_state.last_attack = 0.0;
                if let Ok((pe, _, _)) = players.single() {
                    commands.entity(pe).remove::<LocalMove>();
                }
                tracing::info!("🎮 control attack: {object_id}");
            }
            ControlCommand::Interact { object_id } => {
                control_state.npc_id = Some(object_id);
                control_state.last_npc_call = time.elapsed_secs();
                net.send_packet(&mir2_shared::packets::client::npc::CallNPC {
                    object_id,
                    key: "[@Main]".to_string(),
                });
                tracing::info!("🎮 control interact: {object_id}");
            }
            ControlCommand::Pickup { object_id } => {
                let Ok((pe, ptf, _)) = players.single() else {
                    continue;
                };
                let Some((item_tf, _, _)) = items.iter().find(|(_, _, id)| id.0 == object_id)
                else {
                    tracing::warn!("🎮 control pickup: item {object_id} not found");
                    continue;
                };
                let from = world_to_tile(ptf.translation.x, ptf.translation.y);
                let item_tile = world_to_tile(item_tf.translation.x, item_tf.translation.y);
                let adjacent =
                    (item_tile.0 - from.0).abs() <= 1 && (item_tile.1 - from.1).abs() <= 1;
                if adjacent {
                    net.send_packet(&mir2_shared::packets::client::item::PickUp {});
                    control_state.attack_target = None;
                    tracing::info!("🎮 control pickup: {object_id}");
                } else if let Some(map) = &game_data.map {
                    if let Some(p) = pathfinding::find_path(map, from, item_tile) {
                        if p.is_empty() {
                            tracing::debug!("🎮 control pickup unreachable: {object_id}");
                        } else {
                            let len = p.len();
                            commands.entity(pe).insert(LocalMove {
                                path: p.into(),
                                step_timer_ms: 0.0,
                                run: true,
                                last: None,
                                step_origin: None,
                                turn_acc: 0.0,
                            });
                            control_state.attack_target = None;
                            control_state.pickup_target = Some(object_id);
                            tracing::info!("🎮 control pickup walk: {object_id} ({len} tiles)");
                        }
                    }
                }
            }
            ControlCommand::Chat { message } => {
                tracing::info!("🎮 control chat: {}", message);
                net.send_packet(&mir2_shared::packets::client::chat::Chat {
                    message,
                    linked_items: Vec::new(),
                });
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// parse_dialog_kind 覆盖除 GuestTrade 外全部变体 + 未知返回 None（#2586）
    /// GuestTrade 由网络 trade 会话驱动无独立开关，刻意不做 RPC 映射（批M 审查）
    #[test]
    fn parse_dialog_kind_covers_all_variants() {
        let all = [
            "inventory",
            "character",
            "quest_log",
            "settings",
            "menu",
            "game_shop",
            "minimap",
            "npc",
            "group",
            "friend",
            "trade",
            "inspect",
            "npc_goods",
            "guild",
            "mail",
            "ranking",
            "mentor",
            "relationship",
            "mount",
            "report",
            "hero",
            "hero_inventory",
            "hero_equipment",
            "hero_skill",
            "creature",
            "trust_merchant",
            "item_rental",
            "guild_territory",
            "help",
            "notice",
            "buff",
            "fishing",
            "socket",
            "refine",
            "craft",
            "dura_status",
            "npc_drop",
            "roll",
            "npc_awake",
            "timer",
            "keyboard_layout",
            "big_map",
            "chat_notice",
            "market",
            "storage",
            "skills",
        ];
        // 全部可解析且互不相同（46 个名字一一对应；DialogKind 共 47 个变体，
        // GuestTrade 刻意排除——枚举级穷尽由 has_rpc_mapping 的无通配 match 编译期保证）
        let parsed: Vec<DialogKind> = all.iter().map(|s| parse_dialog_kind(s).unwrap()).collect();
        let uniq: Vec<&DialogKind> = {
            let mut seen: Vec<&DialogKind> = parsed.iter().collect();
            seen.sort_by_key(|k| format!("{k:?}"));
            seen.dedup_by_key(|k| format!("{k:?}"));
            seen
        };
        assert_eq!(all.len(), 46);
        assert_eq!(uniq.len(), all.len(), "46 个名字应映射到 46 个不同变体");
        // 名单与 witness 一致：每个可解析名都有 RPC 映射
        assert!(
            parsed.iter().all(|k| has_rpc_mapping(*k)),
            "名单内全部变体应 has_rpc_mapping"
        );
        // GuestTrade 刻意排除（网络会话驱动）
        assert!(!has_rpc_mapping(DialogKind::GuestTrade));

        // 未知/空/大小写敏感
        assert!(parse_dialog_kind("").is_none());
        assert!(parse_dialog_kind("nope").is_none());
        assert!(
            parse_dialog_kind("Inventory").is_none(),
            "snake_case 小写约定"
        );
    }
}
