use super::*;

pub(super) fn normalize_map_path(input: &str) -> String {
    let s = input.trim();
    if s.is_empty() {
        return "Map/n0.map".to_string();
    }
    let mut p = s.replace('\\', "/");
    if !p.contains('/') {
        if !p.to_ascii_lowercase().ends_with(".map") {
            p.push_str(".map");
        }
        p = format!("Map/{p}");
    }
    if !p.to_ascii_lowercase().ends_with(".map") {
        p.push_str(".map");
    }
    p
}

pub(super) fn load_mock_runtime_config() -> MockRuntimeConfig {
    let mut cfg = MockRuntimeConfig::default();

    // 读取 config.ini（与 network/mod.rs 同风格的简易 INI 解析）
    if let Some(content) = crate::network::read_config_ini() {
        let mut section = String::new();
        for raw in content.lines() {
            let line = raw.trim();
            if line.is_empty() || line.starts_with(';') || line.starts_with('#') {
                continue;
            }
            if line.starts_with('[') && line.ends_with(']') {
                section = line[1..line.len() - 1].trim().to_string();
                continue;
            }
            let Some((k, v)) = line.split_once('=') else {
                continue;
            };
            if !section.eq_ignore_ascii_case("Mock") {
                continue;
            }
            let key = k.trim();
            let value = v.trim();

            if key.eq_ignore_ascii_case("RemotePlayers") {
                if let Ok(n) = value.parse::<usize>() {
                    if n > 0 {
                        cfg.remote_players = n;
                    }
                }
                continue;
            }

            if key.eq_ignore_ascii_case("MassBattle") {
                let v = value.trim().to_ascii_lowercase();
                cfg.mass_battle_enabled = matches!(v.as_str(), "1" | "true" | "yes" | "on");
                continue;
            }
            if key.eq_ignore_ascii_case("MassBattleTeamA") {
                if let Ok(n) = value.parse::<usize>() {
                    cfg.mass_battle_team_a = n;
                }
                continue;
            }
            if key.eq_ignore_ascii_case("MassBattleTeamB") {
                if let Ok(n) = value.parse::<usize>() {
                    cfg.mass_battle_team_b = n;
                }
                continue;
            }
            if key.eq_ignore_ascii_case("MassBattleSpawnRadius") {
                if let Ok(v) = value.parse::<i32>() {
                    cfg.mass_battle_spawn_radius = v.max(1);
                }
                continue;
            }
            if key.eq_ignore_ascii_case("MassBattleEngageRange") {
                if let Ok(v) = value.parse::<i32>() {
                    cfg.mass_battle_engage_range = v.max(1);
                }
                continue;
            }
            if key.eq_ignore_ascii_case("MassBattleRespawnMs") {
                if let Ok(v) = value.parse::<u64>() {
                    cfg.mass_battle_respawn_ms = v.max(200);
                }
                continue;
            }
            if key.eq_ignore_ascii_case("MassBattleAttackersPerTick") {
                if let Ok(v) = value.parse::<usize>() {
                    cfg.mass_battle_attackers_per_tick = v.max(1);
                }
                continue;
            }
            if key.eq_ignore_ascii_case("MassBattleBucket") {
                if let Ok(v) = value.parse::<i32>() {
                    cfg.mass_battle_bucket = v.max(1);
                }
                continue;
            }

            if key.eq_ignore_ascii_case("BossEnabled") {
                let v = value.trim().to_ascii_lowercase();
                cfg.boss_enabled = matches!(v.as_str(), "1" | "true" | "yes" | "on");
                continue;
            }
            if key.eq_ignore_ascii_case("BossCount") {
                if let Ok(v) = value.parse::<usize>() {
                    cfg.boss_count = v;
                }
                continue;
            }
            if key.eq_ignore_ascii_case("BossImage") {
                if let Ok(v) = value.parse::<u16>() {
                    cfg.boss_image = v;
                }
                continue;
            }
            if key.eq_ignore_ascii_case("BossHp") {
                if let Ok(v) = value.parse::<i32>() {
                    cfg.boss_hp = v.max(1);
                }
                continue;
            }
            if key.eq_ignore_ascii_case("BossRespawnMs") {
                if let Ok(v) = value.parse::<u64>() {
                    cfg.boss_respawn_ms = v.max(200);
                }
                continue;
            }
            if key.eq_ignore_ascii_case("BossZoneRadius") {
                if let Ok(v) = value.parse::<i32>() {
                    cfg.boss_zone_radius = v.max(1);
                }
                continue;
            }
            if key.eq_ignore_ascii_case("BossAggroRange") {
                if let Ok(v) = value.parse::<i32>() {
                    cfg.boss_aggro_range = v.max(1);
                }
                continue;
            }
            if key.eq_ignore_ascii_case("BossDamageMin") {
                if let Ok(v) = value.parse::<i32>() {
                    cfg.boss_damage_min = v;
                }
                continue;
            }
            if key.eq_ignore_ascii_case("BossDamageMax") {
                if let Ok(v) = value.parse::<i32>() {
                    cfg.boss_damage_max = v;
                }
                continue;
            }
            if key.eq_ignore_ascii_case("BossAttackCooldownMs") {
                if let Ok(v) = value.parse::<u64>() {
                    cfg.boss_attack_cooldown_ms = v.max(50);
                }
                continue;
            }
            if key.eq_ignore_ascii_case("StartMap") {
                if !value.is_empty() {
                    cfg.start_map = normalize_map_path(value);
                }
                continue;
            }

            if key.eq_ignore_ascii_case("WeaponMin") {
                if let Ok(v) = value.parse::<i16>() {
                    cfg.weapon_min = v;
                }
                continue;
            }
            if key.eq_ignore_ascii_case("WeaponMax") {
                if let Ok(v) = value.parse::<i16>() {
                    cfg.weapon_max = v;
                }
                continue;
            }
            if key.eq_ignore_ascii_case("ArmourMin") {
                if let Ok(v) = value.parse::<i16>() {
                    cfg.armour_min = v;
                }
                continue;
            }
            if key.eq_ignore_ascii_case("ArmourMax") {
                if let Ok(v) = value.parse::<i16>() {
                    cfg.armour_max = v;
                }
                continue;
            }
            if key.eq_ignore_ascii_case("WeaponEffectMin") {
                if let Ok(v) = value.parse::<i16>() {
                    cfg.weapon_effect_min = v;
                }
                continue;
            }
            if key.eq_ignore_ascii_case("WeaponEffectMax") {
                if let Ok(v) = value.parse::<i16>() {
                    cfg.weapon_effect_max = v;
                }
                continue;
            }
            if key.eq_ignore_ascii_case("WingEffectMin") {
                if let Ok(v) = value.parse::<u8>() {
                    cfg.wing_effect_min = v;
                }
                continue;
            }
            if key.eq_ignore_ascii_case("WingEffectMax") {
                if let Ok(v) = value.parse::<u8>() {
                    cfg.wing_effect_max = v;
                }
                continue;
            }

            if key.eq_ignore_ascii_case("MountMin") {
                if let Ok(v) = value.parse::<i16>() {
                    cfg.mount_min = v;
                }
                continue;
            }
            if key.eq_ignore_ascii_case("MountMax") {
                if let Ok(v) = value.parse::<i16>() {
                    cfg.mount_max = v;
                }
                continue;
            }
            if key.eq_ignore_ascii_case("MountMinLevel") {
                if let Ok(v) = value.parse::<u16>() {
                    cfg.mount_min_level = v;
                }
                continue;
            }
            if key.eq_ignore_ascii_case("MountChance") {
                if let Ok(v) = value.parse::<f32>() {
                    cfg.mount_chance = v.clamp(0.0, 1.0);
                }
                continue;
            }

            if key.eq_ignore_ascii_case("AiTickMs") {
                if let Ok(v) = value.parse::<u64>() {
                    cfg.ai_tick_ms = v.max(10);
                }
                continue;
            }
            if key.eq_ignore_ascii_case("ZoneEvalMs") {
                if let Ok(v) = value.parse::<u64>() {
                    cfg.zone_eval_ms = v.max(50);
                }
                continue;
            }
            if key.eq_ignore_ascii_case("RoamPickMs") {
                if let Ok(v) = value.parse::<u64>() {
                    cfg.roam_pick_ms = v.max(50);
                }
                continue;
            }
            if key.eq_ignore_ascii_case("RestChance") {
                if let Ok(v) = value.parse::<f32>() {
                    cfg.rest_chance = v.clamp(0.0, 1.0);
                }
                continue;
            }
            if key.eq_ignore_ascii_case("RestMs") {
                if let Ok(v) = value.parse::<u64>() {
                    cfg.rest_ms = v;
                }
                continue;
            }
            if key.eq_ignore_ascii_case("Perception") {
                if let Ok(v) = value.parse::<i32>() {
                    cfg.perception = v.max(1);
                }
                continue;
            }
            if key.eq_ignore_ascii_case("ChaseDrop") {
                if let Ok(v) = value.parse::<i32>() {
                    cfg.chase_drop = v.max(2);
                }
                continue;
            }
            if key.eq_ignore_ascii_case("AttackCooldownMs") {
                if let Ok(v) = value.parse::<u64>() {
                    cfg.attack_cooldown_ms = v.max(50);
                }
                continue;
            }
        }
    }

    // env 覆盖：保留老习惯（CRYSTAL_* 优先于 config.ini）
    cfg.remote_players = std::env::var(CRYSTAL_REMOTE_PLAYERS)
        .ok()
        .and_then(|v| v.parse::<usize>().ok())
        .filter(|v| *v > 0)
        .unwrap_or(cfg.remote_players);

    if let Ok(v) = std::env::var(CRYSTAL_START_MAP) {
        if !v.trim().is_empty() {
            cfg.start_map = normalize_map_path(v.trim());
        }
    }

    // 修正 min/max 颠倒
    if cfg.weapon_min > cfg.weapon_max {
        std::mem::swap(&mut cfg.weapon_min, &mut cfg.weapon_max);
    }
    if cfg.armour_min > cfg.armour_max {
        std::mem::swap(&mut cfg.armour_min, &mut cfg.armour_max);
    }
    if cfg.weapon_effect_min > cfg.weapon_effect_max {
        std::mem::swap(&mut cfg.weapon_effect_min, &mut cfg.weapon_effect_max);
    }
    if cfg.wing_effect_min > cfg.wing_effect_max {
        std::mem::swap(&mut cfg.wing_effect_min, &mut cfg.wing_effect_max);
    }
    if cfg.mount_min > cfg.mount_max {
        std::mem::swap(&mut cfg.mount_min, &mut cfg.mount_max);
    }

    println!(
        "[MOCK][CFG] mass_battle={} team_a={} team_b={} attackers_per_tick={} bucket={} engage={} spawn_r={} remote_players={} start_map={} boss={} boss_count={} boss_image={} boss_hp={} boss_respawn_ms={}",
        cfg.mass_battle_enabled,
        cfg.mass_battle_team_a,
        cfg.mass_battle_team_b,
        cfg.mass_battle_attackers_per_tick,
        cfg.mass_battle_bucket,
        cfg.mass_battle_engage_range,
        cfg.mass_battle_spawn_radius,
        cfg.remote_players,
        cfg.start_map,
        cfg.boss_enabled,
        cfg.boss_count,
        cfg.boss_image,
        cfg.boss_hp,
        cfg.boss_respawn_ms,
    );

    cfg
}
