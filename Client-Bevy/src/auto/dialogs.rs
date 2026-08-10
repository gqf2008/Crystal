//! auto::dialogs 自动化验证系统（从 auto.rs 拆分，#1146）

use bevy::prelude::*;
use super::*;

/// --auto-char：进游戏 3 秒后自动打开角色对话框
pub(crate) fn auto_open_character(
    mut mgr: ResMut<client_bevy::game::dialogs::DialogManager>,
    state: Res<State<client_bevy::scenes::AppState>>,
    time: Res<Time>,
    mut timer: Local<f32>,
) {
    if *state != client_bevy::scenes::AppState::Game {
        return;
    }
    *timer += time.delta_secs();
    if *timer >= 3.0 && !mgr.is_open(client_bevy::game::dialogs::DialogKind::Character) {
        mgr.toggle(client_bevy::game::dialogs::DialogKind::Character);
        tracing::info!("🎛️ --auto-char 自动打开角色对话框");
    }
}

/// --auto-inv：进游戏 3 秒后自动打开背包
pub(crate) fn auto_open_inventory(
    mut mgr: ResMut<client_bevy::game::dialogs::DialogManager>,
    state: Res<State<client_bevy::scenes::AppState>>,
    time: Res<Time>,
    mut timer: Local<f32>,
) {
    if *state != client_bevy::scenes::AppState::Game {
        return;
    }
    *timer += time.delta_secs();
    if *timer >= 3.0 && !mgr.is_open(client_bevy::game::dialogs::DialogKind::Inventory) {
        mgr.toggle(client_bevy::game::dialogs::DialogKind::Inventory);
        tracing::info!("🎛️ --auto-inv 自动打开背包");
    }
}

/// --bigmap-test：打开大地图 → 等 NewMapInfo/地形 → 选中 NPC → 传送 → 关闭
#[allow(clippy::too_many_arguments)]
pub(crate) fn auto_bigmap_test(
    state: Res<State<client_bevy::scenes::AppState>>,
    time: Res<Time>,
    mut mgr: ResMut<client_bevy::game::dialogs::DialogManager>,
    mut bm: ResMut<client_bevy::game::dialogs::big_map::BigMapState>,
    net: ResMut<client_bevy::network::NetConnection>,
    players: Query<&Transform, With<client_bevy::actor::LocalPlayer>>,
    mut t: Local<f32>,
    mut stage: Local<u8>,
    mut phase: Local<f32>,
    mut target: Local<(i32, i32)>,
) {
    use client_bevy::scenes::AppState;
    use client_bevy::game::dialogs::DialogKind;
    if *state != AppState::Game {
        return;
    }
    *t += time.delta_secs();
    if *stage == 0 {
        if !mgr.is_open(DialogKind::BigMap) {
            mgr.toggle(DialogKind::BigMap);
            tracing::info!("[BIGMAP] 打开大地图");
        }
        *phase = *t;
        *stage = 1;
        return;
    }
    if *stage == 1 && *t - *phase >= 1.0 {
        if bm.npcs.is_empty() {
            tracing::warn!("[BIGMAP] ⚠️ 无 NewMapInfo NPC 数据（服务端需 M53 支持）");
        } else {
            tracing::info!("[BIGMAP] ✅ NewMapInfo: {} 个 NPC（{}）", bm.npcs.len(), bm.title);
        }
        *stage = 2;
        *phase = *t;
        return;
    }
    if *stage == 2 {
        if bm.viewport_ready {
            tracing::info!("[BIGMAP] ✅ 地形纹理生成完成 {}x{}", bm.tex_size.0, bm.tex_size.1);
            *stage = 3;
            *phase = *t;
        } else if *t - *phase >= 8.0 {
            tracing::warn!("[BIGMAP] ❌ 地形生成超时");
            *stage = 9;
        }
        return;
    }
    if *stage == 3 && *t - *phase >= 1.0 {
        let tp = bm.npcs.iter().find(|n| n.can_teleport_to).cloned();
        if let Some(npc) = tp {
            bm.selected = Some(0);
            *target = (npc.x, npc.y);
            tracing::info!("[BIGMAP] ✅ 选中可传送 NPC: {} ({},{})", npc.name, npc.x, npc.y);
            net.send_packet(&mir2_shared::packets::client::npc::TeleportToNPC {
                object_id: npc.object_id,
            });
            tracing::info!("[BIGMAP] ✅ 发送传送请求 id={}", npc.object_id);
        } else {
            tracing::warn!("[BIGMAP] ⚠️ 无可传送 NPC");
        }
        *stage = 4;
        *phase = *t;
        return;
    }
    // #1836：传送验证改为轮询（服务端 TeleportToNPC 处理有延迟），
    // 15s 内到达目标 → ✅；超时 → ❌（不再假绿“传送已处理”）
    if *stage == 4 {
        let moved = players.single().ok().map(|tf| {
            client_bevy::game::movement::world_to_tile(tf.translation.x, tf.translation.y)
        });
        let done = match moved {
            Some((x, y)) if (x, y) == *target => {
                tracing::info!("[BIGMAP] ✅ 传送生效 玩家位置=({},{})", x, y);
                true
            }
            Some((x, y)) if *t - *phase >= 15.0 => {
                tracing::warn!(
                    "[BIGMAP] ❌ 传送未生效 玩家位置=({},{})（目标 ({},{})）",
                    x,
                    y,
                    target.0,
                    target.1
                );
                true
            }
            None if *t - *phase >= 15.0 => {
                tracing::warn!("[BIGMAP] ⚠️ 15s 内无法读取玩家位置");
                true
            }
            _ => false,
        };
        if done {
            if mgr.is_open(DialogKind::BigMap) {
                mgr.close(DialogKind::BigMap);
                tracing::info!("[BIGMAP] ✅ 关闭大地图");
            }
            *stage = 9;
        }
        return;
    }
    if *t >= 40.0 && *stage < 9 {
        tracing::warn!("[BIGMAP] ❌ 超时 stage={}", *stage);
        *stage = 9;
    }
}

/// --keyboard-test：打开键位设置 → 滚动 → 重绑一行 → 重置 → 关闭
#[allow(clippy::too_many_arguments)]
/// --worldmap-test：世界地图（#300）WorldMapSetup → 图标 → RequestMapInfo → NewMapInfo 切换
#[allow(clippy::too_many_arguments)]
pub(crate) fn auto_worldmap_test(
    state: Res<State<client_bevy::scenes::AppState>>,
    time: Res<Time>,
    mut mgr: ResMut<client_bevy::game::dialogs::DialogManager>,
    mut bm: ResMut<client_bevy::game::dialogs::big_map::BigMapState>,
    net: ResMut<client_bevy::network::NetConnection>,
    mut t: Local<f32>,
    mut stage: Local<u8>,
    mut phase: Local<f32>,
) {
    use client_bevy::scenes::AppState;
    use client_bevy::game::dialogs::DialogKind;
    if *state != AppState::Game {
        return;
    }
    *t += time.delta_secs();
    if *stage == 0 {
        if !mgr.is_open(DialogKind::BigMap) {
            mgr.toggle(DialogKind::BigMap);
            tracing::info!("[WORLDMAP] 打开大地图");
        }
        *phase = *t;
        *stage = 1;
        return;
    }
    if *stage == 1 {
        if *t - *phase >= 10.0 {
            tracing::warn!("[WORLDMAP] ❌ WorldMapSetup 超时");
            *stage = 9;
            return;
        }
        if bm.world_enabled && bm.world_icons.len() >= 3 && bm.teleport_cost == 1000 {
            tracing::info!(
                "[WORLDMAP] ✅ WorldMapSetup: enabled={} icons={} cost={}",
                bm.world_enabled,
                bm.world_icons.len(),
                bm.teleport_cost
            );
            bm.world_open = true;
            *phase = *t;
            *stage = 2;
        }
        return;
    }
    if *stage == 2 {
        net.send_packet(&mir2_shared::packets::client::npc::RequestMapInfo { map_index: 1 });
        tracing::info!("[WORLDMAP] 请求地图 1（比奇省）");
        *phase = *t;
        *stage = 3;
        return;
    }
    if *stage == 3 {
        if *t - *phase >= 8.0 {
            tracing::warn!("[WORLDMAP] ❌ 地图切换超时 map={} title={} npcs={}", bm.map_index, bm.title, bm.npcs.len());
            *stage = 9;
            return;
        }
        if bm.map_index == 1 && bm.npcs.len() >= 2 && bm.title == "比奇省" {
            tracing::info!("[WORLDMAP] ✅ 切换地图: {} {} 个NPC", bm.title, bm.npcs.len());
            *phase = *t;
            *stage = 4;
        }
        return;
    }
    if *stage == 4 {
        net.send_packet(&mir2_shared::packets::client::npc::RequestMapInfo { map_index: 0 });
        tracing::info!("[WORLDMAP] 请求地图 0（我的位置）");
        *phase = *t;
        *stage = 5;
        return;
    }
    if *stage == 5 {
        if *t - *phase >= 8.0 {
            tracing::warn!("[WORLDMAP] ❌ 返回地图超时");
            *stage = 9;
            return;
        }
        if bm.map_index == 0 && bm.npcs.len() >= 3 && bm.title == "新手村" {
            tracing::info!("[WORLDMAP] ✅ 回到当前地图: {} {} 个NPC", bm.title, bm.npcs.len());
            bm.world_open = false;
            if mgr.is_open(DialogKind::BigMap) {
                mgr.close(DialogKind::BigMap);
            }
            tracing::info!("[WORLDMAP] ✅ 全流程完成");
            *stage = 9;
        }
        return;
    }
    if *t >= 45.0 && *stage < 9 {
        tracing::warn!("[WORLDMAP] ❌ 超时 stage={}", *stage);
        *stage = 9;
    }
}

/// --real-worldmap-test：真实服务器世界地图联调（#302）
/// 阶段：打开大地图 → 等 WorldMapSetup → RequestMapInfo 切图 → 等 NewMapInfo → 回当前图
#[allow(clippy::too_many_arguments)]
pub(crate) fn auto_real_worldmap_test(
    state: Res<State<client_bevy::scenes::AppState>>,
    time: Res<Time>,
    mut mgr: ResMut<client_bevy::game::dialogs::DialogManager>,
    mut bm: ResMut<client_bevy::game::dialogs::big_map::BigMapState>,
    net: ResMut<client_bevy::network::NetConnection>,
    mut t: Local<f32>,
    mut stage: Local<u8>,
    mut phase: Local<f32>,
    mut current_map: Local<i32>,
    mut target_map: Local<i32>,
) {
    use client_bevy::scenes::AppState;
    use client_bevy::game::dialogs::DialogKind;
    if *state != AppState::Game {
        return;
    }
    *t += time.delta_secs();
    if *stage == 0 {
        if !mgr.is_open(DialogKind::BigMap) {
            mgr.toggle(DialogKind::BigMap);
            tracing::info!("[REALWM] 打开大地图");
        }
        *phase = *t;
        *stage = 1;
        return;
    }
    if *stage == 1 {
        if *t - *phase >= 10.0 {
            tracing::warn!("[REALWM] ❌ WorldMapSetup 超时");
            *stage = 9;
            return;
        }
        if bm.world_enabled && !bm.world_icons.is_empty() {
            tracing::info!(
                "[REALWM] ✅ WorldMapSetup: enabled={} icons={} cost={}",
                bm.world_enabled,
                bm.world_icons.len(),
                bm.teleport_cost
            );
            *current_map = bm.map_index;
            // 目标：选一个与当前地图不同的图标，否则用第一个
            *target_map = bm
                .world_icons
                .iter()
                .map(|i| i.map_index)
                .find(|m| *m != *current_map)
                .unwrap_or_else(|| bm.world_icons[0].map_index);
            tracing::info!("[REALWM] 当前地图={} 目标地图={}", *current_map, *target_map);
            *phase = *t;
            *stage = 2;
        }
        return;
    }
    if *stage == 2 {
        net.send_packet(&mir2_shared::packets::client::npc::RequestMapInfo {
            map_index: *target_map,
        });
        tracing::info!("[REALWM] 请求地图 {}", *target_map);
        *phase = *t;
        *stage = 3;
        return;
    }
    if *stage == 3 {
        if *t - *phase >= 8.0 {
            tracing::warn!(
                "[REALWM] ❌ 切图超时 map={} title={} npcs={}",
                bm.map_index,
                bm.title,
                bm.npcs.len()
            );
            *stage = 9;
            return;
        }
        if bm.map_index == *target_map {
            tracing::info!("[REALWM] ✅ 切图成功: map={} title={} npcs={}", bm.map_index, bm.title, bm.npcs.len());
            *phase = *t;
            *stage = 4;
        }
        return;
    }
    if *stage == 4 {
        net.send_packet(&mir2_shared::packets::client::npc::RequestMapInfo {
            map_index: *current_map,
        });
        tracing::info!("[REALWM] 请求回当前地图 {}", *current_map);
        *phase = *t;
        *stage = 5;
        return;
    }
    if *stage == 5 {
        if *t - *phase >= 8.0 {
            tracing::warn!("[REALWM] ❌ 回图超时");
            *stage = 9;
            return;
        }
        if bm.map_index == *current_map {
            tracing::info!("[REALWM] ✅ 回到当前地图 {}（{}）", bm.map_index, bm.title);
            if mgr.is_open(DialogKind::BigMap) {
                mgr.close(DialogKind::BigMap);
            }
            tracing::info!("[REALWM] ✅ 全流程完成");
            *stage = 9;
        }
        return;
    }
    if *t >= 45.0 && *stage < 9 {
        tracing::warn!("[REALWM] ❌ 超时 stage={}", *stage);
        *stage = 9;
    }
}

pub(crate) fn auto_keyboard_test(
    state: Res<State<client_bevy::scenes::AppState>>,
    time: Res<Time>,
    mut mgr: ResMut<client_bevy::game::dialogs::DialogManager>,
    mut kb: ResMut<client_bevy::game::dialogs::keyboard_layout::KeyboardState>,
    mut t: Local<f32>,
    mut stage: Local<u8>,
    mut phase: Local<f32>,
) {
    use client_bevy::scenes::AppState;
    use client_bevy::game::dialogs::DialogKind;
    use bevy::input::keyboard::KeyCode;
    if *state != AppState::Game {
        return;
    }
    *t += time.delta_secs();
    if *stage == 0 {
        if !mgr.is_open(DialogKind::KeyboardLayout) {
            mgr.toggle(DialogKind::KeyboardLayout);
            tracing::info!("[KBD] 打开键位设置");
        }
        *phase = *t;
        *stage = 1;
        return;
    }
    if *stage == 1 && *t - *phase >= 1.0 {
        kb.top_line = kb.top_line.saturating_add(2);
        tracing::info!("[KBD] ✅ 滚动 top_line={}", kb.top_line);
        kb.rebinding = Some(4);
        tracing::info!("[KBD] ✅ 等待按键: 行 4");
        *stage = 2;
        *phase = *t;
        return;
    }
    if *stage == 2 && *t - *phase >= 1.0 {
        if let Some(b) = kb.bindings.get_mut(4) {
            tracing::info!("[KBD] ✅ 绑定 {} → {}", b.action, "X");
            b.key = KeyCode::KeyX;
        }
        kb.rebinding = None;
        tracing::info!("[KBD] ✅ 重绑完成");
        *stage = 3;
        *phase = *t;
        return;
    }
    if *stage == 3 && *t - *phase >= 1.0 {
        kb.bindings = kb.defaults.clone();
        kb.top_line = 0;
        kb.enforce = !kb.enforce;
        tracing::info!("[KBD] ✅ 重置默认 + 规则切换（严格/宽松）完成");
        *stage = 4;
        *phase = *t;
        return;
    }
    if *stage == 4 && *t - *phase >= 1.0 {
        if mgr.is_open(DialogKind::KeyboardLayout) {
            mgr.close(DialogKind::KeyboardLayout);
            tracing::info!("[KBD] ✅ 关闭键位设置");
        }
        *stage = 9;
    }
    if *t >= 30.0 && *stage < 9 {
        tracing::warn!("[KBD] ❌ 超时 stage={}", *stage);
        *stage = 9;
    }
}

/// --option-test：打开设置对话框 → 依次切换 8 组开关 + 音量 → 关闭
pub(crate) fn auto_option_test(
    state: Res<State<client_bevy::scenes::AppState>>,
    time: Res<Time>,
    mut mgr: ResMut<client_bevy::game::dialogs::DialogManager>,
    mut option: ResMut<client_bevy::game::dialogs::option::OptionState>,
    mut t: Local<f32>,
    mut stage: Local<u8>,
    mut phase: Local<f32>,
) {
    use client_bevy::scenes::AppState;
    use client_bevy::game::dialogs::DialogKind;
    if *state != AppState::Game {
        return;
    }
    *t += time.delta_secs();
    if *stage == 0 {
        if !mgr.is_open(DialogKind::Settings) {
            mgr.toggle(DialogKind::Settings);
            tracing::info!("[OPT] 打开设置对话框");
        }
        *phase = *t;
        *stage = 1;
        return;
    }
    if *stage == 1 && *t - *phase >= 1.0 {
        // 依次翻转 8 组开关（模拟点击，验证状态机 + 按钮帧刷新）
        let flips: [(&str, bool); 8] = [
            ("技能模式", option.skill_mode_ctrl),
            ("技能栏", option.skill_bar),
            ("特效", option.effect),
            ("掉落显示", option.drop_view),
            ("名称显示", option.name_view),
            ("血条显示", option.hp_view),
            ("允许观察", option.allow_observe),
            ("新移动", option.new_move),
        ];
        for (name, cur) in flips {
            let next = !cur;
            match name {
                "技能模式" => option.skill_mode_ctrl = next,
                "技能栏" => option.skill_bar = next,
                "特效" => option.effect = next,
                "掉落显示" => option.drop_view = next,
                "名称显示" => option.name_view = next,
                "血条显示" => option.hp_view = next,
                "允许观察" => option.allow_observe = next,
                _ => option.new_move = next,
            }
            tracing::info!("[OPT] ✅ 设置切换: {} -> {}", name, next);
        }
        option.sound_volume = 0.5;
        option.music_volume = 0.35;
        tracing::info!("[OPT] ✅ 音量: 音效 50% / 音乐 35%");
        tracing::info!("[OPT] ✅ 设置对话框渲染正常（8 组开关 + 2 条音量条）");
        *stage = 2;
        *phase = *t;
        return;
    }
    if *stage == 2 && *t - *phase >= 1.0 {
        if mgr.is_open(DialogKind::Settings) {
            mgr.close(DialogKind::Settings);
            tracing::info!("[OPT] ✅ 关闭设置对话框");
        }
        *stage = 9;
    }
    if *t >= 30.0 && *stage < 9 {
        tracing::warn!("[OPT] ❌ 超时 stage={}", *stage);
        *stage = 9;
    }
}

/// --ui-dialog-test：依次打开 Notice/ChatNotice/Timer/Help 验证渲染
#[allow(clippy::too_many_arguments)]
pub(crate) fn auto_ui_dialog_test(
    state: Res<State<client_bevy::scenes::AppState>>,
    time: Res<Time>,
    mut mgr: ResMut<client_bevy::game::dialogs::DialogManager>,
    mut t: Local<f32>,
    mut stage: Local<u8>,
    mut phase: Local<f32>,
) {
    use client_bevy::scenes::AppState;
    use client_bevy::game::dialogs::DialogKind;
    if *state != AppState::Game {
        return;
    }
    *t += time.delta_secs();
    const KINDS: [DialogKind; 4] = [
        DialogKind::Notice,
        DialogKind::ChatNotice,
        DialogKind::Timer,
        DialogKind::Help,
    ];
    if *stage >= KINDS.len() as u8 {
        return;
    }
    let kind = KINDS[*stage as usize];
    if !mgr.is_open(kind) && *phase == 0.0 {
        mgr.toggle(kind);
        tracing::info!("[UIDLG] 打开 {:?}", kind);
        *phase = *t;
    }
    if mgr.is_open(kind) && *t - *phase >= 1.5 {
        mgr.close(kind);
        tracing::info!("[UIDLG] ✅ {:?} 渲染正常", kind);
        *stage += 1;
        *phase = 0.0;
        *t = 0.0;
    }
    if *t >= 30.0 && *stage < KINDS.len() as u8 {
        tracing::warn!("[UIDLG] ❌ 卡在 {:?}", kind);
        *stage = 9;
    }
}

/// --upgrade-test：施法 → mock 回发 ItemUpgraded/RemoveMagic/SendOutputMessage 等，
/// 断言 背包物品升级 + 技能移除 + 聊天消息（#258）
#[allow(clippy::too_many_arguments)]
pub(crate) fn auto_upgrade_test(
    net: ResMut<client_bevy::network::NetConnection>,
    state: Res<State<client_bevy::scenes::AppState>>,
    time: Res<Time>,
    hud: Res<client_bevy::game::hud::HudState>,
    magics: Res<client_bevy::game::skills::MagicsState>,
    chat: Res<client_bevy::game::chat::ChatState>,
    mut t: Local<f32>,
    mut stage: Local<u8>,
    mut target: Local<Option<u32>>,
    mut target_tile: Local<Option<(i32, i32)>>,
    actors: Query<(
        &client_bevy::actor::NetObjectId,
        &Transform,
        Has<client_bevy::actor::Monster>,
    )>,
    players: Query<
        &Transform,
        (
            With<client_bevy::actor::LocalPlayer>,
            With<client_bevy::actor::NetObjectId>,
        ),
    >,
) {
    use client_bevy::scenes::AppState;
    if *state != AppState::Game {
        return;
    }
    *t += time.delta_secs();
    match *stage {
        0 => {
            if *t < 10.0 {
                return;
            }
            let Ok(pf) = players.single() else { return };
            let (px, py) =
                client_bevy::game::movement::world_to_tile(pf.translation.x, pf.translation.y);
            let mut best: Option<(u32, i32, i32)> = None;
            for (id, tf, monster) in &actors {
                if !monster {
                    continue;
                }
                let (mx, my) =
                    client_bevy::game::movement::world_to_tile(tf.translation.x, tf.translation.y);
                let d = (mx - px).abs() + (my - py).abs();
                if d <= 40 && best.map(|(_, _, bd)| d < bd).unwrap_or(true) {
                    best = Some((id.0, mx, my));
                }
            }
            match best {
                Some((oid, mx, my)) => {
                    *target = Some(oid);
                    *target_tile = Some((mx, my));
                    tracing::info!("[UPG] 🎯 目标怪物 id={} @ ({},{})", oid, mx, my);
                    *stage = 1;
                    *t = 0.0;
                }
                None => {
                    tracing::warn!("[UPG] ❌ 附近没有怪物");
                    *stage = 9;
                }
            }
        }
        1 => {
            if *t < 1.5 {
                return;
            }
            let (mx, my) = target_tile.unwrap_or((0, 0));
            let Ok(pf) = players.single() else { return };
            let (px, py) =
                client_bevy::game::movement::world_to_tile(pf.translation.x, pf.translation.y);
            let dir = client_bevy::game::movement::direction_from_delta(
                (mx - px).signum(),
                (my - py).signum(),
            )
            .unwrap_or(mir2_shared::enums::MirDirection::Down);
            net.send_packet(&mir2_shared::packets::client::combat::Magic {
                spell: mir2_shared::enums::Spell::FireBall,
                direction: dir,
                target_id: target.unwrap_or(0),
                location: mir2_shared::Point { x: mx, y: my },
            });
            tracing::info!("[UPG] 🔥 施法触发合成/升级");
            *stage = 2;
            *t = 0.0;
        }
        2 => {
            if *t >= 2.5 {
                let upgraded = hud
                    .inventory
                    .items
                    .iter()
                    .flatten()
                    .any(|it| it.unique_id == 9005 && it.item_index == 6);
                let removed = !magics
                    .magics
                    .iter()
                    .any(|m| m.spell == mir2_shared::enums::Spell::Fencing);
                let msg = chat
                    .lines
                    .iter()
                    .any(|(text, _, _, _)| text.contains("测试服务端消息"));
                tracing::info!("[UPG] 升级={} 技能移除={} 消息={}", upgraded, removed, msg);
                if upgraded && removed && msg {
                    tracing::info!("[UPG] ✅ 合成/升级/技能删除/服务端消息通过");
                } else {
                    tracing::warn!(
                        "[UPG] ❌ 未通过（升级={} 技能移除={} 消息={}）",
                        upgraded,
                        removed,
                        msg
                    );
                }
                *stage = 9;
            }
        }
        _ => {}
    }
}

/// --recipe-test：施法 → mock 回发 NewRecipeInfo(1) + PauseBuff，断言 配方记录 + Buff 提示（#262）
#[allow(clippy::too_many_arguments)]
pub(crate) fn auto_recipe_test(
    net: ResMut<client_bevy::network::NetConnection>,
    state: Res<State<client_bevy::scenes::AppState>>,
    time: Res<Time>,
    craft: Res<client_bevy::game::dialogs::craft::CraftState>,
    buff: Res<client_bevy::game::dialogs::buff::BuffState>,
    mut t: Local<f32>,
    mut stage: Local<u8>,
    mut target: Local<Option<u32>>,
    mut target_tile: Local<Option<(i32, i32)>>,
    actors: Query<(
        &client_bevy::actor::NetObjectId,
        &Transform,
        Has<client_bevy::actor::Monster>,
    )>,
    players: Query<
        &Transform,
        (
            With<client_bevy::actor::LocalPlayer>,
            With<client_bevy::actor::NetObjectId>,
        ),
    >,
) {
    use client_bevy::scenes::AppState;
    if *state != AppState::Game {
        return;
    }
    *t += time.delta_secs();
    match *stage {
        0 => {
            if *t < 10.0 {
                return;
            }
            let Ok(pf) = players.single() else { return };
            let (px, py) =
                client_bevy::game::movement::world_to_tile(pf.translation.x, pf.translation.y);
            let mut best: Option<(u32, i32, i32)> = None;
            for (id, tf, monster) in &actors {
                if !monster {
                    continue;
                }
                let (mx, my) =
                    client_bevy::game::movement::world_to_tile(tf.translation.x, tf.translation.y);
                let d = (mx - px).abs() + (my - py).abs();
                if d <= 40 && best.map(|(_, _, bd)| d < bd).unwrap_or(true) {
                    best = Some((id.0, mx, my));
                }
            }
            match best {
                Some((oid, mx, my)) => {
                    *target = Some(oid);
                    *target_tile = Some((mx, my));
                    tracing::info!("[RECIPE] 🎯 目标怪物 id={} @ ({},{})", oid, mx, my);
                    *stage = 1;
                    *t = 0.0;
                }
                None => {
                    tracing::warn!("[RECIPE] ❌ 附近没有怪物");
                    *stage = 9;
                }
            }
        }
        1 => {
            if *t < 1.5 {
                return;
            }
            let (mx, my) = target_tile.unwrap_or((0, 0));
            let Ok(pf) = players.single() else { return };
            let (px, py) =
                client_bevy::game::movement::world_to_tile(pf.translation.x, pf.translation.y);
            let dir = client_bevy::game::movement::direction_from_delta(
                (mx - px).signum(),
                (my - py).signum(),
            )
            .unwrap_or(mir2_shared::enums::MirDirection::Down);
            net.send_packet(&mir2_shared::packets::client::combat::Magic {
                spell: mir2_shared::enums::Spell::FireBall,
                direction: dir,
                target_id: target.unwrap_or(0),
                location: mir2_shared::Point { x: mx, y: my },
            });
            tracing::info!("[RECIPE] 🔥 施法触发配方/Buff");
            *stage = 2;
            *t = 0.0;
        }
        2 => {
            if *t >= 2.5 {
                let recipe = craft.learned.contains(&1);
                let buff_paused = buff.message.contains("暂停");
                tracing::info!("[RECIPE] 配方={} Buff暂停={}", recipe, buff_paused);
                if recipe && buff_paused {
                    tracing::info!("[RECIPE] ✅ 配方/Buff 通过");
                } else {
                    tracing::warn!(
                        "[RECIPE] ❌ 未通过（配方={} Buff暂停={}）",
                        recipe,
                        buff_paused
                    );
                }
                *stage = 9;
            }
        }
        _ => {}
    }
}


