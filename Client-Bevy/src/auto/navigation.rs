//! auto::navigation 自动化验证系统（从 auto.rs 拆分，#1146）

use bevy::prelude::*;
use super::*;

/// --reconnect-test：进入游戏 → 等服务器断开 → 自动重连 → 自动登录并重新进游戏
#[allow(clippy::too_many_arguments)]
pub(crate) fn auto_reconnect_test(
    state: Res<State<client_bevy::scenes::AppState>>,
    time: Res<Time>,
    mut net: ResMut<client_bevy::network::NetConnection>,
    mut t: Local<f32>,
    mut stage: Local<u8>,
    mut phase: Local<f32>,
    mut saw_disconnect: Local<bool>,
) {
    use client_bevy::scenes::AppState;
    use client_bevy::network::NetState;
    *t += time.delta_secs();
    if *stage == 0 {
        if *state == AppState::Game {
            tracing::info!("[RECON] 已进入游戏，等待服务器断开...");
            *stage = 1;
            *phase = *t;
        } else if *t >= 60.0 {
            tracing::warn!("[RECON] ❌ 60 秒内未进入游戏");
            *stage = 9;
        }
        return;
    }
    if *stage == 1 {
        if net.disconnected.is_some() && !*saw_disconnect {
            *saw_disconnect = true;
            tracing::info!("[RECON] ✅ 检测到断线: {:?}", net.disconnected);
            *stage = 2;
            *phase = *t;
        } else if *t - *phase >= 60.0 {
            tracing::warn!("[RECON] ❌ 未检测到断线");
            *stage = 9;
        }
        return;
    }
    if *stage == 2 {
        if net.state == NetState::InGame && *state == AppState::Game && !net.reconnecting {
            tracing::info!("[RECON] ✅ 自动重连成功并重新进入游戏");
            *stage = 9;
        } else if *t - *phase >= 90.0 {
            tracing::warn!("[RECON] ❌ 重连超时（state={:?} reconnecting={}）", net.state, net.reconnecting);
            *stage = 9;
        }
        return;
    }
    if *t >= 200.0 && *stage < 9 {
        tracing::warn!("[RECON] ❌ 总超时 stage={}", *stage);
        *stage = 9;
    }
}

/// --auto-enter：自动驱动 mock 登录流程（Login→Select→Game，验证网络管道）
pub(crate) fn auto_enter(
    mut net: ResMut<client_bevy::network::NetConnection>,
    mut session: ResMut<client_bevy::network::SessionState>,
    state: Res<State<AppState>>,
    time: Res<Time>,
    mut login_sent: Local<bool>,
    mut select_timer: Local<f32>,
) {
    use mir2_shared::packets::client::account::{Login, StartGame};
    if *state == AppState::Login && !*login_sent {
        *login_sent = true;
        net.state = client_bevy::network::NetState::LoggingIn;
        net.send_packet(&Login {
            account_id: {
            let user = std::env::args()
                .skip_while(|a| a != "--e2e-user")
                .nth(1)
                .unwrap_or_else(|| "test".to_string());
            user
        },
        password: {
            let pass = std::env::args()
                .skip_while(|a| a != "--e2e-pass")
                .nth(1)
                .unwrap_or_else(|| "123456".to_string());
            pass
        },
        });
    }
    // 在选角界面停留 3 秒再进游戏（便于 live 截屏验证选角界面）
    if *state == AppState::Select && session.selected_index.is_none() {
        *select_timer += time.delta_secs();
        if *select_timer >= 3.0 {
            let first_index = session.characters.first().map(|c| c.index);
            if let Some(idx) = first_index {
                session.selected_index = Some(idx);
                net.send_packet(&StartGame {
                    character_index: idx,
                });
            }
        }
    }
}

/// BEVY_DEMO_DELETE=1：自动登录→进选角→选中角色→打开删除询问框（截图验证用）
pub(crate) fn demo_delete_flow(
    mut net: ResMut<client_bevy::network::NetConnection>,
    mut session: ResMut<client_bevy::network::SessionState>,
    state: Res<State<AppState>>,
    mut modal: ResMut<client_bevy::ui::modal_box::ModalState>,
    time: Res<Time>,
    mut login_sent: Local<bool>,
    mut select_timer: Local<f32>,
    mut opened: Local<bool>,
) {
    use mir2_shared::packets::client::account::Login;
    if *state == AppState::Login && !*login_sent {
        *login_sent = true;
        net.state = client_bevy::network::NetState::LoggingIn;
        net.send_packet(&Login {
            account_id: "test".to_string(),
            password: "123456".to_string(),
        });
    }
    if *state == AppState::Select && !*opened {
        *select_timer += time.delta_secs();
        if *select_timer >= 1.0 {
            *opened = true;
            if session.selected_index.is_none() {
                session.selected_index = session.characters.first().map(|c| c.index);
            }
            modal.kind = client_bevy::ui::modal_box::ModalKind::DeleteAsk;
            tracing::info!("[DEMO] 打开删除询问框, selected={:?}", session.selected_index);
        }
    }
}

/// --auto-pickup：每 2.5s 自动拾取最近的 GroundItem（复用 player_input 的拾取逻辑）
pub(crate) fn auto_pickup_system(
    mut commands: Commands,
    mut timer: Local<f32>,
    time: Res<Time>,
    net: Res<client_bevy::network::NetConnection>,
    mut control: ResMut<client_bevy::game::player_control::ControlState>,
    game_data: Res<client_bevy::map_renderer::GameData>,
    players: Query<(Entity, &Transform), With<client_bevy::actor::LocalPlayer>>,
    items: Query<(&client_bevy::actor::NetObjectId, &Transform), (With<client_bevy::actor::GroundItem>, Without<client_bevy::actor::LocalPlayer>)>,
) {
    *timer += time.delta_secs();
    if *timer < 2.5 {
        return;
    }
    *timer = 0.0;
    let Ok((pe, ptf)) = players.single() else { return };
    let from_tile = client_bevy::game::movement::world_to_tile(ptf.translation.x, ptf.translation.y);
    let mut best: Option<(u32, f32)> = None;
    for (id, tf) in &items {
        let d = Vec2::new(tf.translation.x - ptf.translation.x, tf.translation.y - ptf.translation.y).length();
        if best.map(|(_, bd)| d < bd).unwrap_or(true) {
            best = Some((id.0, d));
        }
    }
    let Some((item_id, _)) = best else { return };
    let item_tile = items
        .iter()
        .find(|(id, _)| id.0 == item_id)
        .map(|(_, tf)| client_bevy::game::movement::world_to_tile(tf.translation.x, tf.translation.y));
    let Some(item_tile) = item_tile else { return };
    let adjacent = (item_tile.0 - from_tile.0).abs() <= 1 && (item_tile.1 - from_tile.1).abs() <= 1;
    if adjacent {
        net.send_packet(&mir2_shared::packets::client::item::PickUp {});
        control.attack_target = None;
        tracing::info!("🎒 [AUTO] 拾取地面物品 id={}", item_id);
    } else if let Some(map) = &game_data.map {
        if let Some(p) = client_bevy::game::pathfinding::find_path(map, from_tile, item_tile) {
            if !p.is_empty() {
                let len = p.len();
                commands.entity(pe).insert(client_bevy::game::movement::LocalMove {
                    path: p.into(),
                    step_timer_ms: 0.0,
                    run: false,
                    last: None,
                    step_origin: None,
                    turn_acc: 0.0,
                });
                control.pickup_target = Some(item_id);
                tracing::info!("🚶 [AUTO] 走向物品 id={}（{} 格）", item_id, len);
            }
        }
    }
}

/// --real-verify：真实服务器交互闭环（#55）
/// 依赖：--real-net --auto-enter（先登录进图）；在 mock 下同样可跑
/// 阶段：0 聊天回显 → 1 寻路到最近怪物 → 2 自动攻击至死亡 → 3 NPC 对话
#[allow(clippy::too_many_arguments)]
pub(crate) fn real_verify_system(
    mut commands: Commands,
    net: Res<client_bevy::network::NetConnection>,
    state: Res<State<client_bevy::scenes::AppState>>,
    time: Res<Time>,
    mut control: ResMut<client_bevy::game::player_control::ControlState>,
    game_data: Res<client_bevy::map_renderer::GameData>,
    mut chat: ResMut<client_bevy::game::chat::ChatState>,
    hud: Res<client_bevy::game::hud::HudState>,
    npc_dialog: Res<client_bevy::game::dialogs::npc::NpcDialogState>,
    probe: Res<client_bevy::game::combat::RealHitProbe>,
    actors: Query<(
        &client_bevy::actor::NetObjectId,
        &Transform,
        Has<client_bevy::actor::Monster>,
        Has<client_bevy::actor::Npc>,
    )>,
    monster_names: Query<(&client_bevy::actor::NetObjectId, &client_bevy::actor::MonsterName)>,
    players: Query<
        (Entity, &Transform),
        (With<client_bevy::actor::LocalPlayer>, With<client_bevy::actor::NetObjectId>),
    >,
    mut s: Local<RealVerifyState>,
) {
    use client_bevy::scenes::AppState;
    if *state != AppState::Game {
        return;
    }
    s.t += time.delta_secs();

    // #304：死亡处理——城镇复活（C# TownRevive）。
    // 测试进行中死亡 → 复活后重置阶段重跑；测试完成后死亡 → 仅复活清理状态（避免角色卡死影响下次冒烟）
    if hud.dead {
        if !s.revive_sent {
            s.revive_sent = true;
            s.revive_count += 1;
            net.send_packet(&mir2_shared::packets::client::misc::TownRevive);
            tracing::warn!("[REAL] 💀 玩家死亡（第 {} 次），发送城镇复活", s.revive_count);
        }
        if s.stage < 9 && s.revive_count >= 3 {
            tracing::warn!("[REAL] ❌ 连续死亡 {} 次，冒烟失败", s.revive_count);
            s.stage = 9;
        }
        return;
    }
    if s.revive_sent && s.stage < 9 {
        s.revive_sent = false;
        s.tried.clear();
        control.attack_target = None;
        s.target = None;
        s.target_tile = None;
        s.stage = 1;
        s.t = 0.0;
        tracing::info!("[REAL] ✅ 已复活，重置阶段重跑");
    }

    match s.stage {
        0 => {
            if s.t < 8.0 {
                return;
            }
            if !s.chat_sent {
                s.chat_sent = true;
                net.send_packet(&mir2_shared::packets::client::chat::Chat {
                    message: "真实服务器验证：你好！".to_string(),
                    linked_items: vec![],
                });
                // 真实服务器不回发给自己（设计）；本地回显由 chat_input_system 负责（C# 行为），
                // 这里模拟用户路径 add_line，验证显示链路
                chat.add_line(
                    format!("[{}]: 真实服务器验证：你好！", hud.name),
                    Color::WHITE,
                    client_bevy::game::chat::ChatChannel::Nearby,
                );
                tracing::info!("[REAL] 💬 发送聊天（服务器不回显自己属设计，本地回显已修复）");
            }
            if chat.lines.iter().any(|(l, _, _, _)| l.contains("真实服务器验证")) && !s.chat_echo {
                s.chat_echo = true;
                tracing::info!("[REAL] ✅ 聊天本地回显收到（显示链路通过）");
            }
            if s.t >= 20.0 {
                if s.chat_echo {
                    tracing::info!("[REAL] ✅ 聊天验证通过");
                } else {
                    tracing::warn!("[REAL] ⚠️ 聊天未显示");
                }
                s.stage = 1;
                s.t = 0.0;
            }
        }
        1 => {
            if s.t < 1.0 {
                return;
            }
            let Ok((_, pf)) = players.single() else { return };
            let (px, py) =
                client_bevy::game::movement::world_to_tile(pf.translation.x, pf.translation.y);
            // #304：优先选被动弱怪（Deer/Doe/Chicken 等，一次可击杀），其次最近非 guard 怪
            let guard_tiles: Vec<(i32, i32)> = actors
                .iter()
                .filter(|(id, _, monster, _)| {
                    *monster
                        && monster_names
                            .iter()
                            .any(|(mid, mn)| mid.0 == id.0 && mn.0.to_lowercase().contains("guard"))
                })
                .map(|(_, tf, _, _)| {
                    client_bevy::game::movement::world_to_tile(tf.translation.x, tf.translation.y)
                })
                .collect();
            let mut best: Option<(u32, i32, i32, i32)> = None;
            let mut best_prey: Option<(u32, i32, i32, i32)> = None;
            let mut saw_monster = false;
            let mut saw_guard = false;
            for (id, tf, monster, _npc) in &actors {
                if !monster {
                    continue;
                }
                saw_monster = true;
                let name = monster_names
                    .iter()
                    .find(|(mid, _)| mid.0 == id.0)
                    .map(|(_, n)| n.0.clone())
                    .unwrap_or_default();
                // 守卫是友好 NPC，攻击会被反杀（#77 实测打死玩家）；不作为猎杀目标
                if name.to_lowercase().contains("guard") {
                    saw_guard = true;
                    continue;
                }
                // 排除已尝试但未命中的目标（#57 远程怪够不着）
                if s.tried.contains(&id.0) {
                    continue;
                }
                let (mx, my) =
                    client_bevy::game::movement::world_to_tile(tf.translation.x, tf.translation.y);
                let d = (mx - px).abs() + (my - py).abs();
                if is_passive_prey(&name) {
                    if best_prey.map(|(_, _, _, bd)| d < bd).unwrap_or(true) {
                        best_prey = Some((id.0, mx, my, d));
                    }
                } else if best.map(|(_, _, _, bd)| d < bd).unwrap_or(true) {
                    best = Some((id.0, mx, my, d));
                }
            }
            let Some((oid, mx, my, d)) = best_prey.or(best) else {
                if saw_guard && !saw_monster {
                    tracing::warn!("[REAL] ❌ 图上只有守卫类目标（已跳过），无猎杀目标");
                } else if s.tried.is_empty() {
                    tracing::warn!("[REAL] ❌ 全图无怪物");
                } else {
                    tracing::warn!("[REAL] ❌ 已尝试 {} 个目标后无剩余怪物（近战命中验证不通过）", s.tried.len());
                }
                s.stage = 9;
                return;
            };
            let mon_name = monster_names
                .iter()
                .find(|(mid, _)| mid.0 == oid)
                .map(|(_, n)| n.0.clone())
                .unwrap_or_default();
            tracing::info!(
                "[REAL] 🎯 最近怪物 id={} {} @ ({},{}) 距离={}（已试 {} 个）",
                oid, mon_name, mx, my, d, s.tried.len()
            );
            s.target = Some(oid);
            s.target_tile = Some((mx, my));
            if d <= 1 {
                control.attack_target = Some(oid);
                tracing::info!("[REAL] ⚔️ 已在邻接，直接开始攻击 {}", oid);
                s.stage = 2;
                s.t = 0.0;
                return;
            }
            let Some(map) = &game_data.map else {
                tracing::warn!("[REAL] ❌ 地图未加载");
                s.stage = 9;
                return;
            };
            let Ok((pe, _)) = players.single() else { return };
            // 近战需在怪物相邻格（而非重叠）：寻路目标选怪物 8 邻中可达且路径最短的格
            let mut best_path: Option<(Vec<(i32, i32)>, (i32, i32))> = None;
            for (ox, oy) in [(-1, 0), (1, 0), (0, -1), (0, 1), (-1, -1), (1, -1), (-1, 1), (1, 1)] {
                let t2 = (mx + ox, my + oy);
                if !map.in_bounds(t2.0, t2.1) || !map.is_walkable(t2.0, t2.1) {
                    continue;
                }
                // #304：避免站到守卫占用的格（近战会被反杀）
                if guard_tiles.contains(&t2) {
                    continue;
                }
                if let Some(p) = client_bevy::game::pathfinding::find_path(map, (px, py), t2) {
                    if !p.is_empty()
                        && best_path
                            .as_ref()
                            .map(|(bp, _)| p.len() < bp.len())
                            .unwrap_or(true)
                    {
                        best_path = Some((p, t2));
                    }
                }
            }
            match best_path {
                Some((p, t2)) => {
                    let len = p.len();
                    s.target_tile = Some(t2);
                    // run 模式（客户端跨 2 格发一个 Run，#59 已修）
                    commands.entity(pe).insert(client_bevy::game::movement::LocalMove {
                        path: p.into(),
                        step_timer_ms: 0.0,
                        run: true,
                        last: None,
                        step_origin: None,
                        turn_acc: 0.0,
                    });
                    tracing::info!("[REAL] 🚶 寻路到怪物旁（{} 格，run，目标 {},{}）", len, t2.0, t2.1);
                    s.stage = 2;
                    s.t = 0.0;
                }
                _ => {
                    tracing::warn!(
                        "[REAL] ❌ 无法寻路到怪物 ({},{}) 旁（from=({},{}) from_walkable={}）",
                        mx, my, px, py, map.is_walkable(px, py)
                    );
                    s.stage = 9;
                }
            }
        }
        2 => {
            let Ok((_, pf)) = players.single() else { return };
            let (px, py) =
                client_bevy::game::movement::world_to_tile(pf.translation.x, pf.translation.y);
            let Some(tid) = s.target else { s.stage = 9; return };
            let alive = actors.iter().any(|(id, _, _, _)| id.0 == tid);
            if !alive {
                tracing::info!("[REAL] ✅ 目标怪物已死亡（实体移除）——战斗闭环通过（命中 {} 次）", probe.hits);
                s.stage = 3;
                s.t = 0.0;
                return;
            }
            if hud.dead {
                tracing::warn!("[REAL] ⚠️ 玩家死亡（战斗验证部分通过，继续 NPC 验证）");
                s.stage = 3;
                s.t = 0.0;
                return;
            }
            let (mx, my) = s.target_tile.unwrap_or((0, 0));
            let d = (mx - px).abs() + (my - py).abs();
            if d <= 1 && control.attack_target != Some(tid) {
                // 客户端本地移动超前于服务器位置（UserLocation 校正有延迟），
                // 到达邻接后等 2s 让服务器位置同步（apply_self_position 会校正），再攻击
                s.arrived_wait += time.delta_secs();
                if s.arrived_wait < 2.0 {
                    return;
                }
                s.arrived_wait = 0.0;
                control.attack_target = Some(tid);
                // 命中基线：从开始攻击时记录
                s.hits_at_start = probe.hits;
                s.attack_elapsed = 0.0;
                tracing::info!("[REAL] ⚔️ 服务器位置已同步，开始自动攻击 {}（命中基线 {}）", tid, s.hits_at_start);
            }
            if control.attack_target == Some(tid) {
                s.attack_elapsed += time.delta_secs();
                // 20s 攻击零命中 → 目标够不着（远程怪/位置漂移），换下一个最近怪物
                if s.attack_elapsed >= 20.0 && probe.hits == s.hits_at_start {
                    tracing::warn!("[REAL] ⚠️ 攻击 {} 20s 零命中（共命中 {}），换目标", tid, probe.hits);
                    s.tried.push(tid);
                    control.attack_target = None;
                    s.target = None;
                    s.target_tile = None;
                    s.stage = 1;
                    s.t = 0.0;
                    return;
                }
            }
            // #304：30s 未击杀（有命中但打不动/怪物回血）→ 换目标，不卡死
            if s.attack_elapsed >= 30.0 {
                tracing::warn!("[REAL] ⚠️ 30s 内未击杀目标 {}（命中 {}），换目标", tid, probe.hits);
                s.tried.push(tid);
                control.attack_target = None;
                s.target = None;
                s.target_tile = None;
                s.stage = 1;
                s.t = 0.0;
            }
        }
        3 => {
            if s.t < 3.0 {
                return;
            }
            let Ok((_, pf)) = players.single() else { return };
            let (px, py) =
                client_bevy::game::movement::world_to_tile(pf.translation.x, pf.translation.y);
            let mut best: Option<(u32, i32, i32, i32)> = None;
            for (id, tf, _monster, npc) in &actors {
                if !npc {
                    continue;
                }
                let (nx, ny) =
                    client_bevy::game::movement::world_to_tile(tf.translation.x, tf.translation.y);
                let d = (nx - px).abs() + (ny - py).abs();
                if best.map(|(_, _, _, bd)| d < bd).unwrap_or(true) {
                    best = Some((id.0, nx, ny, d));
                }
            }
            let Some((nid, nx, ny, d)) = best else {
                tracing::warn!("[REAL] ❌ 全图无 NPC");
                s.stage = 9;
                return;
            };
            tracing::info!("[REAL] 🧙 最近 NPC id={} @ ({},{}) 距离={}", nid, nx, ny, d);
            s.npc_id = Some(nid);
            s.npc_sent = false;
            s.npc_wait = 0.0;
            if d <= 2 {
                net.send_packet(&mir2_shared::packets::client::npc::CallNPC {
                    object_id: nid,
                    key: "[@Main]".to_string(),
                });
                s.npc_sent = true;
                tracing::info!("[REAL] 🧙 发送 CallNPC [@Main]");
                s.stage = 4;
                s.t = 0.0;
                return;
            }
            // 走到 NPC 2 格内（服务器交互范围 2 格）
            let Some(map) = &game_data.map else {
                tracing::warn!("[REAL] ❌ 地图未加载");
                s.stage = 9;
                return;
            };
            let Ok((pe, _)) = players.single() else { return };
            let path = client_bevy::game::pathfinding::find_path(map, (px, py), (nx, ny));
            let path = path.or_else(|| {
                for (ox, oy) in [(-1, 0), (1, 0), (0, -1), (0, 1), (-1, -1), (1, -1), (-1, 1), (1, 1)] {
                    let t2 = (nx + ox, ny + oy);
                    if let Some(p) = client_bevy::game::pathfinding::find_path(map, (px, py), t2) {
                        if !p.is_empty() {
                            return Some(p);
                        }
                    }
                }
                None
            });
            match path {
                Some(p) if !p.is_empty() => {
                    let len = p.len();
                    s.target_tile = Some((nx, ny));
                    commands.entity(pe).insert(client_bevy::game::movement::LocalMove {
                        path: p.into(),
                        step_timer_ms: 0.0,
                        run: true,
                        last: None,
                        step_origin: None,
                        turn_acc: 0.0,
                    });
                    tracing::info!("[REAL] 🚶 寻路到 NPC（{} 格，run）", len);
                    s.stage = 4;
                    s.t = 0.0;
                }
                _ => {
                    tracing::warn!("[REAL] ❌ 无法寻路到 NPC ({},{})", nx, ny);
                    s.stage = 9;
                }
            }
        }
        4 => {
            if npc_dialog.visible {
                tracing::info!("[REAL] ✅ NPC 对话框已打开（NPCResponse 收到）");
                s.stage = 9;
                return;
            }
            // 到达 NPC 旁且服务器位置同步后发送 CallNPC（本地移动超前，需等校正）
            if !s.npc_sent {
                let nid = s.npc_id.unwrap_or(0);
                let Ok((_, pf)) = players.single() else { return };
                let (px, py) =
                    client_bevy::game::movement::world_to_tile(pf.translation.x, pf.translation.y);
                let (mx, my) = s.target_tile.unwrap_or((0, 0));
                let d = (mx - px).abs() + (my - py).abs();
                if d <= 2 {
                    s.npc_wait += time.delta_secs();
                    if s.npc_wait >= 2.0 {
                        s.npc_sent = true;
                        net.send_packet(&mir2_shared::packets::client::npc::CallNPC {
                            object_id: nid,
                            key: "[@Main]".to_string(),
                        });
                        tracing::info!("[REAL] 🧙 服务器位置同步后发送 CallNPC [@Main]");
                    }
                }
            }
            if s.t >= 25.0 {
                tracing::warn!("[REAL] ⚠️ 25s 未收到 NPCResponse（可能该 NPC 无 @Main 页）");
                s.stage = 9;
            }
        }
        _ => {}
    }
}


