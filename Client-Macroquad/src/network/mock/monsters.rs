use super::*;

impl MockNetwork {
    pub(super) fn tick_monster_combat(response_tx: &Sender<NetworkEvent>, state: &mut MockWorldState) {
        // 这些 sound_id 来自 Sound/SoundList.lst（映射到实际 wav 文件）。
        const SFX_PLAYER_DIED: i32 = 10100; // 100.wav

        // 无敌期间：怪物不追击不攻击，避免复活/进游戏后“无法操作”。
        if let Some(until) = state.player_protected_until {
            if Instant::now() < until {
                return;
            } else {
                state.player_protected_until = None;
            }
        }

        if state.last_monster_combat_tick.elapsed() < Duration::from_millis(180) {
            return;
        }
        state.last_monster_combat_tick = Instant::now();

        if state.player_hp_current <= 0 {
            return;
        }

        let player_id = state.player_object_id;
        let (px, py) = state.player_grid;

        // 每次最多处理少量怪，避免事件太多
        let mut acted = 0usize;
        let limit = 8usize;

        let mut candidates: Vec<(u32, i32)> = state
            .monsters
            .iter()
            .filter_map(|(mid, m)| {
                if m.hp <= 0 {
                    return None;
                }
                let dist = (m.pos.0 - px).abs() + (m.pos.1 - py).abs();
                Some((*mid, dist))
            })
            .collect();
        candidates.sort_by_key(|(_, d)| *d);

        // 简单参数：不追求完全还原，只求闭环可玩
        // 普通怪用常量；Boss 用 config.ini 的 boss_* 参数。
        let normal_aggro_range = 8i32;
        let normal_attack_cooldown = Duration::from_millis(900);
        let chase_interval = Duration::from_millis(240);

        for (mid, dist) in candidates {
            if acted >= limit {
                break;
            }

            let Some(m) = state.monsters.get(&mid).copied() else {
                continue;
            };
            if m.hp <= 0 {
                continue;
            }

            let zone_is_boss = state.zones.get(m.zone_idx).map(|z| z.is_boss).unwrap_or(false);
            if zone_is_boss && !state.mock_cfg.boss_enabled {
                continue;
            }

            let aggro_range = if zone_is_boss {
                state.mock_cfg.boss_aggro_range.max(1)
            } else {
                normal_aggro_range
            };
            if dist > aggro_range {
                continue;
            }

            let (mx, my) = m.pos;
            let dx = (px - mx).signum();
            let dy = (py - my).signum();
            let dir = Self::dir_from_delta(dx, dy);

            let attack_cooldown = if zone_is_boss {
                Duration::from_millis(state.mock_cfg.boss_attack_cooldown_ms.max(50))
            } else {
                normal_attack_cooldown
            };

            // 近战：相邻就打
            if dist <= 1 {
                if m.last_attack.elapsed() < attack_cooldown {
                    continue;
                }

                // 更新 last_attack
                if let Some(mm) = state.monsters.get_mut(&mid) {
                    mm.last_attack = Instant::now();
                }

                let _ = response_tx.send(NetworkEvent::ObjectTurn {
                    packet: mir2_shared::packets::server::ObjectTurn {
                        object_id: mid,
                        location_x: mx,
                        location_y: my,
                        direction: dir,
                    },
                });
                let _ = response_tx.send(NetworkEvent::ObjectAttack {
                    object_id: mid,
                    location_x: (mx.max(0) as u32),
                    location_y: (my.max(0) as u32),
                    direction: dir as u8,
                    spell: 0,
                    level: 0,
                    attack_type: 0,
                });

                let damage = if zone_is_boss {
                    let mut lo = state.mock_cfg.boss_damage_min;
                    let mut hi = state.mock_cfg.boss_damage_max;
                    if lo > hi {
                        std::mem::swap(&mut lo, &mut hi);
                    }
                    let span = (hi - lo).max(0) as u32;
                    lo + (Self::rng_next_u32(&mut state.rng) % (span + 1)) as i32
                } else {
                    6 + (Self::rng_next_u32(&mut state.rng) % 7) as i32
                };
                state.player_hp_current = (state.player_hp_current - damage).max(0);

                // 用 ObjectStruck/ObjectDied 走统一落地（NetworkApplySystem 会给玩家播放受击/死亡音效 + 飘字）
                let _ = response_tx.send(NetworkEvent::ObjectStruck {
                    object_id: player_id,
                    attacker_id: mid,
                    damage,
                    location_x: 0,
                    location_y: 0,
                    direction: 0,
                });
                let _ = response_tx.send(NetworkEvent::HealthChanged {
                    current: state.player_hp_current.max(0) as u32,
                    max: state.player_hp_max.max(1) as u32,
                });

                if state.player_hp_current <= 0 {
                    // 同一 tick 可能多只怪同时攻击；用 player_dead_since 去重，避免重复死亡事件/音效。
                    if state.player_dead_since.is_none() {
                        let _ = response_tx.send(NetworkEvent::ObjectDied { object_id: player_id, location_x: 0, location_y: 0, direction: 0, death_type: 0 });
                        let _ = response_tx.send(NetworkEvent::PlaySound {
                            sound_id: SFX_PLAYER_DIED,
                        });
                        let _ = response_tx.send(NetworkEvent::SystemMessage {
                            message: "(MOCK) 你已死亡，5 秒后回城复活".to_string(),
                        });

                        // 标记死亡开始时间，用于 respawn
                        state.player_dead_since = Some(Instant::now());
                    }
                }

                acted += 1;
                continue;
            }

            // 追击：朝玩家走/跑一步（更接近原版：远一点跑，贴近走）
            if m.last_chase_step.elapsed() < chase_interval {
                continue;
            }

            let mut nx = mx + dx;
            let mut ny = my + dy;

            // 避免踩到玩家格子：对角靠近时优先拆成直线
            if (nx, ny) == (px, py) {
                if dx != 0 && dy != 0 {
                    if (mx + dx, my) != (px, py) {
                        nx = mx + dx;
                        ny = my;
                    } else if (mx, my + dy) != (px, py) {
                        nx = mx;
                        ny = my + dy;
                    } else {
                        continue;
                    }
                } else {
                    continue;
                }
            }

            // 简单限制：不要无限跑出出生区太远
            if let Some(z) = state.zones.get(m.zone_idx) {
                let dist_from_center = (nx - z.center.0).abs() + (ny - z.center.1).abs();
                if dist_from_center > z.radius * 6 {
                    continue;
                }
            }

            // 避障：目标格不可走则尝试拆分/侧移（避免追击跑进障碍物里）
            if !Self::map_is_walkable(state, nx, ny) {
                let mut found: Option<(i32, i32)> = None;
                let candidates: [(i32, i32); 6] = [
                    (mx + dx, my),
                    (mx, my + dy),
                    (mx + dx, my + 1),
                    (mx + dx, my - 1),
                    (mx + 1, my + dy),
                    (mx - 1, my + dy),
                ];
                for (tx, ty) in candidates {
                    if (tx, ty) == (px, py) {
                        continue;
                    }
                    if Self::map_is_walkable(state, tx, ty) {
                        found = Some((tx, ty));
                        break;
                    }
                }
                let Some((fx, fy)) = found else {
                    continue;
                };
                nx = fx;
                ny = fy;
            }

            if let Some(mm) = state.monsters.get_mut(&mid) {
                mm.pos = (nx, ny);
                mm.last_chase_step = Instant::now();
            }

            let prefer_run = dist >= 3;
            if prefer_run {
                let _ = response_tx.send(NetworkEvent::ObjectRun {
                    packet: mir2_shared::packets::server::ObjectRun {
                        object_id: mid,
                        location_x: nx,
                        location_y: ny,
                        direction: dir,
                    },
                });
            } else {
                let _ = response_tx.send(NetworkEvent::ObjectWalk {
                    packet: mir2_shared::packets::server::ObjectWalk {
                        object_id: mid,
                        location_x: nx,
                        location_y: ny,
                        direction: dir,
                    },
                });
            }

            acted += 1;
        }
    }

    pub(super) fn spawn_monster_in_zone(
        response_tx: &Sender<NetworkEvent>,
        state: &mut MockWorldState,
        zone_idx: usize,
    ) {
        let Some(zone) = state.zones.get(zone_idx).cloned() else {
            return;
        };

        // MassBattle 模式下只允许 Boss（避免小怪刷屏，保持主目标明确）
        if state.mock_cfg.mass_battle_enabled && !zone.is_boss {
            return;
        }

        // Boss 总开关：如果运行时关掉了 boss，也不要继续生成
        if zone.is_boss && !state.mock_cfg.boss_enabled {
            return;
        }

        // 选择一个区域内随机点作为出生点（必须是可走格，避免刷在障碍物里）
        let mut spawn: Option<(i32, i32)> = None;
        for _ in 0..24 {
            let (tx, ty) = Self::random_pos_in_zone(&mut state.rng, &zone);
            if (tx, ty) == state.player_grid {
                continue;
            }
            if Self::map_is_walkable(state, tx, ty) {
                spawn = Some((tx, ty));
                break;
            }
        }
        let mut final_spawn = spawn.unwrap_or(zone.center);
        if !Self::map_is_walkable(state, final_spawn.0, final_spawn.1) {
            let max_r: i32 = 12;
            'outer: for r in 1..=max_r {
                for dy in -r..=r {
                    for dx in -r..=r {
                        if dx.abs() != r && dy.abs() != r {
                            continue;
                        }
                        let tx = zone.center.0 + dx;
                        let ty = zone.center.1 + dy;
                        if (tx, ty) == state.player_grid {
                            continue;
                        }
                        if Self::map_is_walkable(state, tx, ty) {
                            final_spawn = (tx, ty);
                            break 'outer;
                        }
                    }
                }
            }
        }
        let (x, y) = final_spawn;
        let object_id = state.next_monster_id;
        state.next_monster_id = state.next_monster_id.saturating_add(1);

        let dir = mir2_shared::enums::MirDirection::Down;
        let _ = response_tx.send(NetworkEvent::ObjectMonster {
            packet: mir2_shared::packets::server::ObjectMonster {
                object_id,
                name: format!("{}-Mob{}", zone.name, object_id),
                name_colour: 0,
                location_x: x,
                location_y: y,
                image: zone.monster_image,
                direction: dir,
                effect: 0,
                ai: 0,
                light: 0,
                dead: false,
                skeleton: false,
                poison: mir2_shared::enums::PoisonType::empty(),
                hidden: false,
                shock_time: 0,
                binding_shot_center: false,
                extra: false,
                extra_byte: 0,
                buffs: Vec::new(),
            },
        });

        state.monsters.insert(
            object_id,
            MockMonsterState {
                pos: (x, y),
                hp: zone.monster_hp,
                zone_idx,
                xp_reward: zone.xp_reward,
                last_chase_step: Instant::now() - Duration::from_millis(500),
                last_attack: Instant::now() - Duration::from_millis(1000),
            },
        );

        if zone.is_boss {
            let _ = response_tx.send(NetworkEvent::SystemMessage {
                message: format!("(MOCK) Boss spawned at ({}, {})", x, y),
            });
        }
    }

    pub(super) fn tick_zone_spawns(response_tx: &Sender<NetworkEvent>, state: &mut MockWorldState) {
        // 每个区域按 respawn_interval 补怪
        for zone_idx in 0..state.zones.len() {
            let Some(zone) = state.zones.get_mut(zone_idx) else {
                continue;
            };

            // MassBattle 模式下只补 Boss
            if state.mock_cfg.mass_battle_enabled && !zone.is_boss {
                continue;
            }

            // Boss 总开关
            if zone.is_boss && !state.mock_cfg.boss_enabled {
                continue;
            }

            if zone.last_spawn.elapsed() < zone.respawn_interval {
                continue;
            }

            let alive_in_zone = state
                .monsters
                .values()
                .filter(|m| m.hp > 0 && m.zone_idx == zone_idx)
                .count();
            if alive_in_zone >= zone.max_monsters {
                continue;
            }

            zone.last_spawn = Instant::now();
            Self::spawn_monster_in_zone(response_tx, state, zone_idx);
        }
    }

    pub(super) fn tick_monster_wander(response_tx: &Sender<NetworkEvent>, state: &mut MockWorldState) {
        if state.last_monster_wander_tick.elapsed() < Duration::from_millis(900) {
            return;
        }
        state.last_monster_wander_tick = Instant::now();

        // 每次最多移动少量怪，避免事件太多
        let mut moved = 0usize;
        let limit = 5usize;

        // 为了稳定遍历，先收集 id
        let monster_ids: Vec<u32> = state.monsters.keys().copied().collect();
        for mid in monster_ids {
            if moved >= limit {
                break;
            }
            let Some(m) = state.monsters.get(&mid).copied() else {
                continue;
            };
            if m.hp <= 0 {
                continue;
            }

            // 离玩家很近时不随机游荡，让追击逻辑接管
            let dist_to_player = (m.pos.0 - state.player_grid.0).abs() + (m.pos.1 - state.player_grid.1).abs();
            if dist_to_player <= 8 {
                continue;
            }
            let (zone_center, zone_radius) = match state.zones.get(m.zone_idx) {
                Some(z) => (z.center, z.radius),
                None => continue,
            };

            // 25% 概率动一下
            if !Self::rng_next_u32(&mut state.rng).is_multiple_of(4) {
                continue;
            }

            let (x, y) = m.pos;
            let (dx, dy, dir) = match Self::rng_next_u32(&mut state.rng) % 8 {
                0 => (0, -1, mir2_shared::enums::MirDirection::Up),
                1 => (1, -1, mir2_shared::enums::MirDirection::UpRight),
                2 => (1, 0, mir2_shared::enums::MirDirection::Right),
                3 => (1, 1, mir2_shared::enums::MirDirection::DownRight),
                4 => (0, 1, mir2_shared::enums::MirDirection::Down),
                5 => (-1, 1, mir2_shared::enums::MirDirection::DownLeft),
                6 => (-1, 0, mir2_shared::enums::MirDirection::Left),
                _ => (-1, -1, mir2_shared::enums::MirDirection::UpLeft),
            };
            let nx = x + dx;
            let ny = y + dy;

            // 限制在区域半径内
            let dist_from_center = (nx - zone_center.0).abs() + (ny - zone_center.1).abs();
            if dist_from_center > zone_radius * 2 {
                continue;
            }

            // 避障：怪物不能走进障碍物
            if !Self::map_is_walkable(state, nx, ny) {
                continue;
            }

            if let Some(mm) = state.monsters.get_mut(&mid) {
                mm.pos = (nx, ny);
            }

            let _ = response_tx.send(NetworkEvent::ObjectWalk {
                packet: mir2_shared::packets::server::ObjectWalk {
                    object_id: mid,
                    location_x: nx,
                    location_y: ny,
                    direction: dir,
                },
            });
            moved += 1;
        }
    }
}
