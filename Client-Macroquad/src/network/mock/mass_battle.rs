use super::*;

use std::collections::{HashMap, HashSet};

impl MockNetwork {
    pub(super) fn tick_mass_battle(response_tx: &Sender<NetworkEvent>, state: &mut MockWorldState) {
        fn get_two_mut<T>(slice: &mut [T], a: usize, b: usize) -> (&mut T, &mut T) {
            debug_assert!(a != b);
            if a < b {
                let (left, right) = slice.split_at_mut(b);
                (&mut left[a], &mut right[0])
            } else {
                let (left, right) = slice.split_at_mut(a);
                (&mut right[0], &mut left[b])
            }
        }

        let cfg = state.mock_cfg.clone();
        if !cfg.mass_battle_enabled {
            return;
        }

        let now = Instant::now();
        let tick_every = Duration::from_millis(cfg.ai_tick_ms.max(10) as u64);
        let attack_every = Duration::from_millis(cfg.attack_cooldown_ms.max(50) as u64);
        let respawn_after = Duration::from_millis(cfg.mass_battle_respawn_ms.max(1) as u64);

        let (map_w_eff, map_h_eff) = Self::effective_map_dims(state);
        if map_w_eff <= 0 || map_h_eff <= 0 {
            return;
        }

        // Two opposing centers: prefer left vs right, but keep them connected (same walkable component).
        let center_y = (map_h_eff / 2).clamp(0, map_h_eff.saturating_sub(1).max(0));
        let prefer_a = (
            (map_w_eff / 3).clamp(0, map_w_eff.saturating_sub(1).max(0)),
            center_y,
        );
        let prefer_b = (
            ((map_w_eff * 2) / 3).clamp(0, map_w_eff.saturating_sub(1).max(0)),
            center_y,
        );

        let (center_a, center_b) =
            if let (Some(a), Some(b)) = (state.mass_battle_center_a, state.mass_battle_center_b) {
                (a, b)
            } else {
                let sample_radius = (cfg.mass_battle_spawn_radius * 2).max(30);
                let (a, b) = Self::pick_connected_mass_battle_centers_raw(
                    state.map_width,
                    state.map_height,
                    &state.map_walkable,
                    map_w_eff,
                    map_h_eff,
                    &mut state.rng,
                    prefer_a,
                    prefer_b,
                    sample_radius,
                )
                .unwrap_or((prefer_a, prefer_b));
                state.mass_battle_center_a = Some(a);
                state.mass_battle_center_b = Some(b);
                (a, b)
            };

        // Occupancy for basic collision avoidance (grid-based).
        let mut occupied: HashSet<(i32, i32)> =
            HashSet::with_capacity(state.remote_players.len().saturating_add(8));
        occupied.insert(state.player_grid);
        for rp in &state.remote_players {
            if rp.dead_until.is_none() && rp.hp_current > 0 {
                occupied.insert(rp.grid);
            }
        }

        // Respawn anyone whose timer is up.
        for rp in &mut state.remote_players {
            if let Some(dead_until) = rp.dead_until {
                if now >= dead_until {
                    let spawn_center = if rp.team == 0 { center_a } else { center_b };
                    if let Some((sx, sy)) = Self::pick_random_walkable_near_center_raw(
                        state.map_width,
                        state.map_height,
                        &state.map_walkable,
                        map_w_eff,
                        map_h_eff,
                        &mut state.rng,
                        &occupied,
                        spawn_center,
                        cfg.mass_battle_spawn_radius,
                        200,
                    ) {
                        occupied.remove(&rp.grid);
                        rp.grid = (sx, sy);
                        occupied.insert(rp.grid);
                    }

                    rp.hp_current = rp.hp_max.max(1);
                    rp.dead_until = None;
                    rp.last_tick = now;
                    rp.last_attack = now - attack_every;
                    Self::send_object_player_update(response_tx, rp);
                }
            }
        }

        // Build a simple bucket grid index per team to avoid O(N^2) scans.
        let bucket = cfg.mass_battle_bucket.max(1);
        let mut buckets_team0: HashMap<(i32, i32), Vec<usize>> = HashMap::new();
        let mut buckets_team1: HashMap<(i32, i32), Vec<usize>> = HashMap::new();

        for (idx, rp) in state.remote_players.iter().enumerate() {
            if rp.dead_until.is_some() || rp.hp_current <= 0 {
                continue;
            }
            let key = (rp.grid.0.div_euclid(bucket), rp.grid.1.div_euclid(bucket));
            if rp.team == 0 {
                buckets_team0.entry(key).or_default().push(idx);
            } else {
                buckets_team1.entry(key).or_default().push(idx);
            }
        }

        let total = state.remote_players.len();
        if total == 0 {
            return;
        }

        let engage_range = cfg.mass_battle_engage_range.max(1);
        let bucket_radius = ((engage_range + bucket - 1) / bucket).max(1);
        let mut processed: usize = 0;
        let start = (Self::rng_next_u32(&mut state.rng) as usize) % total;

        for i in 0..total {
            if processed >= cfg.mass_battle_attackers_per_tick.max(1) {
                break;
            }
            let attacker_idx = (start + i) % total;

            // Phase 1: snapshot attacker state without borrowing mutably.
            let (ax, ay, team, can_tick) = {
                let a = &state.remote_players[attacker_idx];
                if a.dead_until.is_some() || a.hp_current <= 0 {
                    (0, 0, 0, false)
                } else {
                    let can_tick = now.duration_since(a.last_tick) >= tick_every;
                    (a.grid.0, a.grid.1, a.team, can_tick)
                }
            };
            if !can_tick {
                continue;
            }

            // Phase 2: find a nearby enemy (nearest) using bucket neighbors.
            let attacker_bucket = (ax.div_euclid(bucket), ay.div_euclid(bucket));
            let enemy_buckets = if team == 0 {
                &buckets_team1
            } else {
                &buckets_team0
            };

            let mut best_target: Option<usize> = None;
            let mut best_d2: i32 = i32::MAX;
            for by in (attacker_bucket.1 - bucket_radius)..=(attacker_bucket.1 + bucket_radius) {
                for bx in (attacker_bucket.0 - bucket_radius)..=(attacker_bucket.0 + bucket_radius)
                {
                    if let Some(candidates) = enemy_buckets.get(&(bx, by)) {
                        for &cand_idx in candidates {
                            let c = &state.remote_players[cand_idx];
                            if c.dead_until.is_some() || c.hp_current <= 0 {
                                continue;
                            }
                            let dx = c.grid.0 - ax;
                            let dy = c.grid.1 - ay;
                            if dx.abs() + dy.abs() > engage_range {
                                continue;
                            }
                            let d2 = dx * dx + dy * dy;
                            if d2 < best_d2 {
                                best_d2 = d2;
                                best_target = Some(cand_idx);
                            }
                        }
                    }
                }
            }

            // If no enemy found nearby, drift toward the opposing center.
            let (tx, ty, target_idx_opt) = if let Some(tidx) = best_target {
                let t = &state.remote_players[tidx];
                (t.grid.0, t.grid.1, Some(tidx))
            } else {
                let c = if team == 0 { center_b } else { center_a };

                // 目标点轻微抖动，避免所有人精确挤到同一格/同一墙角。
                let jitter_r = (cfg.mass_battle_spawn_radius / 2).clamp(2, 8);
                let ox =
                    (Self::rng_next_u32(&mut state.rng) as i32 % (jitter_r * 2 + 1)) - jitter_r;
                let oy =
                    (Self::rng_next_u32(&mut state.rng) as i32 % (jitter_r * 2 + 1)) - jitter_r;
                let jx = (c.0 + ox).clamp(0, map_w_eff.saturating_sub(1).max(0));
                let jy = (c.1 + oy).clamp(0, map_h_eff.saturating_sub(1).max(0));
                if Self::map_is_walkable_raw(state.map_width, state.map_height, &state.map_walkable, jx, jy) {
                    (jx, jy, None)
                } else {
                    (c.0, c.1, None)
                }
            };

            // Phase 3: act (move or attack) with mutable borrows.
            {
                let a = &mut state.remote_players[attacker_idx];
                a.last_tick = now;

                let dx = (tx - a.grid.0).signum();
                let dy = (ty - a.grid.1).signum();
                let manhattan = (tx - a.grid.0).abs() + (ty - a.grid.1).abs();

                // Face target/goal.
                if dx != 0 || dy != 0 {
                    a.direction = Self::dir_from_delta(dx, dy);
                }

                // Attack when adjacent and a real target exists.
                if let Some(target_idx) = target_idx_opt {
                    if manhattan <= 1 && now.duration_since(a.last_attack) >= attack_every {
                        if attacker_idx != target_idx {
                            let (attacker, target) =
                                get_two_mut(&mut state.remote_players, attacker_idx, target_idx);

                            if target.dead_until.is_none() && target.hp_current > 0 {
                                attacker.last_attack = now;

                                let _ = response_tx.send(NetworkEvent::ObjectTurn {
                                    packet: mir2_shared::packets::server::ObjectTurn {
                                        object_id: attacker.id,
                                        location_x: attacker.grid.0,
                                        location_y: attacker.grid.1,
                                        direction: attacker.direction,
                                    },
                                });

                                let _ = response_tx.send(NetworkEvent::ObjectAttack {
                                    packet: mir2_shared::packets::server::ObjectAttack {
                                        object_id: attacker.id,
                                        location_x: attacker.grid.0 as u32,
                                        location_y: attacker.grid.1 as u32,
                                        direction: attacker.direction as u8,
                                        spell: 0,
                                        level: 0,
                                        attack_type: 0,
                                    },
                                });

                                let base = 4 + (attacker.level as i32 / 2);
                                let variance = (Self::rng_next_u32(&mut state.rng) % 5) as i32; // 0..4
                                let dmg = (base + variance).max(1);
                                target.hp_current -= dmg;

                                let _ = response_tx.send(NetworkEvent::ObjectStruck {
                                    object_id: target.id,
                                    attacker_id: attacker.id,
                                    damage: dmg,
                                });

                                if target.hp_current <= 0 {
                                    target.hp_current = 0;
                                    target.dead_until = Some(now + respawn_after);

                                    occupied.remove(&target.grid);

                                    let _ = response_tx.send(NetworkEvent::ObjectDied {
                                        object_id: target.id,
                                    });
                                    let _ = response_tx.send(NetworkEvent::ObjectRemove {
                                        packet: mir2_shared::packets::server::ObjectRemove {
                                            object_id: target.id,
                                        },
                                    });
                                }
                            }
                        }

                        processed += 1;
                        continue;
                    }
                }

                // Move toward tx/ty with basic avoidance.
                // Run 包在客户端通常会被当作“可能跨 2 格”的移动，因此：
                // - 只有在我们实际移动 2 格时才发 ObjectRun
                // - 否则一律发 ObjectWalk（避免“原地跑/跑步滑步感”）
                let want_run = manhattan > 2;

                let step1_candidates = [
                    (a.grid.0 + dx, a.grid.1 + dy),
                    (a.grid.0 + dx, a.grid.1),
                    (a.grid.0, a.grid.1 + dy),
                ];

                let mut step1: Option<(i32, i32)> = None;
                for (nx, ny) in step1_candidates {
                    if nx < 0 || ny < 0 || nx >= map_w_eff || ny >= map_h_eff {
                        continue;
                    }
                    if !Self::map_is_walkable_raw(state.map_width, state.map_height, &state.map_walkable, nx, ny) {
                        continue;
                    }
                    if occupied.contains(&(nx, ny)) {
                        continue;
                    }
                    step1 = Some((nx, ny));
                    break;
                }

                // 如果直线/对角线都走不了（卡墙/拥挤），允许“侧移一步”绕开。
                // 这不是完整寻路，但能显著减少大规模群战里大量单位原地转圈。
                if step1.is_none() {
                    let mut best: Option<((i32, i32), i32)> = None;
                    let neighbors = [
                        (-1, -1),
                        (0, -1),
                        (1, -1),
                        (-1, 0),
                        (1, 0),
                        (-1, 1),
                        (0, 1),
                        (1, 1),
                    ];

                    for (ox, oy) in neighbors {
                        let nx = a.grid.0 + ox;
                        let ny = a.grid.1 + oy;
                        if nx < 0 || ny < 0 || nx >= map_w_eff || ny >= map_h_eff {
                            continue;
                        }
                        if !Self::map_is_walkable_raw(state.map_width, state.map_height, &state.map_walkable, nx, ny) {
                            continue;
                        }
                        if occupied.contains(&(nx, ny)) {
                            continue;
                        }
                        let dist = (nx - tx).abs() + (ny - ty).abs();
                        if best.map(|(_, d)| dist < d).unwrap_or(true) {
                            best = Some(((nx, ny), dist));
                        }
                    }

                    step1 = best.map(|(p, _)| p);
                }

                if let Some((nx1, ny1)) = step1 {
                    let mut moved_to = (nx1, ny1);
                    let mut did_run = false;

                    if want_run {
                        let dx2 = (nx1 - a.grid.0).signum();
                        let dy2 = (ny1 - a.grid.1).signum();
                        let nx2 = nx1 + dx2;
                        let ny2 = ny1 + dy2;
                        if !(nx2 == nx1 && ny2 == ny1)
                            && nx2 >= 0
                            && ny2 >= 0
                            && nx2 < map_w_eff
                            && ny2 < map_h_eff
                            && Self::map_is_walkable_raw(
                                state.map_width,
                                state.map_height,
                                &state.map_walkable,
                                nx2,
                                ny2,
                            )
                            && !occupied.contains(&(nx2, ny2))
                        {
                            moved_to = (nx2, ny2);
                            did_run = true;
                        }
                    }

                    occupied.remove(&a.grid);
                    let mdx = (moved_to.0 - a.grid.0).signum();
                    let mdy = (moved_to.1 - a.grid.1).signum();
                    if mdx != 0 || mdy != 0 {
                        a.direction = Self::dir_from_delta(mdx, mdy);
                    }
                    a.grid = moved_to;
                    occupied.insert(a.grid);

                    if did_run {
                        let _ = response_tx.send(NetworkEvent::ObjectRun {
                            packet: mir2_shared::packets::server::ObjectRun {
                                object_id: a.id,
                                location_x: a.grid.0,
                                location_y: a.grid.1,
                                direction: a.direction,
                            },
                        });
                    } else {
                        let _ = response_tx.send(NetworkEvent::ObjectWalk {
                            packet: mir2_shared::packets::server::ObjectWalk {
                                object_id: a.id,
                                location_x: a.grid.0,
                                location_y: a.grid.1,
                                direction: a.direction,
                            },
                        });
                    }
                }

                // 如果被墙/人堵住：尝试随机侧移一步（避免长期卡在障碍物边缘原地抽搐）
                if step1.is_none() {
                    let dirs: [(i32, i32); 8] = [
                        (0, -1),
                        (1, -1),
                        (1, 0),
                        (1, 1),
                        (0, 1),
                        (-1, 1),
                        (-1, 0),
                        (-1, -1),
                    ];
                    let start = (Self::rng_next_u32(&mut state.rng) % 8) as usize;
                    let mut moved: Option<(i32, i32)> = None;
                    for i in 0..8 {
                        let (dx, dy) = dirs[(start + i) % 8];
                        let nx = a.grid.0 + dx;
                        let ny = a.grid.1 + dy;
                        if nx < 0 || ny < 0 || nx >= map_w_eff || ny >= map_h_eff {
                            continue;
                        }
                        if !Self::map_is_walkable_raw(state.map_width, state.map_height, &state.map_walkable, nx, ny) {
                            continue;
                        }
                        if occupied.contains(&(nx, ny)) {
                            continue;
                        }
                        moved = Some((nx, ny));
                        break;
                    }
                    if let Some((nx, ny)) = moved {
                        occupied.remove(&a.grid);
                        a.direction = Self::dir_from_delta((nx - a.grid.0).signum(), (ny - a.grid.1).signum());
                        a.grid = (nx, ny);
                        occupied.insert(a.grid);
                        let _ = response_tx.send(NetworkEvent::ObjectWalk {
                            packet: mir2_shared::packets::server::ObjectWalk {
                                object_id: a.id,
                                location_x: a.grid.0,
                                location_y: a.grid.1,
                                direction: a.direction,
                            },
                        });
                    }
                }

                processed += 1;
            }
        }
    }
}
