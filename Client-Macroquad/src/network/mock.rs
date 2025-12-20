// ============================================================================
// Mock Network - 离线模拟网络（单机基础）
// ============================================================================

mod config;
mod map;
mod mass_battle;
mod monsters;
mod remote_ai;
mod util;
mod world;

use super::NetworkEvent;

use crossbeam_channel::{unbounded, Receiver, Sender};
use std::collections::{HashMap, HashSet};
use std::thread;
use std::time::{Duration, Instant};

use chrono::Utc;

use mir2_shared::data::client_data::SelectInfo;
use mir2_shared::data::item::{ItemInfo, UserItem};
use mir2_shared::enums::{ChatType, HeroBehaviour, ItemType, MirClass, MirDirection, MirGender, PanelType};

pub const CRYSTAL_REMOTE_PLAYERS: &str = "CRYSTAL_REMOTE_PLAYERS";
pub const CRYSTAL_START_MAP: &str = "CRYSTAL_START_MAP";

pub struct MockNetwork;

#[derive(Debug, Clone)]
pub struct MockRuntimeConfig {
    pub remote_players: usize,
    pub start_map: String,

    // Remote AI
    pub ai_tick_ms: u64,
    pub zone_eval_ms: u64,
    pub roam_pick_ms: u64,
    pub rest_chance: f32,
    pub rest_ms: u64,
    pub perception: i32,
    pub chase_drop: i32,
    pub attack_cooldown_ms: u64,

    // Cosmetics / equipment ranges
    pub weapon_min: i16,
    pub weapon_max: i16,
    pub armour_min: i16,
    pub armour_max: i16,
    pub weapon_effect_min: i16,
    pub weapon_effect_max: i16,
    pub wing_effect_min: u8,
    pub wing_effect_max: u8,

    // Mounts
    pub mount_min: i16,
    pub mount_max: i16,
    pub mount_min_level: u16,
    pub mount_chance: f32,

    // MassBattle
    pub mass_battle_enabled: bool,
    pub mass_battle_team_a: usize,
    pub mass_battle_team_b: usize,
    pub mass_battle_spawn_radius: i32,
    pub mass_battle_engage_range: i32,
    pub mass_battle_respawn_ms: u64,
    pub mass_battle_attackers_per_tick: usize,
    pub mass_battle_bucket: i32,

    // Boss
    pub boss_enabled: bool,
    pub boss_count: usize,
    pub boss_image: u16,
    pub boss_hp: i32,
    pub boss_respawn_ms: u64,
    pub boss_zone_radius: i32,
    pub boss_aggro_range: i32,
    pub boss_damage_min: i32,
    pub boss_damage_max: i32,
    pub boss_attack_cooldown_ms: u64,
}

impl Default for MockRuntimeConfig {
    fn default() -> Self {
        Self {
            remote_players: 25,
            start_map: "Map/n0.map".to_string(),

            ai_tick_ms: 60,
            zone_eval_ms: 800,
            roam_pick_ms: 1000,
            rest_chance: 0.08,
            rest_ms: 420,
            perception: 9,
            chase_drop: 10,
            attack_cooldown_ms: 900,

            weapon_min: 0,
            weapon_max: 60,
            armour_min: 0,
            armour_max: 60,
            weapon_effect_min: 0,
            weapon_effect_max: 16,
            wing_effect_min: 0,
            wing_effect_max: 0,

            mount_min: 0,
            mount_max: 0,
            mount_min_level: 8,
            mount_chance: 0.1,

            mass_battle_enabled: false,
            mass_battle_team_a: 60,
            mass_battle_team_b: 60,
            mass_battle_spawn_radius: 25,
            mass_battle_engage_range: 16,
            mass_battle_respawn_ms: 4000,
            mass_battle_attackers_per_tick: 24,
            mass_battle_bucket: 8,

            boss_enabled: true,
            boss_count: 1,
            boss_image: 902,
            boss_hp: 6000,
            boss_respawn_ms: 25000,
            boss_zone_radius: 18,
            boss_aggro_range: 16,
            boss_damage_min: 18,
            boss_damage_max: 35,
            boss_attack_cooldown_ms: 1200,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum RemoteAiMode {
    Roam,
    Seek,
    Chase,
    Fight,
    Travel,
    Rest,
}

#[derive(Debug, Clone)]
struct MockRemotePlayerState {
    id: u32,
    name: String,
    class: MirClass,
    gender: MirGender,
    level: u16,

    grid: (i32, i32),
    direction: MirDirection,
    hair: u8,

    // visuals
    weapon: i16,
    weapon_effect: i16,
    armour: i16,
    wing_effect: u8,
    mount_type: i16,
    riding_mount: bool,

    // inspect
    equipment: Vec<Option<UserItem>>,

    // ai
    mode: RemoteAiMode,
    last_mode_change: Instant,
    last_zone_eval: Instant,
    goal_zone_idx: usize,
    zone_idx: usize,
    last_roam_pick: Instant,
    roam_goal: (i32, i32),
    target_monster_id: Option<u32>,

    // combat
    hp_current: i32,
    hp_max: i32,
    experience: i64,
    max_experience: i64,
    last_attack: Instant,
    last_tick: Instant,

    // mass battle
    team: u8,
    dead_until: Option<Instant>,
}

#[derive(Debug, Clone, Copy)]
struct MockMonsterState {
    pos: (i32, i32),
    hp: i32,
    zone_idx: usize,
    xp_reward: i64,
    last_chase_step: Instant,
    last_attack: Instant,
}

#[derive(Debug, Clone)]
struct MockZone {
    name: &'static str,
    is_boss: bool,
    center: (i32, i32),
    radius: i32,
    max_monsters: usize,
    respawn_interval: Duration,
    monster_image: u16,
    monster_hp: i32,
    xp_reward: i64,
    last_spawn: Instant,
}

struct MockWorldState {
    in_game: bool,
    mock_cfg: MockRuntimeConfig,
    rng: u64,

    // Character list
    characters: Vec<SelectInfo>,
    active_character_index: i32,
    active_character_name: String,

    // Map + collision
    current_map_path: String,
    map_width: i32,
    map_height: i32,
    map_walkable: Vec<u8>,

    // Map rotation (debug/dev)
    map_rotate_paths: Vec<String>,
    map_rotate_idx: usize,
    last_map_rotate: Instant,

    // Local player (server-authoritative)
    player_object_id: u32,
    player_level: u16,
    player_experience: i64,
    player_max_experience: i64,
    player_gold: u32,
    inventory_capacity: usize,
    player_inventory: Vec<Option<UserItem>>,
    player_grid: (i32, i32),
    player_spawn_grid: (i32, i32),
    player_hp_current: i32,
    player_hp_max: i32,
    player_dead_since: Option<Instant>,
    player_protected_until: Option<Instant>,
    last_player_move_req: Instant,

    // NPC shop
    last_shop_goods: Vec<UserItem>,

    // Local player visual state (for UserInformation)
    player_equipment: Vec<Option<UserItem>>,
    player_mount_type: i16,
    player_riding_mount: bool,

    // World objects
    remote_players: Vec<MockRemotePlayerState>,
    monsters: HashMap<u32, MockMonsterState>,

    // Zones / spawns
    zones: Vec<MockZone>,
    next_monster_id: u32,
    last_monster_wander_tick: Instant,
    last_monster_combat_tick: Instant,

    // MassBattle cached centers
    mass_battle_center_a: Option<(i32, i32)>,
    mass_battle_center_b: Option<(i32, i32)>,
}

impl MockWorldState {
    fn new(cfg: MockRuntimeConfig) -> Self {
        let now = Instant::now();
        let inventory_capacity: usize = 46;
        let mut characters = Vec::new();
        characters.push(SelectInfo {
            index: 0,
            name: "Hero".to_string(),
            level: 1,
            class: MirClass::Warrior,
            gender: MirGender::Male,
            last_access: Utc::now(),
        });

        // Map rotation list: keep small & stable to avoid unexpected missing assets.
        // Ensure the configured start_map is included as the first entry.
        let mut map_rotate_paths: Vec<String> = Vec::new();
        map_rotate_paths.push(cfg.start_map.clone());
        for p in [
            "Map/n0.map",
            "Map/0.map",
            "Map/1.map",
            "Map/2.map",
            "Map/3.map",
            "Map/whitevillage.map",
        ] {
            if !map_rotate_paths.iter().any(|x| x.eq_ignore_ascii_case(p)) {
                map_rotate_paths.push(p.to_string());
            }
        }

        Self {
            in_game: false,
            mock_cfg: cfg.clone(),
            rng: 0xC0FFEE_u64,

            characters,
            active_character_index: 0,
            active_character_name: "Hero".to_string(),

            current_map_path: cfg.start_map.clone(),
            map_width: 0,
            map_height: 0,
            map_walkable: Vec::new(),

            map_rotate_paths,
            map_rotate_idx: 0,
            last_map_rotate: now,

            player_object_id: 1,
            player_level: 1,
            player_experience: 0,
            player_max_experience: 100,
            player_gold: 5000,
            inventory_capacity,
            player_inventory: vec![None; inventory_capacity],
            player_grid: (330, 330),
            player_spawn_grid: (330, 330),
            player_hp_current: 120,
            player_hp_max: 120,
            player_dead_since: None,
            player_protected_until: Some(now + Duration::from_millis(3000)),
            last_player_move_req: now,

            last_shop_goods: Vec::new(),
            player_equipment: vec![None; 14],
            player_mount_type: 0,
            player_riding_mount: false,
            remote_players: Vec::new(),
            monsters: HashMap::new(),

            zones: Vec::new(),
            next_monster_id: 5000,
            last_monster_wander_tick: now,
            last_monster_combat_tick: now,

            mass_battle_center_a: None,
            mass_battle_center_b: None,
        }
    }
}

impl MockNetwork {
    pub fn new() -> (Sender<NetworkEvent>, Receiver<NetworkEvent>) {
        let (client_tx, client_rx) = unbounded::<NetworkEvent>();
        let (server_tx, server_rx) = unbounded::<NetworkEvent>();

        thread::spawn(move || {
            let cfg = config::load_mock_runtime_config();
            let mut state = MockWorldState::new(cfg);

            let _ = server_tx.send(NetworkEvent::Connected);
            let tick_sleep = Duration::from_millis(10);

            loop {
                // event pump
                match client_rx.recv_timeout(tick_sleep) {
                    Ok(ev) => {
                        let should_quit = matches!(ev, NetworkEvent::DisconnectRequest);
                        Self::handle_game_event(ev, &server_tx, &mut state);
                        if should_quit {
                            break;
                        }
                    }
                    Err(crossbeam_channel::RecvTimeoutError::Timeout) => {}
                    Err(crossbeam_channel::RecvTimeoutError::Disconnected) => break,
                }

                Self::tick_world(&server_tx, &mut state);
            }
        });

        (client_tx, server_rx)
    }

    fn handle_game_event(event: NetworkEvent, response_tx: &Sender<NetworkEvent>, state: &mut MockWorldState) {
        match event {
            NetworkEvent::DisconnectRequest => {
                let _ = response_tx.send(NetworkEvent::Disconnected {
                    reason: "(MOCK) Disconnected".to_string(),
                });
            }

            NetworkEvent::KeepAliveSend { time } => {
                let _ = response_tx.send(NetworkEvent::KeepAliveReceived { time });
            }

            NetworkEvent::ClientVersionSend { .. } => {
                let _ = response_tx.send(NetworkEvent::ClientVersionResponse { result: 1 });
            }

            NetworkEvent::LoginRequest { .. } => {
                let _ = response_tx.send(NetworkEvent::LoginSuccess {
                    characters: state.characters.clone(),
                });
            }

            NetworkEvent::NewCharacterRequest { name, class, gender } => {
                let class = MirClass::try_from(class).unwrap_or(MirClass::Warrior);
                let gender = MirGender::try_from(gender).unwrap_or(MirGender::Male);
                let next_index = state.characters.len() as i32;
                state.characters.push(SelectInfo {
                    index: next_index,
                    name: name.clone(),
                    level: 1,
                    class,
                    gender,
                    last_access: Utc::now(),
                });
                let _ = response_tx.send(NetworkEvent::CharacterCreated { name });
                let _ = response_tx.send(NetworkEvent::LoginSuccess {
                    characters: state.characters.clone(),
                });
            }

            NetworkEvent::DeleteCharacterRequest { index } => {
                state.characters.retain(|c| c.index != index);
                let _ = response_tx.send(NetworkEvent::CharacterDeleted { index: index as u32 });
                let _ = response_tx.send(NetworkEvent::LoginSuccess {
                    characters: state.characters.clone(),
                });
            }

            NetworkEvent::StartGameRequest { character_index } => {
                state.in_game = true;
                state.active_character_index = character_index;
                if let Some(c) = state.characters.iter().find(|c| c.index == character_index) {
                    state.active_character_name = c.name.clone();
                    state.player_level = c.level.max(1);
                }

                let _ = response_tx.send(NetworkEvent::StartGame {
                    packet: mir2_shared::packets::server::StartGame {
                        result: 4,
                        resolution: 0,
                    },
                });

                // load map & spawn
                let spawn = Self::load_and_send_map(
                    response_tx,
                    state,
                    &state.current_map_path.clone(),
                    0,
                    "Mock",
                    state.player_spawn_grid.0,
                    state.player_spawn_grid.1,
                    MirDirection::Down as u8,
                );
                state.player_spawn_grid = spawn;
                state.player_grid = spawn;

                // base zones
                state.zones.clear();
                state.monsters.clear();
                state.next_monster_id = 5000;
                state.mass_battle_center_a = None;
                state.mass_battle_center_b = None;

                let (map_w_eff, map_h_eff) = Self::effective_map_dims(state);
                let mut occupied: HashSet<(i32, i32)> = HashSet::new();
                occupied.insert(state.player_grid);

                // Normal zones
                let normal_zone_count = 6usize;
                for i in 0..normal_zone_count {
                    let prefer = (
                        (map_w_eff / 2 + (i as i32 - 3) * 55).clamp(0, map_w_eff.saturating_sub(1).max(0)),
                        (map_h_eff / 2 + ((i as i32 % 3) - 1) * 55).clamp(0, map_h_eff.saturating_sub(1).max(0)),
                    );
                    let center = Self::pick_random_walkable_near_center_raw(
                        state.map_width,
                        state.map_height,
                        &state.map_walkable,
                        map_w_eff,
                        map_h_eff,
                        &mut state.rng,
                        &occupied,
                        prefer,
                        40,
                        4096,
                    )
                    .or_else(|| {
                        Self::pick_random_walkable_unoccupied_raw(
                            state.map_width,
                            state.map_height,
                            &state.map_walkable,
                            map_w_eff,
                            map_h_eff,
                            &mut state.rng,
                            &occupied,
                            4096,
                        )
                    })
                    .unwrap_or(prefer);
                    occupied.insert(center);

                    state.zones.push(MockZone {
                        name: "Zone",
                        is_boss: false,
                        center,
                        radius: 22,
                        max_monsters: 6,
                        respawn_interval: Duration::from_millis(2200),
                        monster_image: 20 + (i as u16 % 10),
                        monster_hp: 80,
                        xp_reward: 25,
                        last_spawn: Instant::now() - Duration::from_millis(99999),
                    });
                }

                // Boss zones
                Self::rebuild_boss_zones(state, state.player_grid);

                // Spawn initial monsters quickly
                Self::tick_zone_spawns(response_tx, state);

                // Remote players
                state.remote_players.clear();
                let (count, team_a_count) = if state.mock_cfg.mass_battle_enabled {
                    (
                        state
                            .mock_cfg
                            .mass_battle_team_a
                            .saturating_add(state.mock_cfg.mass_battle_team_b),
                        state.mock_cfg.mass_battle_team_a,
                    )
                } else {
                    (state.mock_cfg.remote_players, 0)
                };

                for i in 0..count {
                    let id = 1000 + (i as u32);
                    let name = format!("Mock{}", i + 1);
                    let class = match Self::rng_next_u32(&mut state.rng) % 3 {
                        0 => MirClass::Warrior,
                        1 => MirClass::Wizard,
                        _ => MirClass::Taoist,
                    };
                    let gender = if (Self::rng_next_u32(&mut state.rng) % 2) == 0 {
                        MirGender::Male
                    } else {
                        MirGender::Female
                    };

                    let level = (1 + (Self::rng_next_u32(&mut state.rng) % 22) as u16).max(1);

                    let pos = Self::pick_random_walkable_unoccupied_raw(
                        state.map_width,
                        state.map_height,
                        &state.map_walkable,
                        map_w_eff,
                        map_h_eff,
                        &mut state.rng,
                        &occupied,
                        8192,
                    )
                    .unwrap_or((0, 0));
                    occupied.insert(pos);

                    let weapon = (state.mock_cfg.weapon_min
                        + (Self::rng_next_u32(&mut state.rng)
                            % ((state.mock_cfg.weapon_max - state.mock_cfg.weapon_min).max(0) as u32 + 1))
                            as i16)
                        .clamp(state.mock_cfg.weapon_min, state.mock_cfg.weapon_max);
                    let armour = (state.mock_cfg.armour_min
                        + (Self::rng_next_u32(&mut state.rng)
                            % ((state.mock_cfg.armour_max - state.mock_cfg.armour_min).max(0) as u32 + 1))
                            as i16)
                        .clamp(state.mock_cfg.armour_min, state.mock_cfg.armour_max);
                    let weapon_effect = (state.mock_cfg.weapon_effect_min
                        + (Self::rng_next_u32(&mut state.rng)
                            % ((state.mock_cfg.weapon_effect_max - state.mock_cfg.weapon_effect_min).max(0) as u32 + 1))
                            as i16)
                        .clamp(state.mock_cfg.weapon_effect_min, state.mock_cfg.weapon_effect_max);
                    let wing_effect = if state.mock_cfg.wing_effect_max >= state.mock_cfg.wing_effect_min {
                        let span = (state.mock_cfg.wing_effect_max - state.mock_cfg.wing_effect_min) as u32;
                        state.mock_cfg.wing_effect_min + (Self::rng_next_u32(&mut state.rng) % (span + 1)) as u8
                    } else {
                        0
                    };

                    let mut mount_type = 0i16;
                    let mut riding_mount = false;
                    if level >= state.mock_cfg.mount_min_level && state.mock_cfg.mount_chance > 0.0 {
                        let roll = (Self::rng_next_u32(&mut state.rng) as f32) / (u32::MAX as f32);
                        if roll < state.mock_cfg.mount_chance {
                            let span = (state.mock_cfg.mount_max - state.mock_cfg.mount_min).max(0) as u32;
                            mount_type = state.mock_cfg.mount_min
                                + (Self::rng_next_u32(&mut state.rng) % (span + 1)) as i16;
                            riding_mount = mount_type != 0;
                        }
                    }

                    let mut equip: Vec<Option<UserItem>> = vec![None; 14];
                    // Make a couple of fake items for inspection.
                    if weapon != 0 {
                        let mut info = ItemInfo::default();
                        info.index = weapon as i32;
                        info.item_type = mir2_shared::enums::ItemType::Weapon;
                        info.shape = weapon;
                        info.effect = weapon_effect as u8;
                        info.durability = 1000;
                        info.stack_size = 1;
                        info.name = "MockWeapon".to_string();
                        let mut ui = UserItem::with_info(info);
                        ui.unique_id = 10_000_000 + id as u64;
                        ui.count = 1;
                        ui.current_dura = 1000;
                        equip[0] = Some(ui);
                    }
                    if armour != 0 {
                        let mut info = ItemInfo::default();
                        info.index = armour as i32;
                        info.item_type = mir2_shared::enums::ItemType::Armour;
                        info.shape = armour;
                        // Armour.effect 在客户端被用于 wing_effect（mock 用来表现翅膀/特效）
                        info.effect = wing_effect;
                        info.durability = 1000;
                        info.stack_size = 1;
                        info.name = "MockArmour".to_string();
                        let mut ui = UserItem::with_info(info);
                        ui.unique_id = 20_000_000 + id as u64;
                        ui.count = 1;
                        ui.current_dura = 1000;
                        equip[1] = Some(ui);
                    }

                    let now = Instant::now();
                    let mut rp = MockRemotePlayerState {
                        id,
                        name,
                        class,
                        gender,
                        level,
                        grid: pos,
                        direction: MirDirection::Down,
                        hair: (Self::rng_next_u32(&mut state.rng) % 6) as u8,
                        weapon,
                        weapon_effect,
                        armour,
                        wing_effect,
                        mount_type,
                        riding_mount,
                        equipment: equip,
                        mode: RemoteAiMode::Roam,
                        last_mode_change: now,
                        last_zone_eval: now,
                        goal_zone_idx: 0,
                        zone_idx: 0,
                        last_roam_pick: now,
                        roam_goal: pos,
                        target_monster_id: None,
                        hp_current: 120,
                        hp_max: 120,
                        experience: 0,
                        max_experience: Self::exp_for_next_level(level),
                        last_attack: now - Duration::from_millis(state.mock_cfg.attack_cooldown_ms.max(50)),
                        last_tick: now,
                        team: if state.mock_cfg.mass_battle_enabled {
                            if i < team_a_count { 0 } else { 1 }
                        } else {
                            (i % 2) as u8
                        },
                        dead_until: None,
                    };

                    rp.zone_idx = Self::nearest_zone_idx(&state.zones, rp.grid.0, rp.grid.1);
                    rp.goal_zone_idx = rp.zone_idx;
                    state.remote_players.push(rp);
                }

                for rp in &state.remote_players {
                    Self::send_object_player_update(response_tx, rp);
                }

                // Local player: generate visible equipment + optional mount
                // 注意：本地外观主要由 UserInformation.equipment 的 ItemInfo(shape/effect/type) 导出。
                let mut eq: Vec<Option<UserItem>> = vec![None; 14];
                {
                    let cfg = state.mock_cfg.clone();

                    // 资源约束（与仓库 Data/ 目录对齐，避免随机到不存在的 Lib 导致“武器/坐骑没了”）：
                    // - Data/CWeapon: 00..78
                    // - Data/Mount:   00..11
                    const MAX_CWEAPON_INDEX: i16 = 78;
                    const MAX_MOUNT_INDEX: i16 = 11;

                    let weapon_min_cfg = cfg.weapon_min.clamp(0, MAX_CWEAPON_INDEX);
                    let weapon_max_cfg = cfg.weapon_max.clamp(0, MAX_CWEAPON_INDEX);
                    let armour_min_cfg = cfg.armour_min;
                    let armour_max_cfg = cfg.armour_max;
                    let mount_min_cfg = cfg.mount_min.clamp(0, MAX_MOUNT_INDEX);
                    let mount_max_cfg = cfg.mount_max.clamp(0, MAX_MOUNT_INDEX);

                    // weapon slot = 0
                    let weapon_min_vis = if weapon_max_cfg > 0 {
                        weapon_min_cfg.max(1)
                    } else {
                        weapon_min_cfg
                    };
                    let weapon_max_vis = weapon_max_cfg.max(weapon_min_vis);
                    let w_shape = (weapon_min_vis
                        + (Self::rng_next_u32(&mut state.rng)
                            % ((weapon_max_vis - weapon_min_vis).max(0) as u32 + 1))
                            as i16)
                        .clamp(weapon_min_vis, weapon_max_vis);
                    if w_shape > 0 {
                        let mut info = ItemInfo::default();
                        info.index = 5001;
                        info.item_type = mir2_shared::enums::ItemType::Weapon;
                        info.shape = w_shape;
                        info.effect = (cfg.weapon_effect_min
                            + (Self::rng_next_u32(&mut state.rng)
                                % ((cfg.weapon_effect_max - cfg.weapon_effect_min).max(0) as u32 + 1))
                                as i16)
                            .clamp(cfg.weapon_effect_min, cfg.weapon_effect_max) as u8;
                        info.durability = 1000;
                        info.stack_size = 1;
                        info.name = "MockWeapon".to_string();
                        let mut ui = UserItem::with_info(info);
                        ui.unique_id = 700_000_001;
                        ui.count = 1;
                        ui.current_dura = 1000;
                        eq[0] = Some(ui);
                    }

                    // armour slot = 1
                    let armour_min_vis = if armour_max_cfg > 0 {
                        armour_min_cfg.max(1)
                    } else {
                        armour_min_cfg
                    };
                    let armour_max_vis = armour_max_cfg.max(armour_min_vis);
                    let a_shape = (armour_min_vis
                        + (Self::rng_next_u32(&mut state.rng)
                            % ((armour_max_vis - armour_min_vis).max(0) as u32 + 1))
                            as i16)
                        .clamp(armour_min_vis, armour_max_vis);
                    if a_shape > 0 {
                        let mut info = ItemInfo::default();
                        info.index = 5002;
                        info.item_type = mir2_shared::enums::ItemType::Armour;
                        info.shape = a_shape;
                        // Armour.effect 在客户端被用于 wing_effect（mock 用来表现翅膀/特效）
                        let span = (cfg.wing_effect_max - cfg.wing_effect_min) as u32;
                        info.effect = if cfg.wing_effect_max >= cfg.wing_effect_min {
                            cfg.wing_effect_min + (Self::rng_next_u32(&mut state.rng) % (span + 1)) as u8
                        } else {
                            0
                        };
                        info.durability = 1000;
                        info.stack_size = 1;
                        info.name = "MockArmour".to_string();
                        let mut ui = UserItem::with_info(info);
                        ui.unique_id = 700_000_002;
                        ui.count = 1;
                        ui.current_dura = 1000;
                        eq[1] = Some(ui);
                    }

                    // mount: use MountUpdated (local mount is driven by mount events, not equipment)
                    let mut mount_type = 0i16;
                    let mut riding = false;
                    // 让本地玩家“默认可见坐骑”：
                    // - 如果配置了 MountMin/Max（>0），就直接给坐骑（不受等级限制）
                    // - 否则沿用原本概率逻辑
                    if mount_max_cfg > 0 || mount_min_cfg > 0 {
                        let span = (mount_max_cfg - mount_min_cfg).max(0) as u32;
                        mount_type = mount_min_cfg + (Self::rng_next_u32(&mut state.rng) % (span + 1)) as i16;
                        if mount_type == 0 && mount_max_cfg > 0 {
                            mount_type = mount_max_cfg;
                        }
                        riding = mount_type != 0;
                    } else if state.player_level >= cfg.mount_min_level && cfg.mount_chance > 0.0 {
                        let roll = (Self::rng_next_u32(&mut state.rng) as f32) / (u32::MAX as f32);
                        if roll < cfg.mount_chance {
                            let span = (mount_max_cfg - mount_min_cfg).max(0) as u32;
                            mount_type = mount_min_cfg + (Self::rng_next_u32(&mut state.rng) % (span + 1)) as i16;
                            riding = mount_type != 0;
                        }
                    }
                    state.player_mount_type = mount_type;
                    state.player_riding_mount = riding;
                }

                state.player_equipment = eq.clone();

                // 默认给本地玩家一些红药，方便验证“自动喝药”
                if state.player_inventory.iter().all(|x| x.is_none()) {
                    let mut mk_potion = |slot: usize, uid: u64, name: &str| {
                        if slot >= state.player_inventory.len() {
                            return;
                        }
                        let mut info = ItemInfo::default();
                        info.index = 20_000 + slot as i32;
                        info.name = name.to_string();
                        info.item_type = ItemType::Potion;
                        info.stack_size = 1;
                        let mut ui = UserItem::with_info(info);
                        ui.unique_id = uid;
                        ui.count = 1;
                        state.player_inventory[slot] = Some(ui);
                    };
                    mk_potion(0, 880_000_001, "强效金创药");
                    mk_potion(1, 880_000_002, "金创药(中)");
                    mk_potion(2, 880_000_003, "太阳水");
                }

                // Local user info
                let user_packet = mir2_shared::packets::server::UserInformation {
                    object_id: state.player_object_id,
                    real_id: state.player_object_id,
                    name: state.active_character_name.clone(),
                    guild_name: String::new(),
                    guild_rank: String::new(),
                    name_colour: 0,
                    class: MirClass::Warrior,
                    gender: MirGender::Male,
                    level: state.player_level,
                    location_x: state.player_grid.0,
                    location_y: state.player_grid.1,
                    direction: MirDirection::Down,
                    // 头发/头盔：Data/CHair 有 00..08（共9种），随机一个炫酷的
                    hair: (Self::rng_next_u32(&mut state.rng) % 9) as u8,
                    hp: state.player_hp_current,
                    mp: 0,
                    experience: state.player_experience,
                    max_experience: state.player_max_experience.max(1),
                    level_effects: mir2_shared::enums::LevelEffects::empty(),
                    has_hero: false,
                    hero_behaviour: HeroBehaviour::Attack,
                    inventory: Some(state.player_inventory.clone()),
                    equipment: Some(state.player_equipment.clone()),
                    quest_inventory: None,
                    gold: state.player_gold,
                    credit: 0,
                    has_expanded_storage: false,
                    expanded_storage_expiry_time: 0,
                    magics: Vec::new(),
                    summoned_creature_type: 0,
                    creature_summoned: false,
                    allow_observe: false,
                    observer: false,
                };
                let _ = response_tx.send(NetworkEvent::UserInformation { packet: user_packet });

                // IMPORTANT: send local MountUpdated AFTER UserInformation.
                // Otherwise the client may drop the mount update because the LocalPlayer entity
                // (with PlayerData.id) doesn't exist yet.
                if state.player_riding_mount {
                    let _ = response_tx.send(NetworkEvent::MountUpdated {
                        object_id: state.player_object_id,
                        mount_type: state.player_mount_type,
                        riding_mount: true,
                    });
                }
                let _ = response_tx.send(NetworkEvent::HealthChanged {
                    current: state.player_hp_current as u32,
                    max: state.player_hp_max as u32,
                });
                let _ = response_tx.send(NetworkEvent::PlayerLocationChanged {
                    x: state.player_grid.0,
                    y: state.player_grid.1,
                });
                let _ = response_tx.send(NetworkEvent::SystemMessage {
                    message: "(MOCK) Entered game".to_string(),
                });

                // Make the local visual state obvious in logs/chat so we can confirm
                // we're running the latest build and that equipment/mount data is non-empty.
                let weapon_dbg = state
                    .player_equipment
                    .get(0)
                    .and_then(|o| o.as_ref())
                    .and_then(|ui| ui.info.as_ref())
                    .map(|info| (info.shape, info.effect))
                    .unwrap_or((0, 0));
                let armour_dbg = state
                    .player_equipment
                    .get(1)
                    .and_then(|o| o.as_ref())
                    .and_then(|ui| ui.info.as_ref())
                    .map(|info| (info.shape, info.effect))
                    .unwrap_or((0, 0));
                let _ = response_tx.send(NetworkEvent::SystemMessage {
                    message: format!(
                        "(MOCK) Local visuals: weapon(shape={},effect={}) armour(shape={},effect={}) mount_type={} riding={}",
                        weapon_dbg.0,
                        weapon_dbg.1,
                        armour_dbg.0,
                        armour_dbg.1,
                        state.player_mount_type,
                        state.player_riding_mount
                    ),
                });
            }

            NetworkEvent::WalkRequest { direction }
            | NetworkEvent::RunRequest { direction }
            | NetworkEvent::MoveRequest { direction } => {
                if !state.in_game {
                    return;
                }
                if state.last_player_move_req.elapsed() < Duration::from_millis(40) {
                    return;
                }
                state.last_player_move_req = Instant::now();

                let (dx, dy) = dir_to_delta(direction);
                let nx = state.player_grid.0 + dx;
                let ny = state.player_grid.1 + dy;
                if Self::map_is_walkable(state, nx, ny) {
                    state.player_grid = (nx, ny);
                    let _ = response_tx.send(NetworkEvent::PlayerLocationChanged { x: nx, y: ny });
                }
            }

            NetworkEvent::TurnRequest { .. } => {}

            NetworkEvent::ChatRequest { message, .. } => {
                let _ = response_tx.send(NetworkEvent::ChatMessage {
                    sender: "(MOCK)".to_string(),
                    message: format!("Echo: {}", message),
                    chat_type: ChatType::Normal,
                });
            }

            NetworkEvent::InspectRequest { object_id } => {
                if let Some(rp) = state.remote_players.iter().find(|p| p.id == object_id) {
                    let packet = mir2_shared::packets::server::PlayerInspect {
                        name: rp.name.clone(),
                        guild_name: String::new(),
                        guild_rank: String::new(),
                        equipment: rp.equipment.clone(),
                        class: rp.class,
                        gender: rp.gender,
                        hair: rp.hair,
                        level: rp.level,
                        lover_name: String::new(),
                    };
                    let _ = response_tx.send(NetworkEvent::PlayerInspect { packet });
                }
            }

            NetworkEvent::AttackRequest { direction, .. } => {
                if !state.in_game {
                    return;
                }

                let (dx, dy) = dir_to_delta(direction);
                let tx = state.player_grid.0 + dx;
                let ty = state.player_grid.1 + dy;

                // find monster on that tile
                let mut target: Option<u32> = None;
                for (mid, m) in &state.monsters {
                    if m.hp > 0 && m.pos == (tx, ty) {
                        target = Some(*mid);
                        break;
                    }
                }

                let _ = response_tx.send(NetworkEvent::ObjectAttack {
                    packet: mir2_shared::packets::server::ObjectAttack {
                        object_id: state.player_object_id,
                        location_x: state.player_grid.0.max(0) as u32,
                        location_y: state.player_grid.1.max(0) as u32,
                        direction: direction as u8,
                        spell: 0,
                        level: 0,
                        attack_type: 0,
                    },
                });

                let Some(mid) = target else {
                    return;
                };

                let damage = 12 + (Self::rng_next_u32(&mut state.rng) % 10) as i32;
                if let Some(mm) = state.monsters.get_mut(&mid) {
                    mm.hp -= damage;
                }
                let _ = response_tx.send(NetworkEvent::ObjectStruck {
                    object_id: mid,
                    attacker_id: state.player_object_id,
                    damage,
                });

                let dead = state.monsters.get(&mid).map(|m| m.hp <= 0).unwrap_or(false);
                if dead {
                    let xp = state.monsters.get(&mid).map(|m| m.xp_reward).unwrap_or(10);
                    state.monsters.remove(&mid);
                    let _ = response_tx.send(NetworkEvent::ObjectDied { object_id: mid });
                    let _ = response_tx.send(NetworkEvent::ObjectRemove {
                        packet: mir2_shared::packets::server::ObjectRemove { object_id: mid },
                    });

                    state.player_experience += xp;
                    let _ = response_tx.send(NetworkEvent::ExperienceGained { amount: xp });

                    while state.player_experience >= state.player_max_experience {
                        state.player_experience -= state.player_max_experience.max(1);
                        state.player_level = state.player_level.saturating_add(1);
                        state.player_max_experience = Self::exp_for_next_level(state.player_level).max(1);
                        let _ = response_tx.send(NetworkEvent::LevelUp {
                            new_level: state.player_level,
                        });
                    }
                }
            }

            NetworkEvent::NPCCallRequest { npc_object_id, key } => {
                // very small mock shop
                if key.is_empty() {
                    let dialog = "<font color=yellow>Mock Shop</font>\\n[@Buy]".to_string();
                    let _ = response_tx.send(NetworkEvent::NpcDialog { npc_id: npc_object_id, dialog });

                    let mut goods: Vec<UserItem> = Vec::new();
                    for i in 0..6u64 {
                        let mut info = ItemInfo::default();
                        info.index = 1000 + i as i32;
                        info.name = format!("MockItem{}", i + 1);
                        info.price = 50 + (i as u32) * 10;
                        info.stack_size = 99;
                        let mut item = UserItem::with_info(info);
                        item.unique_id = 900_000 + i;
                        item.is_shop_item = true;
                        goods.push(item);
                    }
                    state.last_shop_goods = goods.clone();
                    let _ = response_tx.send(NetworkEvent::NPCGoods {
                        items: goods,
                        rate: 1.0,
                        panel_type: PanelType::Buy,
                        hide_added_stats: false,
                    });
                }
            }

            NetworkEvent::BuyItemRequest {
                item_index,
                count,
                panel_type,
            } => {
                if count == 0 {
                    return;
                }
                let Some(template) = state.last_shop_goods.iter().find(|g| g.unique_id == item_index).cloned() else {
                    let _ = response_tx.send(NetworkEvent::SystemMessage {
                        message: format!("(MOCK) 购买失败：找不到商品 unique_id={}", item_index),
                    });
                    return;
                };

                let unit_price = template.info.as_ref().map(|x| x.price).unwrap_or(0);
                let total_cost_u64 = (unit_price as u64).saturating_mul(count as u64);
                let total_cost = total_cost_u64.min(u32::MAX as u64) as u32;

                if state.player_gold < total_cost {
                    let _ = response_tx.send(NetworkEvent::SystemMessage {
                        message: format!("(MOCK) 金币不足：需要 {}，当前 {}", total_cost, state.player_gold),
                    });
                    return;
                }
                state.player_gold -= total_cost;
                let _ = response_tx.send(NetworkEvent::GoldChanged {
                    delta: -(total_cost as i32),
                });

                let mut purchased = template.clone();
                purchased.count = (count.min(u16::MAX as u32)) as u16;
                purchased.unique_id = 1_000_000_000 + (Self::rng_next_u32(&mut state.rng) as u64);

                // 写入 mock 服务器背包（便于后续 UseItemRequest 找到它）
                if let Some(slot) = state.player_inventory.iter_mut().find(|s| s.is_none()) {
                    *slot = Some(purchased.clone());
                }

                let _ = response_tx.send(NetworkEvent::ItemGained { item: purchased });
                let _ = response_tx.send(NetworkEvent::SystemMessage {
                    message: format!(
                        "(MOCK) 购买成功：unique_id={} x{} 花费={} (panel_type={})",
                        item_index, count, total_cost, panel_type
                    ),
                });
            }

            NetworkEvent::UseItemRequest { unique_id } => {
                // 在 mock 世界里：支持最小闭环（喝红药回血 + 消耗物品）
                let Some(slot_idx) = state
                    .player_inventory
                    .iter()
                    .position(|x| x.as_ref().map(|it| it.unique_id) == Some(unique_id))
                else {
                    return;
                };

                let item = state.player_inventory[slot_idx].take();
                let Some(item) = item else {
                    return;
                };

                let is_potion = item
                    .info
                    .as_ref()
                    .map(|info| info.item_type == ItemType::Potion)
                    .unwrap_or(false);

                if is_potion && state.player_hp_current > 0 {
                    let heal = (state.player_hp_max / 3).max(20);
                    state.player_hp_current = (state.player_hp_current + heal).min(state.player_hp_max);

                    let _ = response_tx.send(NetworkEvent::HealthChanged {
                        current: state.player_hp_current as u32,
                        max: state.player_hp_max as u32,
                    });
                }

                // 物品消耗：直接移除（count=1 的简化模型）
                let _ = response_tx.send(NetworkEvent::ItemLost { unique_id });
            }

            _ => {
                // 未处理的事件：忽略即可
            }
        }
    }
}

fn dir_to_delta(dir: MirDirection) -> (i32, i32) {
    match dir {
        MirDirection::Up => (0, -1),
        MirDirection::UpRight => (1, -1),
        MirDirection::Right => (1, 0),
        MirDirection::DownRight => (1, 1),
        MirDirection::Down => (0, 1),
        MirDirection::DownLeft => (-1, 1),
        MirDirection::Left => (-1, 0),
        MirDirection::UpLeft => (-1, -1),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_mock_network_connection() {
        let (tx, rx) = MockNetwork::new();

        thread::sleep(Duration::from_millis(200));

        let events: Vec<_> = rx.try_iter().collect();
        assert!(events.iter().any(|e| matches!(e, NetworkEvent::Connected)));

        tx.send(NetworkEvent::DisconnectRequest).unwrap();
        thread::sleep(Duration::from_millis(200));

        let events: Vec<_> = rx.try_iter().collect();
        assert!(events.iter().any(|e| matches!(e, NetworkEvent::Disconnected { .. })));
    }

    #[test]
    fn test_mock_network_login() {
        let (tx, rx) = MockNetwork::new();

        tx.send(NetworkEvent::LoginRequest {
            username: "test_user".to_string(),
            password: "test_pass".to_string(),
        })
        .unwrap();

        thread::sleep(Duration::from_millis(300));

        let events: Vec<_> = rx.try_iter().collect();
        assert!(events.iter().any(|e| matches!(e, NetworkEvent::LoginSuccess { .. })));
    }
}
