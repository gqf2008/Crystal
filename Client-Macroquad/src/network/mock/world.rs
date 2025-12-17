use super::*;

impl MockNetwork {
    pub(super) fn tick_world(response_tx: &Sender<NetworkEvent>, state: &mut MockWorldState) {
        if !state.in_game {
            return;
        }

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
        let Some(dead_since) = state.player_dead_since else {
            return;
        };

        // 给一个短的死亡停留时间，让死亡音效/飘字可见
        if dead_since.elapsed() < Duration::from_millis(1600) {
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
        let _ = response_tx.send(NetworkEvent::SystemMessage {
            message: "(MOCK) Respawned at town".to_string(),
        });
    }
}
