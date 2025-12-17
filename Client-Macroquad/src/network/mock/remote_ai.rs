use super::*;

impl MockNetwork {
    pub(super) fn tick_remote_players_ai(response_tx: &Sender<NetworkEvent>, state: &mut MockWorldState) {
        static REMOTE_AI_DIAG_ONCE: std::sync::OnceLock<()> = std::sync::OnceLock::new();
        let _ = REMOTE_AI_DIAG_ONCE.set(()).map(|_| {
            println!("[MOCK][AI] Multi-remote AI enabled (count={})", state.remote_players.len());
        });

        // 行为树风格（方案 A）：用 stateless BT 组织决策，而不是显式“状态机大 switch”。
        // 仍然复用 rp.mode 作为可视化/权重提示，不作为唯一驱动。

        #[derive(Clone, Copy, PartialEq, Eq)]
        enum BtStatus {
            Success,
            Failure,
            Running,
        }

        fn selector(ctx: &mut RemoteBtCtx, nodes: &[fn(&mut RemoteBtCtx) -> BtStatus]) -> BtStatus {
            for n in nodes {
                match n(ctx) {
                    BtStatus::Success => return BtStatus::Success,
                    BtStatus::Running => return BtStatus::Running,
                    BtStatus::Failure => {}
                }
            }
            BtStatus::Failure
        }

        fn sequence(ctx: &mut RemoteBtCtx, nodes: &[fn(&mut RemoteBtCtx) -> BtStatus]) -> BtStatus {
            for n in nodes {
                match n(ctx) {
                    BtStatus::Success => {}
                    BtStatus::Running => return BtStatus::Running,
                    BtStatus::Failure => return BtStatus::Failure,
                }
            }
            BtStatus::Success
        }

        struct RemoteBtCtx<'a> {
            response_tx: &'a Sender<NetworkEvent>,
            zones: &'a [MockZone],
            monsters: &'a mut HashMap<u32, MockMonsterState>,
            occupied: &'a mut HashSet<(i32, i32)>,
            rng: &'a mut u64,
            now: Instant,
            rp: &'a mut MockRemotePlayerState,
            alive_in_zone: usize,
            cfg: &'a MockRuntimeConfig,

            // 地图碰撞（只读）：用于严格避障
            map_width: i32,
            map_height: i32,
            map_walkable: &'a [u8],
            map_w_eff: i32,
            map_h_eff: i32,
        }

        impl<'a> RemoteBtCtx<'a> {
            fn is_occupied(&self, tile: (i32, i32)) -> bool {
                self.occupied.contains(&tile)
            }

            fn is_walkable(&self, x: i32, y: i32) -> bool {
                if x < 0 || y < 0 {
                    return false;
                }
                if self.map_width > 0 && self.map_height > 0 {
                    if x >= self.map_width || y >= self.map_height {
                        return false;
                    }
                }
                if self.map_walkable.is_empty() || self.map_width <= 0 || self.map_height <= 0 {
                    // 未缓存碰撞：退化为“全部可走”
                    return true;
                }
                let idx = (y as usize).saturating_mul(self.map_width as usize) + (x as usize);
                self.map_walkable.get(idx).copied().unwrap_or(1) != 0
            }

            fn nearest_walkable_around(&self, x: i32, y: i32, max_r: i32) -> Option<(i32, i32)> {
                if self.is_walkable(x, y) {
                    return Some((x, y));
                }
                for r in 1..=max_r.max(1) {
                    for dy in -r..=r {
                        for dx in -r..=r {
                            if dx.abs() != r && dy.abs() != r {
                                continue;
                            }
                            let tx = x + dx;
                            let ty = y + dy;
                            if self.is_walkable(tx, ty) {
                                return Some((tx, ty));
                            }
                        }
                    }
                }
                None
            }

            fn pick_target_in_zone(&mut self, perception: i32) {
                if self.rp.target_monster_id.is_some() {
                    return;
                }

                let (rx, ry) = self.rp.grid;
                let mut best: Option<(u32, i32)> = None;
                for (mid, m) in self.monsters.iter() {
                    if m.hp <= 0 || m.zone_idx != self.rp.zone_idx {
                        continue;
                    }
                    let dist = (m.pos.0 - rx).abs() + (m.pos.1 - ry).abs();
                    if dist > perception {
                        continue;
                    }
                    match best {
                        None => best = Some((*mid, dist)),
                        Some((_bid, bdist)) if dist < bdist => best = Some((*mid, dist)),
                        _ => {}
                    }
                }

                if let Some((mid, _)) = best {
                    self.rp.target_monster_id = Some(mid);
                    if self.rp.mode != RemoteAiMode::Chase {
                        self.rp.mode = RemoteAiMode::Chase;
                        self.rp.last_mode_change = self.now;
                    }
                }
            }

            fn eval_best_zone(&mut self) {
                if self
                    .rp
                    .last_zone_eval
                    .elapsed()
                    <= Duration::from_millis(self.cfg.zone_eval_ms)
                {
                    return;
                }
                self.rp.last_zone_eval = self.now;

                if self.zones.is_empty() {
                    return;
                }

                let (rx, ry) = self.rp.grid;
                let mut best_zone = self.rp.goal_zone_idx;
                let mut best_score: i32 = i32::MIN;
                for (idx, z) in self.zones.iter().enumerate() {
                    let alive = self
                        .monsters
                        .values()
                        .filter(|m| m.hp > 0 && m.zone_idx == idx)
                        .count() as i32;
                    let dist = (z.center.0 - rx).abs() + (z.center.1 - ry).abs();
                    let score = alive * 20 - dist;
                    if score > best_score {
                        best_score = score;
                        best_zone = idx;
                    }
                }
                self.rp.goal_zone_idx = best_zone;
            }

            fn step_towards(&mut self, tx: i32, ty: i32, prefer_run: bool) -> BtStatus {
                let (rx, ry) = self.rp.grid;
                let mut dx = (tx - rx).signum();
                let mut dy = (ty - ry).signum();
                if dx == 0 && dy == 0 {
                    return BtStatus::Success;
                }

                // 在计算新位置前，先从占位集合里移除自己当前格子，避免把自己当成障碍。
                let old_tile = (rx, ry);
                self.occupied.remove(&old_tile);

                // 让对角移动偶尔退化成直线，显得更“人”。
                if dx != 0 && dy != 0 && (MockNetwork::rng_next_u32(self.rng) % 3 == 0) {
                    if MockNetwork::rng_next_u32(self.rng) % 2 == 0 {
                        dx = 0;
                    } else {
                        dy = 0;
                    }
                }

                // 候选步进：优先向目标靠近；如果被占位/障碍挡住，尝试拆分/侧移。
                // 注意：每个候选都必须通过 walkable 校验，保证“不会跑进障碍物里”。
                let mut picked: Option<(i32, i32, i32, i32)> = None;
                let candidates: [(i32, i32, i32, i32); 7] = [
                    (rx + dx, ry + dy, dx, dy),
                    (rx + dx, ry, dx, 0),
                    (rx, ry + dy, 0, dy),
                    (rx + dx, ry + 1, dx, 1),
                    (rx + dx, ry - 1, dx, -1),
                    (rx + 1, ry + dy, 1, dy),
                    (rx - 1, ry + dy, -1, dy),
                ];

                for (cx, cy, cdx, cdy) in candidates {
                    let mut nx = cx;
                    let mut ny = cy;
                    nx = nx.clamp(0, self.map_w_eff.saturating_sub(1).max(0));
                    ny = ny.clamp(0, self.map_h_eff.saturating_sub(1).max(0));

                    if (nx, ny) == (rx, ry) {
                        continue;
                    }
                    if self.is_occupied((nx, ny)) {
                        continue;
                    }
                    if !self.is_walkable(nx, ny) {
                        continue;
                    }
                    picked = Some((nx, ny, cdx, cdy));
                    break;
                }

                // 若首选方向被挡住：尝试“换方向绕一下”（8 邻域随机打散）。
                if picked.is_none() {
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
                    // 轻量 shuffle：用 rng 选一个起点偏移
                    let start = (MockNetwork::rng_next_u32(self.rng) % 8) as usize;
                    for i in 0..8 {
                        let (sdx, sdy) = dirs[(start + i) % 8];
                        let nx = (rx + sdx).clamp(0, self.map_w_eff.saturating_sub(1).max(0));
                        let ny = (ry + sdy).clamp(0, self.map_h_eff.saturating_sub(1).max(0));
                        if (nx, ny) == (rx, ry) {
                            continue;
                        }
                        if self.is_occupied((nx, ny)) {
                            continue;
                        }
                        if !self.is_walkable(nx, ny) {
                            continue;
                        }
                        picked = Some((nx, ny, sdx, sdy));
                        break;
                    }
                }

                let Some((nx, ny, _fdx, _fdy)) = picked else {
                    // 彻底动不了：恢复占位，并“只转向不移动”，避免客户端看到原地跑。
                    self.occupied.insert(old_tile);
                    let turn = match MockNetwork::rng_next_u32(self.rng) % 8 {
                        0 => mir2_shared::enums::MirDirection::Up,
                        1 => mir2_shared::enums::MirDirection::UpRight,
                        2 => mir2_shared::enums::MirDirection::Right,
                        3 => mir2_shared::enums::MirDirection::DownRight,
                        4 => mir2_shared::enums::MirDirection::Down,
                        5 => mir2_shared::enums::MirDirection::DownLeft,
                        6 => mir2_shared::enums::MirDirection::Left,
                        _ => mir2_shared::enums::MirDirection::UpLeft,
                    };
                    self.rp.direction = turn;
                    let _ = self.response_tx.send(NetworkEvent::ObjectTurn {
                        packet: mir2_shared::packets::server::ObjectTurn {
                            object_id: self.rp.id,
                            location_x: rx,
                            location_y: ry,
                            direction: turn,
                        },
                    });
                    return BtStatus::Running;
                };

                let fdx = (nx - rx).signum();
                let fdy = (ny - ry).signum();
                self.rp.direction = MockNetwork::dir_from_delta(fdx, fdy);

                // Run 包在客户端会按“可能跨 2 格”做表现，所以只有我们实际跨 2 格时才发 Run。
                let mut did_run = false;
                let mut final_pos = (nx, ny);
                if prefer_run {
                    let nx2 = (nx + fdx).clamp(0, self.map_w_eff.saturating_sub(1).max(0));
                    let ny2 = (ny + fdy).clamp(0, self.map_h_eff.saturating_sub(1).max(0));
                    if (nx2, ny2) != (nx, ny)
                        && (nx2, ny2) != (rx, ry)
                        && !self.is_occupied((nx2, ny2))
                        && self.is_walkable(nx2, ny2)
                    {
                        final_pos = (nx2, ny2);
                        did_run = true;
                    }
                }

                if final_pos == (rx, ry) {
                    // 不发送“假移动包”
                    self.occupied.insert(old_tile);
                    return BtStatus::Running;
                }

                self.rp.grid = final_pos;
                self.occupied.insert(final_pos);

                if did_run {
                    let _ = self.response_tx.send(NetworkEvent::ObjectRun {
                        packet: mir2_shared::packets::server::ObjectRun {
                            object_id: self.rp.id,
                            location_x: final_pos.0,
                            location_y: final_pos.1,
                            direction: self.rp.direction,
                        },
                    });
                } else {
                    let _ = self.response_tx.send(NetworkEvent::ObjectWalk {
                        packet: mir2_shared::packets::server::ObjectWalk {
                            object_id: self.rp.id,
                            location_x: final_pos.0,
                            location_y: final_pos.1,
                            direction: self.rp.direction,
                        },
                    });
                }

                BtStatus::Running
            }
        }

        // ===== BT Nodes =====
        fn act_refresh(ctx: &mut RemoteBtCtx) -> BtStatus {
            // 目标保持：如果已有目标且仍存活就继续，否则清空
            if let Some(tid) = ctx.rp.target_monster_id {
                if !ctx.monsters.get(&tid).map(|m| m.hp > 0).unwrap_or(false) {
                    ctx.rp.target_monster_id = None;
                }
            }

            // 统计当前区怪物数（供 travel 条件使用）
            ctx.alive_in_zone = ctx
                .monsters
                .values()
                .filter(|m| m.hp > 0 && m.zone_idx == ctx.rp.zone_idx)
                .count();

            // 周期性评估去哪个区更划算
            ctx.eval_best_zone();

            // 寻敌
            ctx.pick_target_in_zone(ctx.cfg.perception);
            BtStatus::Success
        }

        fn act_rest_gate(ctx: &mut RemoteBtCtx) -> BtStatus {
            // 正在休息：到点后恢复 Seek，并允许继续决策
            if ctx.rp.mode == RemoteAiMode::Rest {
                if ctx
                    .rp
                    .last_mode_change
                    .elapsed()
                    > Duration::from_millis(ctx.cfg.rest_ms)
                {
                    ctx.rp.mode = RemoteAiMode::Seek;
                    ctx.rp.last_mode_change = ctx.now;
                    return BtStatus::Failure;
                }
                return BtStatus::Running;
            }

            // 偶尔发呆（只在非战斗态）
            if matches!(ctx.rp.mode, RemoteAiMode::Roam | RemoteAiMode::Seek) {
                let roll = (MockNetwork::rng_next_u32(ctx.rng) % 10_000) as f32 / 10_000.0;
                if roll < ctx.cfg.rest_chance {
                    ctx.rp.mode = RemoteAiMode::Rest;
                    ctx.rp.last_mode_change = ctx.now;
                    return BtStatus::Running;
                }
            }

            BtStatus::Failure
        }

        fn cond_can_fight(ctx: &mut RemoteBtCtx) -> BtStatus {
            let Some(tid) = ctx.rp.target_monster_id else {
                return BtStatus::Failure;
            };
            let (rx, ry) = ctx.rp.grid;
            let Some(m) = ctx.monsters.get(&tid).copied() else {
                return BtStatus::Failure;
            };
            if m.zone_idx != ctx.rp.zone_idx {
                return BtStatus::Failure;
            }
            // 太远就丢目标
            let dist_far = (m.pos.0 - rx).abs() + (m.pos.1 - ry).abs();
            if dist_far > ctx.cfg.chase_drop {
                ctx.rp.target_monster_id = None;
                return BtStatus::Failure;
            }
            let dist = (m.pos.0 - rx).abs() + (m.pos.1 - ry).abs();
            if dist <= 1 {
                BtStatus::Success
            } else {
                BtStatus::Failure
            }
        }

        fn act_fight(ctx: &mut RemoteBtCtx) -> BtStatus {
            let Some(tid) = ctx.rp.target_monster_id else {
                return BtStatus::Failure;
            };
            let (rx, ry) = ctx.rp.grid;

            ctx.rp.mode = RemoteAiMode::Fight;

            if ctx
                .rp
                .last_attack
                .elapsed()
                < Duration::from_millis(ctx.cfg.attack_cooldown_ms)
            {
                return BtStatus::Running;
            }
            ctx.rp.last_attack = ctx.now;

            let _ = ctx.response_tx.send(NetworkEvent::ObjectTurn {
                packet: mir2_shared::packets::server::ObjectTurn {
                    object_id: ctx.rp.id,
                    location_x: rx,
                    location_y: ry,
                    direction: ctx.rp.direction,
                },
            });

            let _ = ctx.response_tx.send(NetworkEvent::ObjectAttack {
                packet: mir2_shared::packets::server::ObjectAttack {
                    object_id: ctx.rp.id,
                    location_x: rx as u32,
                    location_y: ry as u32,
                    direction: ctx.rp.direction as u8,
                    spell: 0,
                    level: 0,
                    attack_type: 0,
                },
            });

            // 命中/闪避
            let hit_roll = (MockNetwork::rng_next_u32(ctx.rng) % 100) as i32;
            let hit_chance = (75 + (ctx.rp.level as i32 * 2)).min(96);
            if hit_roll > hit_chance {
                ctx.rp.mode = RemoteAiMode::Chase;
                return BtStatus::Running;
            }

            let damage = 6 + ((ctx.rp.level as i32).min(20) / 2);
            if let Some(mm) = ctx.monsters.get_mut(&tid) {
                mm.hp -= damage;
            }

            let _ = ctx.response_tx.send(NetworkEvent::ObjectStruck {
                object_id: tid,
                attacker_id: ctx.rp.id,
                damage,
            });

            let dead = ctx.monsters.get(&tid).map(|mm| mm.hp <= 0).unwrap_or(false);
            if dead {
                let xp = ctx.monsters.get(&tid).map(|mm| mm.xp_reward).unwrap_or(10);

                let _ = ctx.response_tx.send(NetworkEvent::ObjectDied { object_id: tid });
                let _ = ctx.response_tx.send(NetworkEvent::ObjectRemove {
                    packet: mir2_shared::packets::server::ObjectRemove { object_id: tid },
                });
                ctx.monsters.remove(&tid);
                ctx.rp.target_monster_id = None;
                ctx.rp.mode = RemoteAiMode::Seek;

                ctx.rp.experience += xp;
                let mut leveled = false;
                while ctx.rp.experience >= ctx.rp.max_experience && ctx.rp.max_experience > 0 {
                    ctx.rp.experience -= ctx.rp.max_experience;
                    ctx.rp.level = ctx.rp.level.saturating_add(1);
                    ctx.rp.max_experience = MockNetwork::exp_for_next_level(ctx.rp.level);
                    leveled = true;
                }

                let _ = ctx.response_tx.send(NetworkEvent::SystemMessage {
                    message: format!("(MOCK) {} killed monster {} (+{} exp)", ctx.rp.name, tid, xp),
                });
                if leveled {
                    MockNetwork::send_object_player_update(ctx.response_tx, ctx.rp);
                    let _ = ctx.response_tx.send(NetworkEvent::SystemMessage {
                        message: format!("(MOCK) {} leveled up to {}!", ctx.rp.name, ctx.rp.level),
                    });
                }
            } else {
                ctx.rp.mode = RemoteAiMode::Chase;
            }

            BtStatus::Running
        }

        fn cond_has_target(ctx: &mut RemoteBtCtx) -> BtStatus {
            let Some(tid) = ctx.rp.target_monster_id else {
                return BtStatus::Failure;
            };
            let Some(m) = ctx.monsters.get(&tid).copied() else {
                ctx.rp.target_monster_id = None;
                return BtStatus::Failure;
            };
            if m.hp <= 0 {
                ctx.rp.target_monster_id = None;
                return BtStatus::Failure;
            }
            if m.zone_idx != ctx.rp.zone_idx {
                // 目标跨区：转 Travel
                ctx.rp.goal_zone_idx = m.zone_idx;
                ctx.rp.mode = RemoteAiMode::Travel;
                ctx.rp.last_mode_change = ctx.now;
                return BtStatus::Failure;
            }
            BtStatus::Success
        }

        fn act_chase(ctx: &mut RemoteBtCtx) -> BtStatus {
            let Some(tid) = ctx.rp.target_monster_id else {
                return BtStatus::Failure;
            };
            let Some(m) = ctx.monsters.get(&tid).copied() else {
                ctx.rp.target_monster_id = None;
                return BtStatus::Failure;
            };
            let (rx, ry) = ctx.rp.grid;
            let dist_far = (m.pos.0 - rx).abs() + (m.pos.1 - ry).abs();
            if dist_far > ctx.cfg.chase_drop {
                ctx.rp.target_monster_id = None;
                ctx.rp.mode = RemoteAiMode::Seek;
                return BtStatus::Failure;
            }
            ctx.rp.mode = RemoteAiMode::Chase;
            ctx.step_towards(m.pos.0, m.pos.1, true)
        }

        fn cond_should_travel(ctx: &mut RemoteBtCtx) -> BtStatus {
            if ctx.rp.target_monster_id.is_some() {
                return BtStatus::Failure;
            }
            if ctx.rp.goal_zone_idx == ctx.rp.zone_idx {
                return BtStatus::Failure;
            }
            // 当前区没怪：更倾向换区
            if ctx.alive_in_zone == 0 {
                return BtStatus::Success;
            }
            BtStatus::Failure
        }

        fn act_travel(ctx: &mut RemoteBtCtx) -> BtStatus {
            ctx.rp.mode = RemoteAiMode::Travel;
            let Some(z) = ctx.zones.get(ctx.rp.goal_zone_idx) else {
                ctx.rp.mode = RemoteAiMode::Seek;
                return BtStatus::Failure;
            };

            let target = ctx
                .nearest_walkable_around(z.center.0, z.center.1, 12)
                .unwrap_or(z.center);
            let (rx, ry) = ctx.rp.grid;
            let arrive_dist = (target.0 - rx).abs() + (target.1 - ry).abs();
            if arrive_dist <= 2 {
                ctx.rp.zone_idx = ctx.rp.goal_zone_idx;
                ctx.rp.mode = RemoteAiMode::Seek;
                ctx.rp.last_mode_change = ctx.now;
                return BtStatus::Success;
            }
            ctx.step_towards(target.0, target.1, true)
        }

        fn act_roam(ctx: &mut RemoteBtCtx) -> BtStatus {
            if ctx
                .rp
                .last_roam_pick
                .elapsed()
                > Duration::from_millis(ctx.cfg.roam_pick_ms)
            {
                ctx.rp.last_roam_pick = ctx.now;
                if let Some(z) = ctx.zones.get(ctx.rp.zone_idx) {
                    let mut picked: Option<(i32, i32)> = None;
                    for _ in 0..24 {
                        let (tx, ty) = MockNetwork::random_pos_in_zone(ctx.rng, z);
                        if ctx.is_walkable(tx, ty) {
                            picked = Some((tx, ty));
                            break;
                        }
                    }
                    ctx.rp.roam_goal = picked.unwrap_or(z.center);
                }
            }
            ctx.rp.mode = RemoteAiMode::Roam;
            let prefer_run = matches!(ctx.rp.mode, RemoteAiMode::Travel | RemoteAiMode::Chase)
                || (MockNetwork::rng_next_u32(ctx.rng) % 4 == 0);
            ctx.step_towards(ctx.rp.roam_goal.0, ctx.rp.roam_goal.1, prefer_run)
        }

        let now = Instant::now();
        let zones = &state.zones;
        let mut rng = state.rng;
        let cfg = state.mock_cfg.clone();

        let map_width = state.map_width;
        let map_height = state.map_height;
        let map_walkable: &[u8] = state.map_walkable.as_slice();
        let (map_w_eff, map_h_eff) = MockNetwork::effective_map_dims(state);

        // 预构建占位集合：避免 3000 人时每步 O(n) 扫描导致 O(n^2)
        let mut occupied_tiles: HashSet<(i32, i32)> = state.remote_players.iter().map(|p| p.grid).collect();
        occupied_tiles.insert(state.player_grid);
        for m in state.monsters.values() {
            if m.hp > 0 {
                occupied_tiles.insert(m.pos);
            }
        }

        let monsters = &mut state.monsters;

        let player_count = state.remote_players.len();
        for i in 0..player_count {
            let (_left, right) = state.remote_players.split_at_mut(i);
            let Some((rp, _rest)) = right.split_first_mut() else {
                break;
            };

            if rp
                .last_tick
                .elapsed()
                < Duration::from_millis(cfg.ai_tick_ms)
            {
                continue;
            }
            rp.last_tick = now;

            let mut ctx = RemoteBtCtx {
                response_tx,
                zones,
                monsters,
                occupied: &mut occupied_tiles,
                rng: &mut rng,
                now,
                rp,
                alive_in_zone: 0,
                cfg: &cfg,

                map_width,
                map_height,
                map_walkable,
                map_w_eff,
                map_h_eff,
            };

            let _ = sequence(
                &mut ctx,
                &[
                    act_refresh,
                    // Rest gate：如果触发/处于休息，则本帧结束
                    |c| match act_rest_gate(c) {
                        BtStatus::Failure => BtStatus::Success,
                        other => other,
                    },
                    // Engage
                    |c| {
                        selector(
                            c,
                            &[
                                // Fight: in melee
                                |cc| sequence(cc, &[cond_can_fight, act_fight]),
                                // Chase: has target
                                |cc| sequence(cc, &[cond_has_target, act_chase]),
                                // Travel: empty zone
                                |cc| sequence(cc, &[cond_should_travel, act_travel]),
                                // Roam
                                act_roam,
                            ],
                        )
                    },
                ],
            );
        }

        state.rng = rng;
    }
}
