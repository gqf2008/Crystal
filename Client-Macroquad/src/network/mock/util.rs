use super::*;

impl MockNetwork {
    pub(super) fn exp_for_next_level(level: u16) -> i64 {
        // 简单曲线：等级越高升级所需越高。离线 mock 不追求严格还原。
        (level as i64) * 60 + 40
    }

    pub(super) fn dir_from_delta(dx: i32, dy: i32) -> mir2_shared::enums::MirDirection {
        match (dx.signum(), dy.signum()) {
            (0, -1) => mir2_shared::enums::MirDirection::Up,
            (1, -1) => mir2_shared::enums::MirDirection::UpRight,
            (1, 0) => mir2_shared::enums::MirDirection::Right,
            (1, 1) => mir2_shared::enums::MirDirection::DownRight,
            (0, 1) => mir2_shared::enums::MirDirection::Down,
            (-1, 1) => mir2_shared::enums::MirDirection::DownLeft,
            (-1, 0) => mir2_shared::enums::MirDirection::Left,
            (-1, -1) => mir2_shared::enums::MirDirection::UpLeft,
            _ => mir2_shared::enums::MirDirection::Down,
        }
    }

    pub(super) fn send_object_player_update(response_tx: &Sender<NetworkEvent>, rp: &MockRemotePlayerState) {
        let _ = response_tx.send(NetworkEvent::ObjectPlayer {
            packet: mir2_shared::packets::server::ObjectPlayer {
                object_id: rp.id,
                name: rp.name.clone(),
                guild_name: "".to_string(),
                guild_rank_name: "".to_string(),
                name_colour: 0,
                class: rp.class,
                gender: rp.gender,
                level: rp.level,
                location_x: rp.grid.0,
                location_y: rp.grid.1,
                direction: rp.direction,
                hair: rp.hair,
                light: 0,
                weapon: rp.weapon,
                weapon_effect: rp.weapon_effect,
                armour: rp.armour,
                poison: mir2_shared::enums::PoisonType::empty(),
                dead: false,
                hidden: false,
                effect: mir2_shared::enums::SpellEffect::None,
                wing_effect: rp.wing_effect,
                extra: false,
                mount_type: rp.mount_type,
                riding_mount: rp.riding_mount,
                fishing: false,
                transform_type: 0,
                element_orb_effect: 0,
                element_orb_lvl: 0,
                element_orb_max: 0,
                buffs: Vec::new(),
                level_effects: mir2_shared::enums::LevelEffects::empty(),
            },
        });
    }

    pub(super) fn rng_next_u32(seed: &mut u64) -> u32 {
        // xorshift64*
        let mut x = *seed;
        x ^= x >> 12;
        x ^= x << 25;
        x ^= x >> 27;
        *seed = x;
        ((x.wrapping_mul(0x2545F4914F6CDD1D_u64)) >> 32) as u32
    }

    pub(super) fn random_pos_in_zone(seed: &mut u64, zone: &MockZone) -> (i32, i32) {
        let r = zone.radius.max(1);
        let dx = (Self::rng_next_u32(seed) as i32 % (r * 2 + 1)) - r;
        let dy = (Self::rng_next_u32(seed) as i32 % (r * 2 + 1)) - r;
        (zone.center.0 + dx, zone.center.1 + dy)
    }

    pub(super) fn effective_map_dims(state: &MockWorldState) -> (i32, i32) {
        if state.map_width > 0 && state.map_height > 0 {
            return (state.map_width, state.map_height);
        }

        let map_w: i32 = std::env::var("CRYSTAL_MOCK_MAP_W")
            .ok()
            .and_then(|v| v.parse::<i32>().ok())
            .filter(|v| *v > 0)
            .unwrap_or(700);
        let map_h: i32 = std::env::var("CRYSTAL_MOCK_MAP_H")
            .ok()
            .and_then(|v| v.parse::<i32>().ok())
            .filter(|v| *v > 0)
            .unwrap_or(700);

        (map_w.max(1), map_h.max(1))
    }

    pub(super) fn map_is_walkable_raw(
        map_width: i32,
        map_height: i32,
        map_walkable: &[u8],
        x: i32,
        y: i32,
    ) -> bool {
        if x < 0 || y < 0 {
            return false;
        }
        if map_width > 0 && map_height > 0 {
            if x >= map_width || y >= map_height {
                return false;
            }
        }
        if map_walkable.is_empty() || map_width <= 0 || map_height <= 0 {
            // 未缓存碰撞：退化为“全部可走”
            return true;
        }
        let idx = (y as usize).saturating_mul(map_width as usize) + (x as usize);
        map_walkable.get(idx).copied().unwrap_or(1) != 0
    }

    /// 在整个地图范围内做“均匀采样 + walkable 拒绝采样”，因此对 walkable 格是严格均匀分布。
    pub(super) fn pick_random_walkable_unoccupied_raw(
        map_width: i32,
        map_height: i32,
        map_walkable: &[u8],
        map_w_eff: i32,
        map_h_eff: i32,
        rng: &mut u64,
        occupied: &HashSet<(i32, i32)>,
        max_tries: usize,
    ) -> Option<(i32, i32)> {
        if map_w_eff <= 0 || map_h_eff <= 0 {
            return None;
        }

        for _ in 0..max_tries.max(1) {
            let x = (Self::rng_next_u32(rng) as i32).rem_euclid(map_w_eff);
            let y = (Self::rng_next_u32(rng) as i32).rem_euclid(map_h_eff);
            if !Self::map_is_walkable_raw(map_width, map_height, map_walkable, x, y) {
                continue;
            }
            if occupied.contains(&(x, y)) {
                continue;
            }
            return Some((x, y));
        }
        None
    }

    pub(super) fn pick_random_walkable_near_center_raw(
        map_width: i32,
        map_height: i32,
        map_walkable: &[u8],
        map_w_eff: i32,
        map_h_eff: i32,
        rng: &mut u64,
        occupied: &HashSet<(i32, i32)>,
        center: (i32, i32),
        radius: i32,
        max_tries: usize,
    ) -> Option<(i32, i32)> {
        if map_w_eff <= 0 || map_h_eff <= 0 {
            return None;
        }
        let r = radius.max(1);

        for _ in 0..max_tries.max(1) {
            let dx = (Self::rng_next_u32(rng) as i32 % (r * 2 + 1)) - r;
            let dy = (Self::rng_next_u32(rng) as i32 % (r * 2 + 1)) - r;
            let x = (center.0 + dx).clamp(0, map_w_eff.saturating_sub(1).max(0));
            let y = (center.1 + dy).clamp(0, map_h_eff.saturating_sub(1).max(0));

            if !Self::map_is_walkable_raw(map_width, map_height, map_walkable, x, y) {
                continue;
            }
            if occupied.contains(&(x, y)) {
                continue;
            }
            return Some((x, y));
        }

        None
    }

    /// 以 prefer_a 为起点做 BFS，确保返回的 (a,b) 在同一可走连通分量内。
    ///
    /// 典型问题：map 的左右两侧被墙完全隔开时，若中心点固定在左右，
    /// 所有单位会“朝不可达目标推进”，最终挤在最近可达的墙角/死胡同。
    pub(super) fn pick_connected_mass_battle_centers_raw(
        map_width: i32,
        map_height: i32,
        map_walkable: &[u8],
        map_w_eff: i32,
        map_h_eff: i32,
        rng: &mut u64,
        prefer_a: (i32, i32),
        prefer_b: (i32, i32),
        sample_radius: i32,
    ) -> Option<((i32, i32), (i32, i32))> {
        use std::collections::VecDeque;

        if map_width <= 0 || map_height <= 0 || map_w_eff <= 0 || map_h_eff <= 0 {
            return None;
        }

        let empty: HashSet<(i32, i32)> = HashSet::new();

        // 先把 prefer 点修正到附近的可走格。
        let a = Self::pick_random_walkable_near_center_raw(
            map_width,
            map_height,
            map_walkable,
            map_w_eff,
            map_h_eff,
            rng,
            &empty,
            prefer_a,
            sample_radius,
            4096,
        )
        .or_else(|| {
            Self::pick_random_walkable_unoccupied_raw(
                map_width,
                map_height,
                map_walkable,
                map_w_eff,
                map_h_eff,
                rng,
                &empty,
                4096,
            )
        })
        .unwrap_or(prefer_a);

        let b0 = Self::pick_random_walkable_near_center_raw(
            map_width,
            map_height,
            map_walkable,
            map_w_eff,
            map_h_eff,
            rng,
            &empty,
            prefer_b,
            sample_radius,
            4096,
        )
        .or_else(|| {
            Self::pick_random_walkable_unoccupied_raw(
                map_width,
                map_height,
                map_walkable,
                map_w_eff,
                map_h_eff,
                rng,
                &empty,
                4096,
            )
        })
        .unwrap_or(prefer_b);

        // BFS distances from a
        let mut dist: Vec<i32> = vec![-1; (map_width as usize).saturating_mul(map_height as usize)];
        let idx = |x: i32, y: i32| (y as usize).saturating_mul(map_width as usize) + (x as usize);

        if !Self::map_is_walkable_raw(map_width, map_height, map_walkable, a.0, a.1) {
            return None;
        }
        let a_idx = idx(a.0, a.1);
        if a_idx >= dist.len() {
            return None;
        }
        dist[a_idx] = 0;
        let mut q: VecDeque<(i32, i32)> = VecDeque::new();
        q.push_back(a);

        while let Some((x, y)) = q.pop_front() {
            let base = dist[idx(x, y)];
            let neighbors = [(x + 1, y), (x - 1, y), (x, y + 1), (x, y - 1)];
            for (nx, ny) in neighbors {
                if nx < 0 || ny < 0 || nx >= map_w_eff || ny >= map_h_eff {
                    continue;
                }
                if !Self::map_is_walkable_raw(map_width, map_height, map_walkable, nx, ny) {
                    continue;
                }
                let ni = idx(nx, ny);
                if ni >= dist.len() {
                    continue;
                }
                if dist[ni] != -1 {
                    continue;
                }
                dist[ni] = base + 1;
                q.push_back((nx, ny));
            }
        }

        // b0 如果可达就直接用；否则在“可达点里”挑一个最接近 prefer_b 的点。
        let mut best_b: Option<(i32, i32)> = None;
        let mut best_score: i32 = i32::MAX;

        let b0i = idx(b0.0, b0.1);
        if b0i < dist.len() && dist[b0i] >= 0 {
            best_b = Some(b0);
        } else {
            // 在 b0 附近采样一些点，挑可达且离 prefer_b 最近的。
            let sr = sample_radius.max(6);
            for _ in 0..2048 {
                let dx = (Self::rng_next_u32(rng) as i32 % (sr * 2 + 1)) - sr;
                let dy = (Self::rng_next_u32(rng) as i32 % (sr * 2 + 1)) - sr;
                let x = (prefer_b.0 + dx).clamp(0, map_w_eff.saturating_sub(1).max(0));
                let y = (prefer_b.1 + dy).clamp(0, map_h_eff.saturating_sub(1).max(0));

                if !Self::map_is_walkable_raw(map_width, map_height, map_walkable, x, y) {
                    continue;
                }
                let di = idx(x, y);
                if di >= dist.len() {
                    continue;
                }
                if dist[di] < 0 {
                    continue;
                }
                // 更靠近 prefer_b 的可达点更优；同分时倾向更远（让战线更拉开）
                let man_to_prefer = (x - prefer_b.0).abs() + (y - prefer_b.1).abs();
                let score = man_to_prefer;
                if score < best_score {
                    best_score = score;
                    best_b = Some((x, y));
                }
            }
        }

        best_b.filter(|b| *b != a).map(|b| (a, b))
    }

    pub(super) fn nearest_zone_idx(zones: &[MockZone], x: i32, y: i32) -> usize {
        if zones.is_empty() {
            return 0;
        }
        let mut best_idx = 0usize;
        let mut best_dist: i32 = i32::MAX;
        for (idx, z) in zones.iter().enumerate() {
            let d = (z.center.0 - x).abs() + (z.center.1 - y).abs();
            if d < best_dist {
                best_dist = d;
                best_idx = idx;
            }
        }
        best_idx
    }

    pub(super) fn map_is_walkable(state: &MockWorldState, x: i32, y: i32) -> bool {
        if x < 0 || y < 0 {
            return false;
        }
        if state.map_width > 0 && state.map_height > 0 {
            if x >= state.map_width || y >= state.map_height {
                return false;
            }
        }
        if state.map_walkable.is_empty() || state.map_width <= 0 || state.map_height <= 0 {
            // 未加载碰撞：退化为“全部可走”
            return true;
        }
        let idx = (y as usize).saturating_mul(state.map_width as usize) + (x as usize);
        state.map_walkable.get(idx).copied().unwrap_or(1) != 0
    }
}
