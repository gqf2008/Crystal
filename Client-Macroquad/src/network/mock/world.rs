use super::*;

impl MockNetwork {
    pub(super) fn tick_world(response_tx: &Sender<NetworkEvent>, state: &mut MockWorldState) {
        if !state.in_game {
            return;
        }

        // 每分钟轮换一张地图（开发/压测用）
        Self::tick_map_rotation(response_tx, state);

        // 挂机看门狗：长期运行时，若玩家附近长期没怪（例如换图/刷怪中心漂移/越界），自动补一个 boss。
        Self::tick_afk_watchdog(response_tx, state);

        // 玩家死亡：回城复活（离线 mock 最小闭环）
        Self::tick_player_respawn(response_tx, state);

        // 群体战斗：两阵营远程玩家互砍
        // 但仍然允许刷/驱动 Boss（给本地玩家一个“大怪目标”）。
        if state.mock_cfg.mass_battle_enabled {
            Self::tick_zone_spawns(response_tx, state);
            Self::tick_monster_combat(response_tx, state);
            Self::tick_monster_wander(response_tx, state);
            Self::tick_mass_battle(response_tx, state);
            return;
        }

        // 远程玩家 AI：更高频率推进
        Self::tick_remote_players_ai(response_tx, state);

        // 刷怪：按区域补足数量
        Self::tick_zone_spawns(response_tx, state);

        // 怪物 AI：追击 + 攻击本地玩家（server-driven combat）
        Self::tick_monster_combat(response_tx, state);

        // 怪物游荡：低频随机走动（避免刷屏/性能）
        Self::tick_monster_wander(response_tx, state);
    }

    fn tick_afk_watchdog(response_tx: &Sender<NetworkEvent>, state: &mut MockWorldState) {
        // 节流：避免刷屏/频繁重建
        let interval = Duration::from_secs(12);
        if state.last_afk_watchdog.elapsed() < interval {
            return;
        }

        // 不开 boss 就没必要做；玩家死了也先不管（会走 respawn）
        if !state.mock_cfg.boss_enabled || state.mock_cfg.boss_count == 0 {
            return;
        }
        if state.player_hp_current <= 0 {
            return;
        }

        let (px, py) = state.player_grid;

        // 判断“附近有没有目标”：只要有任意活怪离得够近，就认为正常。
        // 这里用一个偏大的阈值，防止怪刚刷出来但还没进 aggro_range 就触发重建。
        let near_threshold = state.mock_cfg.boss_aggro_range.max(8).saturating_mul(3);
        let mut nearest: i32 = i32::MAX;
        let mut any_alive = false;
        for m in state.monsters.values() {
            if m.hp <= 0 {
                continue;
            }
            any_alive = true;
            let d = (m.pos.0 - px).abs() + (m.pos.1 - py).abs();
            if d < nearest {
                nearest = d;
            }
            if d <= near_threshold {
                // 附近有活怪：一切正常
                state.last_afk_watchdog = Instant::now();
                return;
            }
        }

        // 没有任何活怪，或活怪都离得很远：认为“站桩风险高”，做一次修复动作。
        // 注意：我们不强制清空怪物表（避免破坏其他压测场景），只保证在玩家附近补一个 boss。
        state.last_afk_watchdog = Instant::now();

        // 以玩家当前位置重建 boss zones，确保 center 不越界/不偏离。
        Self::rebuild_boss_zones(state, state.player_grid);

        // 立刻补一只 boss（只要存在 boss zone）
        let mut spawned = false;
        for zone_idx in 0..state.zones.len() {
            if state.zones.get(zone_idx).map(|z| z.is_boss).unwrap_or(false) {
                Self::spawn_monster_in_zone(response_tx, state, zone_idx);
                spawned = true;
            }
        }

        if spawned {
            let msg = if any_alive {
                format!("(MOCK) AFK watchdog: nearest monster too far (d={}); spawned boss near player", nearest)
            } else {
                "(MOCK) AFK watchdog: no alive monsters; spawned boss near player".to_string()
            };
            let _ = response_tx.send(NetworkEvent::SystemMessage { message: msg });
        }
    }

    fn tick_map_rotation(response_tx: &Sender<NetworkEvent>, state: &mut MockWorldState) {
        let interval = Duration::from_secs(60);
        if state.last_map_rotate.elapsed() < interval {
            return;
        }
        if state.map_rotate_paths.is_empty() {
            return;
        }

        state.last_map_rotate = Instant::now();
        state.map_rotate_idx = (state.map_rotate_idx + 1) % state.map_rotate_paths.len();
        let next_map = state.map_rotate_paths[state.map_rotate_idx].clone();
        state.current_map_path = next_map.clone();

        // ✅ 换图时清理旧怪物：否则它们会停留在旧坐标（可能越界/离玩家极远），
        // 造成“AI 一直站原地（没有附近目标）”并且怪物表无限膨胀。
        if !state.monsters.is_empty() {
            let ids: Vec<u32> = state.monsters.keys().copied().collect();
            for object_id in ids {
                let _ = response_tx.send(NetworkEvent::ObjectRemove {
                    packet: mir2_shared::packets::server::ObjectRemove { object_id },
                });
            }
            state.monsters.clear();
        }

        // 使用当前位置作为“期望落点”，load_and_send_map 会自动修正到可走格。
        let spawn = Self::load_and_send_map(
            response_tx,
            state,
            &next_map,
            state.map_rotate_idx as i32,
            "Mock Rotate",
            state.player_grid.0,
            state.player_grid.1,
            MirDirection::Down as u8,
        );

        state.player_spawn_grid = spawn;
        state.player_grid = spawn;

        // ✅ 换图后重建 Boss 刷新区域：Boss 必须围绕新地图/新落点生成。
        // 否则 zone.center 可能越界，导致永远刷不出怪、挂机站桩。
        Self::rebuild_boss_zones(state, spawn);

        // 立即补一只 Boss（让挂机能立刻开始工作；避免等待 respawn_interval）。
        // 只刷 boss zone，普通怪保持关闭（当前 mock 设计主要用于“大怪目标”）。
        for zone_idx in 0..state.zones.len() {
            if state.zones.get(zone_idx).map(|z| z.is_boss).unwrap_or(false) {
                Self::spawn_monster_in_zone(response_tx, state, zone_idx);
            }
        }

        let _ = response_tx.send(NetworkEvent::PlayerLocationChanged {
            x: state.player_grid.0,
            y: state.player_grid.1,
        });
        let _ = response_tx.send(NetworkEvent::SystemMessage {
            message: format!("(MOCK) Map rotated: {}", next_map),
        });
    }

    pub(super) fn rebuild_boss_zones(state: &mut MockWorldState, prefer_center: (i32, i32)) {
        state.zones.retain(|z| !z.is_boss);

        let cfg = state.mock_cfg.clone();
        if !cfg.boss_enabled || cfg.boss_count == 0 {
            return;
        }

        let (map_w_eff, map_h_eff) = Self::effective_map_dims(state);
        let mut occupied: HashSet<(i32, i32)> = HashSet::new();
        occupied.insert(state.player_grid);

        for _ in 0..cfg.boss_count {
            // 更靠近玩家：避免“进图完全找不到 Boss”。
            let offset_r = (cfg.boss_zone_radius * 2).clamp(4, 28);
            let ox = (Self::rng_next_u32(&mut state.rng) as i32 % (offset_r * 2 + 1)) - offset_r;
            let oy = (Self::rng_next_u32(&mut state.rng) as i32 % (offset_r * 2 + 1)) - offset_r;
            let c0 = (
                (prefer_center.0 + ox).clamp(0, map_w_eff.saturating_sub(1).max(0)),
                (prefer_center.1 + oy).clamp(0, map_h_eff.saturating_sub(1).max(0)),
            );

            let center = Self::pick_random_walkable_near_center_raw(
                state.map_width,
                state.map_height,
                state.map_walkable.as_slice(),
                map_w_eff,
                map_h_eff,
                &mut state.rng,
                &occupied,
                c0,
                cfg.boss_zone_radius * 4,
                4096,
            )
            .unwrap_or(c0);
            occupied.insert(center);

            state.zones.push(MockZone {
                name: "Boss",
                is_boss: true,
                center,
                radius: cfg.boss_zone_radius,
                max_monsters: 1,
                respawn_interval: Duration::from_millis(cfg.boss_respawn_ms.max(200)),
                monster_image: cfg.boss_image,
                monster_hp: cfg.boss_hp.max(1),
                xp_reward: 200,
                // 让 tick_zone_spawns 进入游戏后尽快补一只 boss
                last_spawn: Instant::now() - Duration::from_millis(cfg.boss_respawn_ms.max(200)),
            });
        }
    }

    fn tick_player_respawn(response_tx: &Sender<NetworkEvent>, state: &mut MockWorldState) {
        // sound_id 来自 Sound/SoundList.lst
        const SFX_PLAYER_RESPAWNED: i32 = 10156; // levelup.wav

        // 兜底：若出现“血量已为 0，但未记录死亡开始时间”的异常状态，自动补齐。
        // 否则客户端会一直显示“正在回城复活...”，而服务器永远不会触发 respawn。
        if state.player_hp_current <= 0 && state.player_dead_since.is_none() {
            state.player_dead_since = Some(Instant::now());
            let _ = response_tx.send(NetworkEvent::SystemMessage {
                message: "(MOCK) 修复异常死亡状态：已启动 5 秒回城复活倒计时".to_string(),
            });
        }

        let Some(dead_since) = state.player_dead_since else {
            return;
        };

        // 死亡后停留一段时间：播放死亡动画 + 倒计时回城复活
        if dead_since.elapsed() < Duration::from_secs(5) {
            return;
        }

        state.player_dead_since = None;
        state.player_grid = state.player_spawn_grid;
        state.player_hp_current = state.player_hp_max.max(1);
        state.player_protected_until = Some(Instant::now() + Duration::from_millis(3000));

        let _ = response_tx.send(NetworkEvent::HealthChanged {
            current: state.player_hp_current.max(0) as u32,
            max: state.player_hp_max.max(1) as u32,
        });
        let _ = response_tx.send(NetworkEvent::PlayerLocationChanged {
            x: state.player_grid.0,
            y: state.player_grid.1,
        });
        let _ = response_tx.send(NetworkEvent::PlaySound {
            sound_id: SFX_PLAYER_RESPAWNED,
        });
        let _ = response_tx.send(NetworkEvent::SystemMessage {
            message: "(MOCK) Respawned at town".to_string(),
        });
    }
}
