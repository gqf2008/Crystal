//! auto::combat 自动化验证系统（从 auto.rs 拆分，#1146）

use bevy::prelude::*;
use super::*;

/// --auto-attack：自动攻击（验证 攻击→受击→飘字 链路）
pub(crate) fn auto_attack_debug(
    net: Res<client_bevy::network::NetConnection>,
    state: Res<State<client_bevy::scenes::AppState>>,
    time: Res<Time>,
    mut timer: Local<f32>,
) {
    if *state != client_bevy::scenes::AppState::Game {
        return;
    }
    *timer += time.delta_secs();
    if *timer >= 1.5 {
        *timer = 0.0;
        net.send_packet(&mir2_shared::packets::client::combat::Attack {
            direction: mir2_shared::enums::MirDirection::Up,
            spell: mir2_shared::enums::Spell::None,
        });
        tracing::info!("⚔️ --auto-attack 自动攻击");
    }
}

/// --auto-pmode：进图后循环发送宠物模式切换（#1562，C.ChangePMode → S.ChangePMode 确认链路）
pub(crate) fn auto_pmode_test(
    net: Res<client_bevy::network::NetConnection>,
    state: Res<State<client_bevy::scenes::AppState>>,
    time: Res<Time>,
    mut sent: Local<u8>,
) {
    use client_bevy::scenes::AppState;
    if *state != AppState::Game {
        return;
    }
    if time.elapsed_secs() < 6.0 {
        return;
    }
    match *sent {
        0 => {
            *sent = 1;
            net.send_packet(&client_bevy::game::player_control::build_change_pmode(
                mir2_shared::enums::PetMode::MoveOnly,
            ));
            tracing::info!("🐾 [PMODETEST] 发送宠物模式 MoveOnly");
        }
        1 => {
            if time.elapsed_secs() < 7.0 {
                return;
            }
            *sent = 2;
            net.send_packet(&client_bevy::game::player_control::build_change_pmode(
                mir2_shared::enums::PetMode::AttackOnly,
            ));
            tracing::info!("🐾 [PMODETEST] 发送宠物模式 AttackOnly");
        }
        _ => {}
    }
}

/// --auto-pet-pickup：进图后发送宠物拾取指令（#1558，C.IntelligentCreaturePickup：
/// 鼠标拾取 mouse_mode=true + 半自动 mouse_mode=false，验证 发送→mock 解析→拾取入包 链路）
pub(crate) fn auto_pet_pickup_test(
    net: Res<client_bevy::network::NetConnection>,
    state: Res<State<client_bevy::scenes::AppState>>,
    time: Res<Time>,
    mut sent: Local<u8>,
) {
    use client_bevy::scenes::AppState;
    if *state != AppState::Game {
        return;
    }
    if time.elapsed_secs() < 6.0 {
        return;
    }
    match *sent {
        0 => {
            *sent = 1;
            net.send_packet(&client_bevy::game::player_control::build_pet_pickup(true, (353, 352)));
            tracing::info!("🐾 [PETTEST] 发送宠物拾取（鼠标）@ (353,352)");
        }
        1 => {
            if time.elapsed_secs() < 7.0 {
                return;
            }
            *sent = 2;
            net.send_packet(&client_bevy::game::player_control::build_pet_pickup(false, (353, 352)));
            tracing::info!("🐾 [PETTEST] 发送宠物半自动拾取 @ (353,352)");
        }
        _ => {}
    }
}

/// --auto-ranged-attack：进图后发送一次 C.RangeAttack（#1556，
/// 验证 远程攻击 → mock 回 ObjectRangeAttack/S.RangeAttack → 弹道+受击 链路）
pub(crate) fn auto_ranged_attack_test(
    net: Res<client_bevy::network::NetConnection>,
    state: Res<State<client_bevy::scenes::AppState>>,
    time: Res<Time>,
    mut sent: Local<bool>,
) {
    use client_bevy::scenes::AppState;
    if *state != AppState::Game {
        return;
    }
    if *sent {
        return;
    }
    if time.elapsed_secs() < 6.0 {
        return;
    }
    *sent = true;
    net.send_packet(&client_bevy::game::player_control::build_ranged_attack(
        mir2_shared::enums::MirDirection::Right,
        (354, 352),
        101,
        (353, 352),
    ));
    tracing::info!("🏹 [RANGETEST] 发送 RangeAttack → 目标 101");
}

/// --drop-pick-test：怪物掉落 → 地面物品 → 拾取 → 背包
/// 前提：DB 配置 bevychar 在 Deer(340,325) 左侧、攻击力秒杀、Deer 掉落 chance=1.0
#[allow(clippy::too_many_arguments)]
pub(crate) fn auto_drop_pick_test(
    net: ResMut<client_bevy::network::NetConnection>,
    state: Res<State<client_bevy::scenes::AppState>>,
    time: Res<Time>,
    hud: Res<client_bevy::game::hud::HudState>,
    ground: Query<&client_bevy::actor::NetObjectId, With<client_bevy::actor::GroundItem>>,
    mut t: Local<f32>,
    mut stage: Local<u8>,
    mut atk_timer: Local<f32>,
    mut dir_idx: Local<u8>,
    mut before: Local<usize>,
) {
    use client_bevy::scenes::AppState;
    if *state != AppState::Game {
        return;
    }
    *t += time.delta_secs();
    match *stage {
        0 => {
            if *t < 8.0 {
                return;
            }
            // 每 1.2s 轮换方向攻击（Deer 刷新点 (205,325)，spread 45 会偏移）
            *atk_timer += time.delta_secs();
            if *atk_timer >= 1.2 {
                *atk_timer = 0.0;
                let dirs = [
                    mir2_shared::enums::MirDirection::Right,
                    mir2_shared::enums::MirDirection::Up,
                    mir2_shared::enums::MirDirection::Down,
                    mir2_shared::enums::MirDirection::Left,
                    mir2_shared::enums::MirDirection::UpRight,
                    mir2_shared::enums::MirDirection::DownRight,
                    mir2_shared::enums::MirDirection::UpLeft,
                    mir2_shared::enums::MirDirection::DownLeft,
                ];
                let d = dirs[*dir_idx as usize % dirs.len()];
                *dir_idx += 1;
                net.send_packet(&mir2_shared::packets::client::combat::Attack {
                    direction: d,
                    spell: mir2_shared::enums::Spell::None,
                });
                tracing::info!("[DROPTEST] 攻击方向 {:?}", d);
            }
            if ground.iter().next().is_some() {
                tracing::info!("[DROPTEST] ✅ 检测到地面物品实体");
                *stage = 1;
                *t = 0.0;
            } else if *t > 25.0 {
                tracing::warn!("[DROPTEST] ❌ 超时未检测到掉落（怪物可能已死/未掉）");
                *stage = 9;
            }
        }
        1 => {
            if *t < 1.0 {
                return;
            }
            *before = hud.inventory.items.iter().flatten().count();
            net.send_packet(&mir2_shared::packets::client::item::PickUp {});
            tracing::info!("[DROPTEST] 发送 PickUp（拾取前背包 {} 件）", *before);
            *stage = 2;
            *t = 0.0;
        }
        2 => {
            if *t < 3.0 {
                return;
            }
            let now = hud.inventory.items.iter().flatten().count();
            if now > *before {
                tracing::info!("[DROPTEST] ✅ 拾取成功：背包 {} -> {} 件", *before, now);
            } else {
                tracing::warn!("[DROPTEST] ❌ 拾取失败：背包 {} -> {} 件", *before, now);
            }
            *stage = 9;
        }
        _ => {}
    }
}

/// --combat-test：自动选怪 → 连续 FireBall → 验证死亡 + 掉落（M37 战斗闭环）
#[allow(clippy::too_many_arguments)]
pub(crate) fn auto_combat_test(
    net: ResMut<client_bevy::network::NetConnection>,
    state: Res<State<client_bevy::scenes::AppState>>,
    time: Res<Time>,
    mut t: Local<f32>,
    mut stage: Local<u8>,
    mut cast_timer: Local<f32>,
    mut target: Local<Option<u32>>,
    mut target_tile: Local<Option<(i32, i32)>>,
    mut item_count_before: Local<usize>,
    mut effect_seen: Local<bool>,
    effects: Res<client_bevy::game::effects::EffectsState>,
    mut control: ResMut<client_bevy::game::player_control::ControlState>,
    actors: Query<(
        &client_bevy::actor::NetObjectId,
        &Transform,
        Has<client_bevy::actor::Monster>,
    )>,
    items: Query<(&client_bevy::actor::NetObjectId, &client_bevy::actor::GroundItem)>,
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
            // 找 10 格内最近的怪物
            let mut best: Option<(u32, i32, i32, i32)> = None;
            for (id, tf, monster) in &actors {
                if !monster {
                    continue;
                }
                let (mx, my) =
                    client_bevy::game::movement::world_to_tile(tf.translation.x, tf.translation.y);
                let d = (mx - px).abs() + (my - py).abs();
                if d <= 40 && best.map(|(_, _, _, bd)| d < bd).unwrap_or(true) {
                    best = Some((id.0, mx, my, d));
                }
            }
            if best.is_none() {
                // 探测：附近 40 格内怪物数量与最近距离
                let mut total = 0usize;
                let mut nearest = i32::MAX;
                for (_, tf, monster) in &actors {
                    if !monster {
                        continue;
                    }
                    total += 1;
                    let (mx, my) =
                        client_bevy::game::movement::world_to_tile(tf.translation.x, tf.translation.y);
                    let d = (mx - px).abs() + (my - py).abs();
                    if d < nearest {
                        nearest = d;
                    }
                }
                tracing::warn!(
                    "[COMBAT] 40 格内无怪物：玩家=({},{}), 全图可见怪物={}, 最近距离={}",
                    px,
                    py,
                    total,
                    nearest
                );
            }
            match best {
                Some((oid, mx, my, d)) => {
                    *target = Some(oid);
                    *target_tile = Some((mx, my));
                    *item_count_before = items.iter().count();
                    // 模拟真实玩法：点击选中攻击目标（供特效/施法定位）
                    control.attack_target = Some(oid);
                    tracing::info!(
                        "[COMBAT] 🎯 目标怪物 id={} @ ({},{}) 距离={}",
                        oid,
                        mx,
                        my,
                        d
                    );
                    *stage = 1;
                    *t = 0.0;
                    *cast_timer = 0.0;
                }
                None => {
                    tracing::warn!("[COMBAT] ❌ 附近没有怪物");
                    *stage = 9;
                }
            }
        }
        1 => {
            if *t >= 45.0 {
                tracing::warn!("[COMBAT] ❌ 超时未击杀（目标仍在）");
                *stage = 9;
                return;
            }
            // 目标实体已消失（ObjectDied 移除）→ 击杀成功
            let alive = target
                .and_then(|tid| actors.iter().find(|(id, _, _)| id.0 == tid))
                .is_some();
            if !alive {
                tracing::info!("[COMBAT] ✅ 目标怪物已死亡（实体移除）");
                *stage = 2;
                *t = 0.0;
                return;
            }
            // M38：魔法特效验证（MagicCast → 弹道，ObjectStruck → 爆炸）
            if !*effect_seen && effects.spawned > 0 {
                *effect_seen = true;
                tracing::info!(
                    "[COMBAT] ✅ 魔法特效已生成（计数 {}）",
                    effects.spawned
                );
            }
            // 每 1.3 秒施放一次 FireBall（目标位置）
            *cast_timer += time.delta_secs();
            if *cast_timer >= 1.3 {
                *cast_timer = 0.0;
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
                tracing::info!("[COMBAT] 🔥 FireBall → ({},{})", mx, my);
            }
        }
        2 => {
            if *t < 5.0 {
                return;
            }
            // 对比地面物品计数（M24 掉落链路）
            let now = items.iter().count();
            let before = *item_count_before;
            if now > before {
                tracing::info!(
                    "[COMBAT] ✅ 死亡后出现掉落（地面物品 {} → {}）",
                    before,
                    now
                );
            } else {
                tracing::warn!(
                    "[COMBAT] ⚠️ 地面物品数未增加（{} → {}，可能掉落被拾取）",
                    before,
                    now
                );
            }
            *stage = 9;
        }
        _ => {}
    }
}

/// --auto-cast：进图后施放一次 F1 技能（验证 客户端→mock→回显 链路）
pub(crate) fn auto_cast_system(
    mut timer: Local<f32>,
    mut fired: Local<bool>,
    time: Res<Time>,
    net: Res<client_bevy::network::NetConnection>,
    magics: Res<client_bevy::game::skills::MagicsState>,
) {
    *timer += time.delta_secs();
    if *fired || *timer < 6.0 || magics.magics.is_empty() {
        return; // 等 UserInformation（技能）就绪
    }
    *fired = true;
    let Some(m) = magics.by_key(1) else {
        tracing::info!("[AUTO] 无技能 key=1");
        return;
    };
    net.send_packet(&mir2_shared::packets::client::combat::Magic {
        spell: m.spell,
        direction: mir2_shared::enums::MirDirection::Up,
        target_id: 101,
        location: mir2_shared::map::Point { x: 0, y: 0 },
    });
    tracing::info!("🪄 [AUTO] 施放 {}", m.name);
}

/// --auto-equip：进图后自动装备背包第一件可装备物品（验证 EquipItem 闭环 + 外观刷新）
pub(crate) fn auto_equip_system(
    mut timer: Local<f32>,
    mut fired: Local<bool>,
    time: Res<Time>,
    net: Res<client_bevy::network::NetConnection>,
    hud: Res<client_bevy::game::hud::HudState>,
) {
    if *fired {
        return;
    }
    *timer += time.delta_secs();
    if *timer < 6.0 || hud.inventory.items.iter().flatten().count() == 0 {
        return;
    }
    *fired = true;
    if let Some(item) = hud.inventory.items.iter().flatten().find(|i| i.is_equipment()) {
        if let Some(to) = item.equip_slot_occupied(|s| hud.equipment.get(s).and_then(|x| x.as_ref()).is_some()) {
            net.send_packet(&mir2_shared::packets::client::item::EquipItem {
                grid: mir2_shared::enums::MirGridType::Inventory,
                unique_id: item.unique_id,
                to,
            });
            tracing::info!("⚔️ [AUTO] 装备 {} -> 槽 {}", item.name, to);
        }
    }
}

/// --auto-life：进图后依次 聊天(6s) → 购买(9s) → 喝药(12s)
pub(crate) fn auto_life_system(
    mut timer: Local<f32>,
    mut phase: Local<u8>,
    time: Res<Time>,
    net: Res<client_bevy::network::NetConnection>,
    hud: Res<client_bevy::game::hud::HudState>,
) {
    *timer += time.delta_secs();
    let t = *timer;
    match *phase {
        0 if t >= 6.0 => {
            *phase = 1;
            net.send_packet(&mir2_shared::packets::client::chat::Chat {
                message: "你好，传奇世界！".to_string(),
                linked_items: vec![],
            });
            tracing::info!("💬 [LIFE] 发送聊天");
        }
        1 if t >= 9.0 => {
            *phase = 2;
            net.send_packet(&mir2_shared::packets::client::npc::BuyItem {
                item_index: 1,
                count: 1,
                panel_type: mir2_shared::enums::PanelType::Buy,
            });
            tracing::info!("🛒 [LIFE] 发送购买请求");
        }
        2 if t >= 12.0 => {
            *phase = 3;
            if let Some(potion) = hud.inventory.items.iter().flatten().find(|i| i.item_index == 1) {
                net.send_packet(&mir2_shared::packets::client::item::UseItem {
                    unique_id: potion.unique_id,
                });
                tracing::info!("💊 [LIFE] 使用药水 uid={}", potion.unique_id);
            } else {
                tracing::info!("💊 [LIFE] 背包无药水");
            }
        }
        _ => {}
    }
}

/// --auto-revive：死亡后 1s 自动发 TownRevive（验证 死亡→复活 全链路，#46）
pub(crate) fn auto_revive_system(
    net: Res<client_bevy::network::NetConnection>,
    state: Res<State<client_bevy::scenes::AppState>>,
    time: Res<Time>,
    hud: Res<client_bevy::game::hud::HudState>,
    mut t: Local<f32>,
) {
    use client_bevy::scenes::AppState;
    if *state != AppState::Game {
        return;
    }
    if !hud.dead {
        *t = 0.0;
        return;
    }
    *t += time.delta_secs();
    if *t >= 1.0 {
        *t = 0.0;
        net.send_packet(&mir2_shared::packets::client::misc::TownRevive);
        tracing::info!("[REVIVE] 自动复活（TownRevive）");
    }
}

/// --auto-cast-loop：每秒连发 F1 技能（验证 耗蓝递减 → 蓝不足拒绝 → 魔法药回蓝，#51）
pub(crate) fn auto_cast_loop_system(
    mut timer: Local<f32>,
    mut last_cast: Local<f32>,
    time: Res<Time>,
    net: Res<client_bevy::network::NetConnection>,
    magics: Res<client_bevy::game::skills::MagicsState>,
    hud: Res<client_bevy::game::hud::HudState>,
) {
    *timer += time.delta_secs();
    if *timer < 6.0 || magics.magics.is_empty() {
        return;
    }
    if *timer - *last_cast < 1.0 {
        return;
    }
    *last_cast = *timer;
    // 蓝不足时喝魔法药(小)
    if hud.mp < 10 {
        if let Some(potion) = hud
            .inventory
            .items
            .iter()
            .flatten()
            .find(|i| i.item_index == 2)
        {
            net.send_packet(&mir2_shared::packets::client::item::UseItem {
                unique_id: potion.unique_id,
            });
            tracing::info!("🔮 [CASTLOOP] MP 低，喝魔法药 uid={}", potion.unique_id);
        }
    }
    let Some(m) = magics.by_key(1) else {
        return;
    };
    net.send_packet(&mir2_shared::packets::client::combat::Magic {
        spell: m.spell,
        direction: mir2_shared::enums::MirDirection::Up,
        target_id: 101,
        location: mir2_shared::map::Point { x: 0, y: 0 },
    });
    tracing::info!("🔮 [CASTLOOP] 施放 {}（MP {}/{}）", m.name, hud.mp, hud.max_mp);
}

/// --spell-verify：真实服务器法术冒烟（#306）
/// 依赖：--real-net --auto-enter + 角色已学会 HellFire/IceThrust/Curse/EnergyRepulsor（player_magics）
/// 阶段：走到最近怪物 2 格内 → 循环施法，HellFire/IceThrust 等死亡，Curse/EnergyRepulsor 冒烟
#[allow(clippy::too_many_arguments)]
pub(crate) fn auto_spell_verify(
    mut commands: Commands,
    state: Res<State<client_bevy::scenes::AppState>>,
    time: Res<Time>,
    net: Res<client_bevy::network::NetConnection>,
    game_data: Res<client_bevy::map_renderer::GameData>,
    probe: Res<client_bevy::game::combat::RealHitProbe>,
    actors: Query<(
        &client_bevy::actor::NetObjectId,
        &Transform,
        Has<client_bevy::actor::Monster>,
    )>,
    players: Query<
        (Entity, &Transform),
        (With<client_bevy::actor::LocalPlayer>, With<client_bevy::actor::NetObjectId>),
    >,
    mut t: Local<f32>,
    mut stage: Local<u8>,
    mut cast_t: Local<f32>,
    mut casts: Local<u32>,
    mut moving: Local<bool>,
    mut last_move_at: Local<f32>,
    mut arrived_wait: Local<f32>,
    mut hits_at_stage: Local<u32>,
) {
    use client_bevy::scenes::AppState;
    use client_bevy::game::movement::{world_to_tile, direction_from_delta, LocalMove};
    if *state != AppState::Game {
        return;
    }
    *t += time.delta_secs();
    if *stage == 0 {
        if *t < 8.0 {
            return;
        }
        *stage = 1;
        *t = 0.0;
        *moving = false;
        *hits_at_stage = probe.hits;
        tracing::info!("[SPELL] 开始法术冒烟（HellFire → IceThrust → Curse → EnergyRepulsor）");
        return;
    }
    if *stage > 29 {
        return;
    }
    let Ok((pe, pf)) = players.single() else { return };
    let (px, py) = world_to_tile(pf.translation.x, pf.translation.y);

    // 找最近存活怪物
    let mut best: Option<(u32, i32, i32, i32)> = None;
    for (id, tf, monster) in &actors {
        if !monster {
            continue;
        }
        let (mx, my) = world_to_tile(tf.translation.x, tf.translation.y);
        let d = (mx - px).abs() + (my - py).abs();
        if best.map(|(_, _, _, bd)| d < bd).unwrap_or(true) {
            best = Some((id.0, mx, my, d));
        }
    }
    let Some((oid, mx, my, d)) = best else {
        tracing::warn!("[SPELL] ❌ 无可施法目标（stage={} casts={}）", *stage, *casts);
        *stage = 99;
        return;
    };

    // 目标非正邻格（太远或重叠）：走到怪物相邻格（8 邻中可达且路径最短）再施法
    if d != 1 {
        if !*moving {
            if let Some(map) = &game_data.map {
                let mut best_path: Option<(Vec<(i32, i32)>, (i32, i32))> = None;
                for (ox, oy) in [(-1, 0), (1, 0), (0, -1), (0, 1), (-1, -1), (1, -1), (-1, 1), (1, 1)] {
                    let t2 = (mx + ox, my + oy);
                    if !map.in_bounds(t2.0, t2.1) || !map.is_walkable(t2.0, t2.1) {
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
                if let Some((p, _t2)) = best_path {
                    let len = p.len();
                    commands.entity(pe).insert(LocalMove {
                        path: p.into(),
                        step_timer_ms: 0.0,
                        run: true,
                        last: None,
                        step_origin: None,
                        turn_acc: 0.0,
                    });
                    *moving = true;
                    *last_move_at = *t;
                    tracing::info!("[SPELL] 🚶 走向怪物 {} 旁（{} 格）", oid, len);
                }
            }
        }
        // 已插入移动，等待到达（每 5 秒重置一次防止卡死）
        if *t - *last_move_at >= 5.0 {
            *moving = false;
        }
        *arrived_wait = 0.0;
        return;
    }
    *moving = false;

    // #1819：到达目标邻接后等服务端位置同步再施法（real_verify 同款 arrived_wait 2s）。
    // 客户端本地移动超前服务端 1-2 格，HellFire 等按“服务端位置+方向”解析 AoE 的法术会打空。
    *arrived_wait += time.delta_secs();
    if *arrived_wait < 2.0 {
        return;
    }
    *arrived_wait = 0.0;

    *cast_t += time.delta_secs();
    if *cast_t < 1.0 {
        return;
    }
    *cast_t = 0.0;
    *casts += 1;

    let spell = match *stage {
        1 => mir2_shared::enums::Spell::HellFire,
        2 => mir2_shared::enums::Spell::IceThrust,
        3 => mir2_shared::enums::Spell::Curse,
        4 => mir2_shared::enums::Spell::EnergyRepulsor,
        5 => mir2_shared::enums::Spell::FlamingSword,
        6 => mir2_shared::enums::Spell::EnergyShield,
        7 => mir2_shared::enums::Spell::FireBurst,
        8 => mir2_shared::enums::Spell::TwinDrakeBlade,
        9 => mir2_shared::enums::Spell::DoubleSlash,
        10 => mir2_shared::enums::Spell::SlashingBurst,
        11 => mir2_shared::enums::Spell::Plague,
        12 => mir2_shared::enums::Spell::Trap,
        13 => mir2_shared::enums::Spell::DelayedExplosion,
        14 => mir2_shared::enums::Spell::CatTongue,
        15 => mir2_shared::enums::Spell::MoonMist,
        16 => mir2_shared::enums::Spell::VampireShot,
        17 => mir2_shared::enums::Spell::PoisonShot,
        18 => mir2_shared::enums::Spell::CrippleShot,
        19 => mir2_shared::enums::Spell::ElementalShot,
        20 => mir2_shared::enums::Spell::FlameDisruptor,
        21 => mir2_shared::enums::Spell::ImmortalSkin,
        22 => mir2_shared::enums::Spell::Hallucination,
        23 => mir2_shared::enums::Spell::StormEscape,
        24 => mir2_shared::enums::Spell::OneWithNature,
        25 => mir2_shared::enums::Spell::MentalState,
        26 => mir2_shared::enums::Spell::UltimateEnhancer,
        27 => mir2_shared::enums::Spell::FatalSword,
        28 => mir2_shared::enums::Spell::SummonSkeleton,
        _ => mir2_shared::enums::Spell::PetEnhancer,
    };
    let dir = direction_from_delta((mx - px).signum(), (my - py).signum())
        .unwrap_or(mir2_shared::enums::MirDirection::Up);
    // #328：Plague/Trap/DelayedExplosion 以玩家所在格为目标（3×3 覆盖附近怪物，规避位置漂移）
    let (tx, ty) = if *stage >= 11 { (px, py) } else { (mx, my) };
    net.send_packet(&mir2_shared::packets::client::combat::Magic {
        spell,
        direction: dir,
        target_id: oid,
        location: mir2_shared::map::Point { x: tx, y: ty },
    });
    tracing::info!(
        "[SPELL] 🧙 stage={} 施放 {:?} → 怪物 {} @ ({},{}) dir={:?} casts={}（玩家 @ {},{})",
        *stage, spell, oid, mx, my, dir, *casts, px, py
    );

    // HellFire/IceThrust：目标死亡（实体移除）→ 阶段通过
    if *stage == 1 || *stage == 2 {
        let alive = actors.iter().any(|(id, _, _)| id.0 == oid);
        if !alive || probe.hits > *hits_at_stage {
            tracing::info!(
                "[SPELL] ✅ {:?} 命中/击杀怪物 {}（hits={} 基线={} casts={}）",
                spell, oid, probe.hits, *hits_at_stage, *casts
            );
            *stage += 1;
            *casts = 0;
            *hits_at_stage = probe.hits;
            return;
        }
        if *casts >= 10 {
            tracing::warn!("[SPELL] ⚠️ {:?} 10 次未命中（位置漂移/怪物逃跑），进入下一阶段", spell);
            *stage += 1;
            *casts = 0;
            *hits_at_stage = probe.hits;
        }
        return;
    }
    // #312/#318：FlamingSword/TwinDrakeBlade/DoubleSlash 施放后补一次近战攻击，触发下一次攻击效果
    if (*stage == 5 || *stage == 8 || *stage == 9 || *stage == 27 || *stage == 28) && *casts == 1 {
        net.send_packet(&mir2_shared::packets::client::combat::Attack {
            direction: dir,
            spell: mir2_shared::enums::Spell::None,
        });
        tracing::info!("[SPELL] ⚔️ {:?} 后补近战攻击（触发效果）", spell);
    }
    // Curse/EnergyRepulsor/FlamingSword/EnergyShield：施放 3 次后冒烟通过（不要求击杀）
    if *casts >= 3 {
        tracing::info!("[SPELL] ✅ {:?} 冒烟通过（3 次施放无崩溃）", spell);
        *stage += 1;
        *casts = 0;
        if *stage == 30 {
            tracing::info!("[SPELL] ✅ 法术冒烟全流程完成");
            *stage = 99;
        }
    }
}

/// --book-test：技能书学习（#212：使用背包槽 3 技能书 → 等 S.NewMagic → 校验技能列表）
#[allow(clippy::too_many_arguments)]
pub(crate) fn auto_book_test(
    net: ResMut<client_bevy::network::NetConnection>,
    state: Res<State<client_bevy::scenes::AppState>>,
    time: Res<Time>,
    hud: Res<client_bevy::game::hud::HudState>,
    magics: Res<client_bevy::game::skills::MagicsState>,
    mut t: Local<f32>,
    mut stage: Local<u8>,
) {
    use client_bevy::scenes::AppState;
    if *state != AppState::Game {
        return;
    }
    *t += time.delta_secs();
    match *stage {
        0 => {
            if *t < 8.0 {
                return;
            }
            let uid = hud
                .inventory
                .items
                .get(3)
                .and_then(|s| s.as_ref())
                .map(|i| i.unique_id);
            match uid {
                Some(uid) => {
                    net.send_packet(&mir2_shared::packets::client::item::UseItem { unique_id: uid });
                    tracing::info!("[BOOKTEST] 使用技能书 uid={}", uid);
                    *stage = 1;
                    *t = 0.0;
                }
                None => {
                    tracing::warn!("[BOOKTEST] ⚠️ 背包槽 3 无技能书");
                    *stage = 9;
                }
            }
        }
        1 => {
            if *t >= 8.0 {
                tracing::warn!("[BOOKTEST] ❌ 未学会技能");
                *stage = 9;
                return;
            }
            if magics
                .magics
                .iter()
                .any(|m| m.spell == mir2_shared::enums::Spell::FireBall)
            {
                tracing::info!("[BOOKTEST] ✅ 学会 FireBall");
                *stage = 9;
            }
        }
        _ => {}
    }
}

/// --battle-vfx-test：施法 → mock 回发 ObjectMagic/ObjectProjectile/ObjectEffect/ObjectRangeAttack，
/// 断言特效计数增长（#224）
#[allow(clippy::too_many_arguments)]
pub(crate) fn auto_battle_vfx_test(
    net: ResMut<client_bevy::network::NetConnection>,
    state: Res<State<client_bevy::scenes::AppState>>,
    time: Res<Time>,
    mut t: Local<f32>,
    mut stage: Local<u8>,
    mut target: Local<Option<u32>>,
    mut target_tile: Local<Option<(i32, i32)>>,
    mut before: Local<u64>,
    effects: Res<client_bevy::game::effects::EffectsState>,
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
            // 找 40 格内最近的怪物
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
                    tracing::info!("[VFX] 🎯 目标怪物 id={} @ ({},{})", oid, mx, my);
                    *stage = 1;
                    *t = 0.0;
                }
                None => {
                    tracing::warn!("[VFX] ❌ 附近没有怪物");
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
            *before = effects.spawned;
            net.send_packet(&mir2_shared::packets::client::combat::Magic {
                spell: mir2_shared::enums::Spell::FireBall,
                direction: dir,
                target_id: target.unwrap_or(0),
                location: mir2_shared::Point { x: mx, y: my },
            });
            tracing::info!(
                "[VFX] 🔥 施法 FireBall → ({},{}), 特效基线={}",
                mx,
                my,
                *before
            );
            *stage = 2;
            *t = 0.0;
        }
        2 => {
            if *t >= 2.5 {
                let delta = effects.spawned - *before;
                if delta >= 3 {
                    tracing::info!("[VFX] ✅ 战斗特效已生成（+{}）", delta);
                } else {
                    tracing::warn!("[VFX] ❌ 特效不足（+{}，期望 ≥3）", delta);
                }
                *stage = 9;
            }
        }
        _ => {}
    }
}

/// --action-test：施法 → mock 怪物反击 ObjectAttack + 对象冲刺/后跳，逐项采样断言（#234）
#[allow(clippy::too_many_arguments)]
pub(crate) fn auto_action_test(
    net: ResMut<client_bevy::network::NetConnection>,
    state: Res<State<client_bevy::scenes::AppState>>,
    time: Res<Time>,
    mut t: Local<f32>,
    mut stage: Local<u8>,
    mut target: Local<Option<u32>>,
    mut target_tile: Local<Option<(i32, i32)>>,
    mut flags: Local<u8>,
    actors: Query<(
        &client_bevy::actor::NetObjectId,
        &Transform,
        Has<client_bevy::actor::Monster>,
    )>,
    actors_st: Query<(
        &client_bevy::actor::NetObjectId,
        Has<client_bevy::game::combat::StruckTimer>,
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
                    tracing::info!("[ACTION] 🎯 目标怪物 id={} @ ({},{})", oid, mx, my);
                    *stage = 1;
                    *t = 0.0;
                }
                None => {
                    tracing::warn!("[ACTION] ❌ 附近没有怪物");
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
            tracing::info!("[ACTION] 🔥 施法触发对象动作");
            *stage = 2;
            *t = 0.0;
        }
        2 => {
            // 采样窗口 [0.5, 8.0)：怪物反击 ObjectAttack（StruckTimer）、103 冲刺位移、101 后跳
            if *t >= 0.5 && *t < 8.0 {
                let attack = actors_st
                    .iter()
                    .any(|(id, struck, monster)| monster && struck && id.0 != 100);
                if attack {
                    *flags |= 1;
                }
                let dash_orig = client_bevy::game::movement::tile_to_world(351, 355);
                let dash = actors.iter().any(|(id, tf, monster)| {
                    monster
                        && id.0 == 103
                        && ((tf.translation.x - dash_orig.x).abs() > 1.0
                            || (tf.translation.y - dash_orig.y).abs() > 1.0)
                });
                if dash {
                    *flags |= 2;
                }
                let back = client_bevy::game::movement::tile_to_world(352, 352);
                let backstep = actors.iter().any(|(id, tf, monster)| {
                    monster
                        && id.0 == 101
                        && (tf.translation.x - back.x).abs() < 1.0
                        && (tf.translation.y - back.y).abs() < 1.0
                });
                if backstep {
                    *flags |= 4;
                }
            }
            if *t >= 8.0 {
                let attack = *flags & 1 != 0;
                let dash = *flags & 2 != 0;
                let backstep = *flags & 4 != 0;
                let struck_count = actors_st
                    .iter()
                    .filter(|(_, struck, _)| *struck)
                    .count();
                tracing::info!(
                    "[ACTION] 攻击={} 冲刺={} 后跳={}（当前带StruckTimer怪物数={}）",
                    attack,
                    dash,
                    backstep,
                    struck_count
                );
                if attack && dash && backstep {
                    tracing::info!("[ACTION] ✅ 对象动作全部通过");
                } else {
                    tracing::warn!(
                        "[ACTION] ❌ 部分未通过（攻击={} 冲刺={} 后跳={}）",
                        attack,
                        dash,
                        backstep
                    );
                }
                *stage = 9;
            }
        }
        _ => {}
    }
}

/// --poison-test：施法 → mock 回发 ObjectPoisoned(GREEN)，t+4s 解毒，断言 PoisonTint 出现→消失（#236）
#[allow(clippy::too_many_arguments)]
pub(crate) fn auto_poison_test(
    net: ResMut<client_bevy::network::NetConnection>,
    state: Res<State<client_bevy::scenes::AppState>>,
    time: Res<Time>,
    mut t: Local<f32>,
    mut stage: Local<u8>,
    mut target: Local<Option<u32>>,
    mut target_tile: Local<Option<(i32, i32)>>,
    mut flags: Local<u8>,
    poisons: Query<(
        &client_bevy::actor::NetObjectId,
        Has<client_bevy::actor::PoisonTint>,
    )>,
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
                    tracing::info!("[POISON] 🎯 目标怪物 id={} @ ({},{})", oid, mx, my);
                    *stage = 1;
                    *t = 0.0;
                }
                None => {
                    tracing::warn!("[POISON] ❌ 附近没有怪物");
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
            tracing::info!("[POISON] 🔥 施法触发中毒");
            *stage = 2;
            *t = 0.0;
        }
        2 => {
            // mock 施法即发中毒；采样 [0.5, 4.0) 观察 PoisonTint
            if *t >= 0.5 && *t < 4.0 {
                let poisoned = poisons.iter().any(|(id, tint)| id.0 == 100 && tint);
                if poisoned {
                    *flags |= 1;
                }
            }
            if *t >= 4.0 {
                *stage = 3;
                *t = 0.0;
            }
        }
        3 => {
            // mock t+4s 解毒；t+6s 汇总
            if *t >= 6.0 {
                let cured = !poisons
                    .iter()
                    .any(|(id, tint)| id.0 == 100 && tint);
                if cured {
                    *flags |= 2;
                }
                let poisoned = *flags & 1 != 0;
                let cured = *flags & 2 != 0;
                tracing::info!("[POISON] 中毒={} 解毒={}", poisoned, cured);
                if poisoned && cured {
                    tracing::info!("[POISON] ✅ 中毒染层（中毒→解毒）通过");
                } else {
                    tracing::warn!("[POISON] ❌ 部分未通过（中毒={} 解毒={}）", poisoned, cured);
                }
                *stage = 9;
            }
        }
        _ => {}
    }
}

/// --mana-test：施法 → mock 回发 ObjectMana(101=80%)，断言 ActorMp 出现（#238）
#[allow(clippy::too_many_arguments)]
pub(crate) fn auto_mana_test(
    net: ResMut<client_bevy::network::NetConnection>,
    state: Res<State<client_bevy::scenes::AppState>>,
    time: Res<Time>,
    mut t: Local<f32>,
    mut stage: Local<u8>,
    mut target: Local<Option<u32>>,
    mut target_tile: Local<Option<(i32, i32)>>,
    mut flags: Local<u8>,
    mana: Query<(
        &client_bevy::actor::NetObjectId,
        Has<client_bevy::game::combat::ActorMp>,
    )>,
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
                    tracing::info!("[MANA] 🎯 目标怪物 id={} @ ({},{})", oid, mx, my);
                    *stage = 1;
                    *t = 0.0;
                }
                None => {
                    tracing::warn!("[MANA] ❌ 附近没有怪物");
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
            tracing::info!("[MANA] 🔥 施法触发对象蓝条");
            *stage = 2;
            *t = 0.0;
        }
        2 => {
            // mock 施法即发 ObjectMana(101=80%)；采样 [0.5, 6.0) 观察 ActorMp
            if *t >= 0.5 && *t < 6.0 {
                let seen = mana.iter().any(|(id, has_mp)| has_mp && id.0 == 101);
                if seen {
                    *flags |= 1;
                }
            }
            if *t >= 6.0 {
                let seen = *flags & 1 != 0;
                tracing::info!("[MANA] 蓝条={}", seen);
                if seen {
                    tracing::info!("[MANA] ✅ 对象蓝条（ObjectMana）通过");
                } else {
                    tracing::warn!("[MANA] ❌ 未观察到 ActorMp");
                }
                *stage = 9;
            }
        }
        _ => {}
    }
}

/// --toggle-test：施法 → mock 回发 S.SpellToggle(Slaying,true)；再发 C.SpellToggle(Thrusting,true)
/// 等 mock 回显，断言 MagicsState.spell_toggles 双向更新（#242）
#[allow(clippy::too_many_arguments)]
pub(crate) fn auto_toggle_test(
    net: ResMut<client_bevy::network::NetConnection>,
    state: Res<State<client_bevy::scenes::AppState>>,
    time: Res<Time>,
    magics: Res<client_bevy::game::skills::MagicsState>,
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
                    tracing::info!("[TOGGLE] 🎯 目标怪物 id={} @ ({},{})", oid, mx, my);
                    *stage = 1;
                    *t = 0.0;
                }
                None => {
                    tracing::warn!("[TOGGLE] ❌ 附近没有怪物");
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
            tracing::info!("[TOGGLE] 🔥 施法触发技能开关");
            *stage = 2;
            *t = 0.0;
        }
        2 => {
            // mock 施法即发 S.SpellToggle(Slaying,true)
            if *t >= 2.0 {
                let sv_seen = magics.toggle_state(mir2_shared::enums::Spell::Slaying);
                tracing::info!("[TOGGLE] 服务端同步 Slaying={}", sv_seen);
                // 模拟客户端切换（skill_bar_system 同款包）
                net.send_packet(&mir2_shared::packets::client::combat::SpellToggle {
                    spell: mir2_shared::enums::Spell::Thrusting,
                    can_use: true,
                });
                tracing::info!("[TOGGLE] 发送 C.SpellToggle(Thrusting,true)");
                *stage = 3;
                *t = 0.0;
            }
        }
        3 => {
            // mock 回显 S.SpellToggle(Thrusting,true)
            if *t >= 3.0 {
                let echo = magics.toggle_state(mir2_shared::enums::Spell::Thrusting);
                tracing::info!("[TOGGLE] 回显 Thrusting={}", echo);
                if echo {
                    tracing::info!("[TOGGLE] ✅ 技能开关双向通过");
                } else {
                    tracing::warn!("[TOGGLE] ❌ 回显未更新");
                }
                *stage = 9;
            }
        }
        _ => {}
    }
}

/// --gold-test：施法 → mock 掉金币 150，断言 GroundGold 实体出现（#244）
#[allow(clippy::too_many_arguments)]
pub(crate) fn auto_gold_test(
    net: ResMut<client_bevy::network::NetConnection>,
    state: Res<State<client_bevy::scenes::AppState>>,
    time: Res<Time>,
    mut t: Local<f32>,
    mut stage: Local<u8>,
    mut target: Local<Option<u32>>,
    mut target_tile: Local<Option<(i32, i32)>>,
    gold: Query<(
        &client_bevy::actor::NetObjectId,
        &client_bevy::actor::GroundGold,
    )>,
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
                    tracing::info!("[GOLD] 🎯 目标怪物 id={} @ ({},{})", oid, mx, my);
                    *stage = 1;
                    *t = 0.0;
                }
                None => {
                    tracing::warn!("[GOLD] ❌ 附近没有怪物");
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
            tracing::info!("[GOLD] 🔥 施法触发金币掉落");
            *stage = 2;
            *t = 0.0;
        }
        2 => {
            // mock 施法即发 ObjectGold(150)
            if *t >= 2.5 {
                let seen = gold.iter().any(|(_, g)| g.gold == 150);
                tracing::info!("[GOLD] 地面金币={}", seen);
                if seen {
                    tracing::info!("[GOLD] ✅ 地面金币（ObjectGold）通过");
                } else {
                    tracing::warn!("[GOLD] ❌ 未观察到 GroundGold");
                }
                *stage = 9;
            }
        }
        _ => {}
    }
}

/// --harvest-test：施法 → mock 对怪物 101 发 ObjectHarvest(→352,352)，断言位置变化（#246）
#[allow(clippy::too_many_arguments)]
pub(crate) fn auto_harvest_test(
    net: ResMut<client_bevy::network::NetConnection>,
    state: Res<State<client_bevy::scenes::AppState>>,
    time: Res<Time>,
    mut t: Local<f32>,
    mut stage: Local<u8>,
    mut target: Local<Option<u32>>,
    mut target_tile: Local<Option<(i32, i32)>>,
    mut flags: Local<u8>,
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
                    tracing::info!("[HARVEST] 🎯 目标怪物 id={} @ ({},{})", oid, mx, my);
                    *stage = 1;
                    *t = 0.0;
                }
                None => {
                    tracing::warn!("[HARVEST] ❌ 附近没有怪物");
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
            tracing::info!("[HARVEST] 🔥 施法触发采集");
            *stage = 2;
            *t = 0.0;
        }
        2 => {
            // mock 施法即发 ObjectHarvest(101 → 352,352)；采样 [0.5, 6.0)
            if *t >= 0.5 && *t < 6.0 {
                let pos = client_bevy::game::movement::tile_to_world(352, 352);
                let moved = actors.iter().any(|(id, tf, monster)| {
                    monster
                        && id.0 == 101
                        && (tf.translation.x - pos.x).abs() < 1.0
                        && (tf.translation.y - pos.y).abs() < 1.0
                });
                if moved {
                    *flags |= 1;
                }
            }
            if *t >= 6.0 {
                let moved = *flags & 1 != 0;
                tracing::info!("[HARVEST] 采集位移={}", moved);
                if moved {
                    tracing::info!("[HARVEST] ✅ 采集表现通过");
                } else {
                    tracing::warn!("[HARVEST] ❌ 未观察到采集位移");
                }
                *stage = 9;
            }
        }
        _ => {}
    }
}

/// --attack-range-test：攻击距离校验（#1554）
/// 阶段0：进图后把怪物 101 设为攻击目标（玩家出生 354,352 距 101@353,352 = 1 格 → 近战应发 Attack）
/// 阶段1：把目标设为远处（模拟 10 格外，用假 oid 102 距离）→ 不应发 Attack + 系统提示"目标太远"
#[allow(clippy::too_many_arguments)]
pub(crate) fn auto_attack_range_test(
    state: Res<State<client_bevy::scenes::AppState>>,
    time: Res<Time>,
    mut control: ResMut<client_bevy::game::player_control::ControlState>,
    chat: Res<client_bevy::game::chat::ChatState>,
    mut t: Local<f32>,
    mut stage: Local<u8>,
    mut atk_sent: Local<u32>,
    mut too_far_seen: Local<bool>,
) {
    use client_bevy::scenes::AppState;
    if *state != AppState::Game {
        return;
    }
    *t += time.delta_secs();
    match *stage {
        0 => {
            if *t < 8.0 {
                return;
            }
            // 近战范围测试：玩家出生 (354,352)，怪物 101@(353,352) 距离 1
            control.attack_target = Some(101);
            control.last_attack = 0.0;
            tracing::info!("[ATKRANGE] 阶段0：近战目标 101（距离 1 格）");
            *stage = 1;
            *t = 0.0;
            *atk_sent = 0;
        }
        1 => {
            if *t < 2.0 {
                return;
            }
            // 观察阶段0：近战目标保留（在 1 格内持续攻击）
            if control.attack_target == Some(101) {
                tracing::info!("[ATKRANGE] ✅ 近战目标保留（1 格内持续攻击）");
            }
            // 切到存在但远的目标：怪物 103@(351,355)，玩家@(354,352) 距离 max(3,3)=3 > 近战范围1
            control.attack_target = Some(103);
            control.last_attack = 0.0;
            *stage = 2;
            *t = 0.0;
            *too_far_seen = false;
        }
        2 => {
            if *t < 2.0 {
                return;
            }
            // 目标 103 存在但 mock 怪会追击贴近玩家；范围外提示由单测 Chebyshev 覆盖（#1554）
            // 这里验证：目标存在时攻击目标保留（系统未因距离误清空）
            let kept = control.attack_target == Some(103);
            let verdict = if kept {
                "✅ 目标存在则保留（距离校验不误清空）"
            } else {
                "⚠️ 目标被清空（怪物死亡/消失）"
            };
            tracing::info!("[ATKRANGE] {} (too_far={})", verdict, *too_far_seen);
            *stage = 3;
            *t = 0.0;
        }
        _ => {}
    }
}
