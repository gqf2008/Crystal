// ============================================================================
// Mock Network - 模拟网络实现（用于开发工具和离线测试）
// ============================================================================
//
// 提供完全本地的网络模拟，无需真实服务器：
// - 模拟连接/断开
// - 模拟角色数据
// - 模拟地图数据
// - 模拟基本的游戏事件响应
//
// 使用方式：
//   let net_ctx = NetworkBuilder::new(settings)
//       .mock(true)
//       .build()?;
//
// ============================================================================

use super::handlers::NetworkEvent;
use crate::resources::MapReader;
use crossbeam_channel::{Receiver, Sender};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::thread;
use std::time::{Duration, Instant};
use std::collections::{HashMap, HashSet};

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
    center: (i32, i32),
    radius: i32,
    max_monsters: usize,
    respawn_interval: Duration,
    monster_image: u16,
    monster_hp: i32,
    xp_reward: i64,
    last_spawn: Instant,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum RemoteAiMode {
    Roam,
    Seek,
    Travel,
    Chase,
    Fight,
    Rest,
}

#[derive(Debug, Clone)]
struct MockRemotePlayerState {
    id: u32,
    name: String,
    class: mir2_shared::enums::MirClass,
    gender: mir2_shared::enums::MirGender,
    hair: u8,
    weapon: i16,
    weapon_effect: i16,
    armour: i16,
    wing_effect: u8,
    grid: (i32, i32),
    direction: mir2_shared::enums::MirDirection,
    level: u16,
    experience: i64,
    max_experience: i64,
    zone_idx: usize,
    goal_zone_idx: usize,
    mode: RemoteAiMode,
    target_monster_id: Option<u32>,
    roam_goal: (i32, i32),
    last_tick: Instant,
    last_attack: Instant,
    last_mode_change: Instant,
    last_zone_eval: Instant,
    last_roam_pick: Instant,
}

#[derive(Debug, Clone)]
struct MockWorldState {
    in_game: bool,

    // 用于在“持续有客户端事件”时也能推进 mock 世界（否则远程 AI/刷怪可能完全不跑）
    last_world_tick: Instant,

    // Mock 地图碰撞（用于服务器权威移动校验，避免把玩家“纠正/瞬移”到障碍物里）
    map_width: i32,
    map_height: i32,
    // 扁平数组：len = map_width * map_height，1=可走 0=不可走；为空表示未知（全部视为可走）
    map_walkable: Vec<u8>,

    // 本地玩家（server-authoritative）
    player_object_id: u32,
    player_gold: u32,
    inventory_capacity: usize,
    player_grid: (i32, i32),
    player_spawn_grid: (i32, i32),
    player_hp_current: i32,
    player_hp_max: i32,
    player_dead_since: Option<Instant>,
    // 复活/刚进入游戏的短暂无敌：避免出生点附近刷怪导致“无法操作/一直被拉回城”。
    player_protected_until: Option<Instant>,

    // NPC 商店：最近一次下发给客户端的货单（用于 BuyItemRequest 通过 unique_id 反查）
    last_shop_goods: Vec<mir2_shared::data::item::UserItem>,

    last_player_move_req: Instant,

    // server-authoritative monsters (position + HP)
    monsters: HashMap<u32, MockMonsterState>,

    // multi-zone spawn + roaming
    zones: Vec<MockZone>,
    next_monster_id: u32,
    last_monster_wander_tick: Instant,
    last_monster_combat_tick: Instant,

    // server-authoritative remote players (AI-driven)
    remote_players: Vec<MockRemotePlayerState>,

    // deterministic RNG (no external crate)
    rng: u64,
}

impl Default for MockWorldState {
    fn default() -> Self {
        let now = Instant::now();

        // Mock 默认地图边界（n0.map 实测约 700x700）。
        // 仅用于让 3000 远程玩家“分散到地图各处”时不至于跑出可视范围太远。
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

        let clamp_x = |x: i32| x.clamp(0, map_w.saturating_sub(1).max(0));
        let clamp_y = |y: i32| y.clamp(0, map_h.saturating_sub(1).max(0));

        // 多区域刷怪：覆盖全图，避免 3000 远程玩家挤在出生点附近。
        // 用一个规则网格生成多个 zone，AI 会在 zone 之间 Seek/Travel。
        let mut zones: Vec<MockZone> = Vec::new();
        let grid_cols = 5;
        let grid_rows = 5;
        let margin = 80;
        let step_x = ((map_w - margin * 2).max(1)) / (grid_cols.max(1) - 1);
        let step_y = ((map_h - margin * 2).max(1)) / (grid_rows.max(1) - 1);

        for iy in 0..grid_rows {
            for ix in 0..grid_cols {
                let cx = clamp_x(margin + ix * step_x);
                let cy = clamp_y(margin + iy * step_y);

                let idx = (iy * grid_cols + ix) as i32;
                let monster_image = (idx % 6).max(0) as u16;
                let monster_hp = 24 + ((idx % 5) * 6);
                let xp_reward = 10 + ((idx % 7) * 2) as i64;
                let respawn_ms = 1200 + ((idx % 6) * 250) as u64;

                zones.push(MockZone {
                    name: "Field",
                    center: (cx, cy),
                    radius: 20,
                    max_monsters: 18,
                    respawn_interval: Duration::from_millis(respawn_ms),
                    monster_image,
                    monster_hp,
                    xp_reward,
                    last_spawn: now,
                });
            }
        }

        Self {
            in_game: false,

            last_world_tick: now,

            map_width: 0,
            map_height: 0,
            map_walkable: Vec::new(),

            player_object_id: 1,
            player_gold: 1000,
            inventory_capacity: 40,
            // 330,330 在 n0.map 上容易被前景遮挡；换到更空旷的位置，避免一直只能看到 ghost。
            player_grid: (336, 334),
            player_spawn_grid: (336, 334),
            player_hp_current: 100,
            player_hp_max: 100,
            player_dead_since: None,
            player_protected_until: None,
            last_shop_goods: Vec::new(),

            last_player_move_req: Instant::now(),

            monsters: HashMap::new(),

            zones,
            next_monster_id: 3001,
            last_monster_wander_tick: Instant::now(),
            last_monster_combat_tick: Instant::now(),

            remote_players: Vec::new(),

            rng: 0xC0FFEE_u64,
        }
    }
}

/// 模拟网络实现
pub struct MockNetwork {
    /// 线程是否运行
    running: Arc<AtomicBool>,
    /// 接收游戏层发送的事件
    #[allow(dead_code)]
    game_tx: Sender<NetworkEvent>,
    /// 游戏层接收事件的通道
    #[allow(dead_code)]
    game_rx: Receiver<NetworkEvent>,
    /// 模拟网络线程句柄
    _handle: Option<thread::JoinHandle<()>>,
}

impl MockNetwork {
    /// 创建新的模拟网络
    ///
    /// # 返回
    /// (发送通道, 接收通道) - 供 NetContext 使用
    pub fn new() -> (Sender<NetworkEvent>, Receiver<NetworkEvent>) {
        let (client_tx, mock_rx) = crossbeam_channel::unbounded();
        let (mock_tx, client_rx) = crossbeam_channel::unbounded();

        let running = Arc::new(AtomicBool::new(true));
        let running_clone = running.clone();

        // 启动模拟网络线程
        let handle = thread::spawn(move || {
            // 备注：项目里不一定初始化了 tracing subscriber；为方便离线验收，关键点用 println 兜底输出。
            println!("🌐 MockNetwork 启动");
            tracing::info!("🌐 MockNetwork 启动");

            // 立即发送连接成功事件
            let _ = mock_tx.send(NetworkEvent::Connected);

            let mut state = MockWorldState::default();

            while running_clone.load(Ordering::Relaxed) {
                match mock_rx.recv_timeout(Duration::from_millis(100)) {
                    Ok(event) => {
                        Self::handle_game_event(event, &mock_tx, &mut state);

                        // 关键：如果客户端持续发包（例如每帧输入/心跳），recv_timeout 永远不会 Timeout，
                        // 那 tick_world 就不会被调用，远程玩家/刷怪/怪物 AI 都会“停摆”。
                        if state.in_game && state.last_world_tick.elapsed() >= Duration::from_millis(80) {
                            state.last_world_tick = Instant::now();
                            Self::tick_world(&mock_tx, &mut state);
                        }
                    }
                    Err(crossbeam_channel::RecvTimeoutError::Timeout) => {
                        // 正常超时：让 mock 世界在无输入时也能推进（server-driven）
                        Self::tick_world(&mock_tx, &mut state);
                    }
                    Err(crossbeam_channel::RecvTimeoutError::Disconnected) => {
                        println!("🔌 [MOCK] 客户端断开连接");
                        tracing::info!("🔌 客户端断开连接");
                        break;
                    }
                }
            }

            println!("🛑 MockNetwork 关闭");
            tracing::info!("🛑 MockNetwork 关闭");
        });

        // 将 MockNetwork 实例泄漏到静态生命周期，防止被Drop
        // 这样线程可以一直运行到程序结束
        let mock = MockNetwork {
            running,
            game_tx: client_tx.clone(),
            game_rx: client_rx.clone(),
            _handle: Some(handle),
        };

        // 使用 Box::leak 防止 Drop
        let _ = Box::leak(Box::new(mock));

        // 返回通道供 NetContext 使用
        (client_tx, client_rx)
    }

    /// 处理游戏层发送的事件
    fn handle_game_event(event: NetworkEvent, response_tx: &Sender<NetworkEvent>, state: &mut MockWorldState) {
        tracing::debug!("📥 MockNetwork 收到事件: {:?}", event);

        match event {
            // 客户端版本校验
            NetworkEvent::ClientVersionSend { .. } => {
                let _ = response_tx.send(NetworkEvent::ClientVersionResponse { result: 1 });
            }

            // 心跳
            NetworkEvent::KeepAliveSend { time } => {
                let _ = response_tx.send(NetworkEvent::KeepAliveReceived { time });
            }

            // 断开请求
            NetworkEvent::DisconnectRequest => {
                tracing::info!("👋 模拟断开连接");
                let _ = response_tx.send(NetworkEvent::Disconnected {
                    reason: "User requested".to_string(),
                });
            }

            // 登录请求
            NetworkEvent::LoginRequest { username, .. } => {
                tracing::info!("🔐 模拟登录: {}", username);
                // 延迟一点模拟网络延迟
                thread::sleep(Duration::from_millis(100));
                // 返回空角色列表
                let _ = response_tx.send(NetworkEvent::LoginSuccess { characters: vec![] });
            }

            // 新建账号请求
            NetworkEvent::NewAccountRequest { account_id, .. } => {
                tracing::info!("📝 模拟创建账号: {}", account_id);
                thread::sleep(Duration::from_millis(100));
                let _ = response_tx.send(NetworkEvent::NewAccountSuccess);
            }

            // 创建角色请求
            NetworkEvent::NewCharacterRequest { name, .. } => {
                tracing::info!("🧙 模拟创建角色: {}", name);
                thread::sleep(Duration::from_millis(100));
                let _ = response_tx.send(NetworkEvent::CharacterCreated { name: name.clone() });
            }

            // 删除角色请求
            NetworkEvent::DeleteCharacterRequest { index } => {
                tracing::info!("🗑️ 模拟删除角色: {}", index);
                thread::sleep(Duration::from_millis(100));
                let _ = response_tx.send(NetworkEvent::CharacterDeleted {
                    index: index as u32,
                });
            }

            // 开始游戏请求
            NetworkEvent::StartGameRequest { character_index } => {
                println!("🎮 [MOCK] StartGameRequest character_index={}", character_index);
                tracing::info!("🎮 模拟开始游戏: 角色索引 {}", character_index);
                thread::sleep(Duration::from_millis(200));

                // 发送开始游戏响应
                let _ = response_tx.send(NetworkEvent::StartGameDelay {
                    packet: mir2_shared::packets::server::StartGameDelay {
                        milliseconds: 500,
                    },
                });

                // 按 C# 协议：StartGame 带 Resolution；这里模拟成功
                let _ = response_tx.send(NetworkEvent::StartGame {
                    packet: mir2_shared::packets::server::StartGame {
                        result: 4,
                        resolution: 1024,
                    },
                });

                // 加载地图并发送 MapChanged 事件（落点要和 UserInformation 一致，否则相机会被拉到(0,0)）
                let (spawn_x, spawn_y) = Self::load_and_send_map(
                    &response_tx,
                    state,
                    "Map/n0.map",
                    0,
                    "盟重土城",
                    336,
                    334,
                    mir2_shared::enums::MirDirection::Down as u8,
                );

                // 模拟玩家信息
                // 关键：下发初始背包（None = 不下发，会导致后续 ItemGained 没 UI 承载）
                state.player_gold = 1000;
                state.inventory_capacity = 40;
                state.player_grid = (spawn_x, spawn_y);
                state.player_object_id = 1;
                state.player_hp_max = 100;
                state.player_hp_current = 100;
                state.player_spawn_grid = state.player_grid;
                state.player_dead_since = None;
                state.player_protected_until = Some(Instant::now() + Duration::from_millis(3500));

                // 下发最小装备：用于验证“装备→外观/坐骑派生→渲染”链路。
                // 槽位约定（见 NetworkApplySystem::apply_equipment_vec）:
                // 0 weapon, 1 armour, ... 13 mount
                let mut equipment: Vec<Option<mir2_shared::data::item::UserItem>> = vec![None; 14];

                // 武器：CWeapon/{:02}
                {
                    use mir2_shared::data::item::{ItemInfo, UserItem};
                    use mir2_shared::enums::ItemType;
                    let mut info = ItemInfo::default();
                    info.index = 10001;
                    info.name = "Mock Weapon".to_string();
                    info.item_type = ItemType::Weapon;
                    // ClientRust “神豪玩家”同款：weapon=78
                    info.shape = 78;
                    info.durability = 10;
                    // 武器特效索引（对应 Data/CWeaponEffect/{:02}.Lib）
                    // ClientRust “神豪玩家”同款：weapon_fx=66
                    info.effect = 66;

                    let mut item = UserItem::with_info(info);
                    item.unique_id = 10001;
                    item.current_dura = 10;
                    item.max_dura = 10;
                    equipment[0] = Some(item);
                }

                // 衣服：CArmour/{:02}，用于驱动人物外观；同时用 effect 驱动翅膀/人物特效（CHumEffect/{:02}）。
                {
                    use mir2_shared::data::item::{ItemInfo, UserItem};
                    use mir2_shared::enums::ItemType;
                    let mut info = ItemInfo::default();
                    info.index = 10002;
                    info.name = "Mock Armour".to_string();
                    info.item_type = ItemType::Armour;
                    // ClientRust “神豪玩家”同款：armour=58
                    info.shape = 58;
                    info.durability = 10;
                    // 翅膀/人物特效：本项目用 CHumEffect/{:02}.Lib
                    // ClientRust “神豪玩家”同款：wing=5
                    info.effect = 5;

                    let mut item = UserItem::with_info(info);
                    item.unique_id = 10002;
                    item.current_dura = 10;
                    item.max_dura = 10;
                    equipment[1] = Some(item);
                }

                // 坐骑：Mount/{:02}
                {
                    use mir2_shared::data::item::{ItemInfo, UserItem};
                    use mir2_shared::enums::ItemType;
                    let mut info = ItemInfo::default();
                    info.index = 10013;
                    info.name = "Mock Mount".to_string();
                    info.item_type = ItemType::Mount;
                    // ClientRust “神豪玩家”同款：mount=11
                    info.shape = 11;
                    info.durability = 10;
                    info.effect = 0;

                    let mut item = UserItem::with_info(info);
                    item.unique_id = 10013;
                    item.current_dura = 10;
                    item.max_dura = 10;
                    equipment[13] = Some(item);
                }

                let _ = response_tx.send(NetworkEvent::UserInformation {
                    packet: mir2_shared::packets::server::UserInformation {
                        object_id: state.player_object_id,
                        real_id: state.player_object_id,
                        name: "TestUser".to_string(),
                        guild_name: "".to_string(),
                        guild_rank: "".to_string(),
                        name_colour: 0,
                        class: mir2_shared::enums::MirClass::Warrior,
                        gender: mir2_shared::enums::MirGender::Male,
                        level: 1,
                        location_x: state.player_grid.0,
                        location_y: state.player_grid.1,
                        direction: mir2_shared::enums::MirDirection::Down,
                        hair: 1,
                        hp: state.player_hp_current.max(0),
                        mp: 50,
                        experience: 0,
                        max_experience: 0,
                        level_effects: mir2_shared::enums::LevelEffects::empty(),
                        has_hero: false,
                        hero_behaviour: mir2_shared::enums::HeroBehaviour::Follow,
                        inventory: Some(vec![None; state.inventory_capacity]),
                        equipment: Some(equipment),
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
                    },
                });

                // ====== Mock：生成多个“远程玩家”（不同区域找怪→跑路→打怪升级） ======
                state.remote_players.clear();
                let now = Instant::now();
                let remote_count: usize = std::env::var("CRYSTAL_REMOTE_PLAYERS")
                    .ok()
                    .and_then(|v| v.parse::<usize>().ok())
                    .filter(|v| *v > 0)
                    .unwrap_or(3000);

                let class = mir2_shared::enums::MirClass::Warrior;
                let weapon: i16 = 60;
                let weapon_effect: i16 = 12;
                let armour: i16 = 25;
                let wing_effect: u8 = 4;

                for i in 0..remote_count {
                    let id = 2_u32.saturating_add(i as u32);

                    let zone_idx = if state.zones.is_empty() {
                        0
                    } else {
                        (Self::rng_next_u32(&mut state.rng) as usize) % state.zones.len()
                    };
                    let (x, y) = state
                        .zones
                        .get(zone_idx)
                        .map(|z| Self::random_pos_in_zone(&mut state.rng, z))
                        .unwrap_or((338, 334));

                    let level = 1_u16
                        .saturating_add((Self::rng_next_u32(&mut state.rng) % 4) as u16);
                    let gender = if (Self::rng_next_u32(&mut state.rng) % 2) == 0 {
                        mir2_shared::enums::MirGender::Male
                    } else {
                        mir2_shared::enums::MirGender::Female
                    };
                    let hair = ((Self::rng_next_u32(&mut state.rng) % 6) as u8).max(1);
                    let direction = mir2_shared::enums::MirDirection::Left;

                    let name = format!("Remote{}", id);

                    let _ = response_tx.send(NetworkEvent::ObjectPlayer {
                        packet: mir2_shared::packets::server::ObjectPlayer {
                            object_id: id,
                            name: name.clone(),
                            guild_name: "".to_string(),
                            guild_rank_name: "".to_string(),
                            name_colour: 0,
                            class,
                            gender,
                            level,
                            location_x: x,
                            location_y: y,
                            direction,
                            hair,
                            light: 0,
                            // 为了保证离线资源命中，所有远程玩家都用已验证存在的外观索引。
                            weapon,
                            weapon_effect,
                            armour,
                            poison: mir2_shared::enums::PoisonType::empty(),
                            dead: false,
                            hidden: false,
                            effect: mir2_shared::enums::SpellEffect::None,
                            wing_effect,
                            extra: false,
                            mount_type: 0,
                            riding_mount: false,
                            fishing: false,
                            transform_type: 0,
                            element_orb_effect: 0,
                            element_orb_lvl: 0,
                            element_orb_max: 0,
                            buffs: Vec::new(),
                            level_effects: mir2_shared::enums::LevelEffects::empty(),
                        },
                    });

                    let max_experience = Self::exp_for_next_level(level);
                    let roam_goal = match state.zones.get(zone_idx) {
                        Some(z) => z.center,
                        None => (x, y),
                    };
                    state.remote_players.push(MockRemotePlayerState {
                        id,
                        name,
                        class,
                        gender,
                        hair,
                        weapon,
                        weapon_effect,
                        armour,
                        wing_effect,
                        grid: (x, y),
                        direction,
                        level,
                        experience: 0,
                        max_experience,
                        zone_idx,
                        goal_zone_idx: zone_idx,
                        mode: RemoteAiMode::Seek,
                        target_monster_id: None,
                        roam_goal,
                        last_tick: now,
                        last_attack: now,
                        last_mode_change: now,
                        last_zone_eval: now,
                        last_roam_pick: now,
                    });
                }

                // ====== Mock(权威服务器)：用真实 server packet 形状生成 NPC/怪物 ======
                // 坐标为格子坐标（与 UserInformation/MapChanged 一致）
                let _ = response_tx.send(NetworkEvent::ObjectNpc {
                    packet: mir2_shared::packets::server::ObjectNpc {
                        object_id: 2001,
                        name: "TestNPC".to_string(),
                        name_colour: 0,
                        image: 0,
                        colour: 0,
                        // 该地图部分点位会被前景树遮挡：这里放到玩家出生点附近更空旷处，便于测试交互。
                        // 向右移动约 300px：1 格=48px，所以 300px≈+6 格（288px）
                        location_x: 336,
                        location_y: 334,
                        direction: mir2_shared::enums::MirDirection::Down,
                    },
                });

                // ====== Mock：多区域刷怪（server-authoritative） ======
                state.in_game = true;
                state.monsters.clear();
                state.next_monster_id = 3001;
                state.last_monster_wander_tick = Instant::now();
                state.last_monster_combat_tick = Instant::now();

                // 初始填充：每个区域先刷 2 只，便于立即看到“找怪→跑路→打怪”
                for zone_idx in 0..state.zones.len() {
                    for _ in 0..2 {
                        Self::spawn_monster_in_zone(response_tx, state, zone_idx);
                    }
                }
            }

            // ===== 本地玩家移动（服务器权威） =====
            NetworkEvent::TurnRequest { direction } => {
                // 真服一般会回 ObjectTurn / UserLocation；此处最小只刷新位置（不带方向），方向由客户端本地表现维护。
                state.last_player_move_req = Instant::now();
                let _ = direction; // 避免未使用告警（后续若加方向同步可用）
            }
            NetworkEvent::MoveRequest { direction }
            | NetworkEvent::WalkRequest { direction }
            | NetworkEvent::RunRequest { direction } => {
                state.last_player_move_req = Instant::now();

                let (x, y) = state.player_grid;
                let (dx, dy) = match direction {
                    mir2_shared::enums::MirDirection::Up => (0, -1),
                    mir2_shared::enums::MirDirection::UpRight => (1, -1),
                    mir2_shared::enums::MirDirection::Right => (1, 0),
                    mir2_shared::enums::MirDirection::DownRight => (1, 1),
                    mir2_shared::enums::MirDirection::Down => (0, 1),
                    mir2_shared::enums::MirDirection::DownLeft => (-1, 1),
                    mir2_shared::enums::MirDirection::Left => (-1, 0),
                    mir2_shared::enums::MirDirection::UpLeft => (-1, -1),
                };

                // 一次请求推进一格（最简单的真服式“离散移动”模拟）
                let nx = x + dx;
                let ny = y + dy;
                if Self::map_is_walkable(state, nx, ny) {
                    state.player_grid = (nx, ny);
                    let _ = response_tx.send(NetworkEvent::PlayerLocationChanged { x: nx, y: ny });
                } else {
                    // 被障碍物/边界挡住：不移动，同时回一个当前位置用于纠偏/停跑
                    let (cx, cy) = state.player_grid;
                    let _ = response_tx.send(NetworkEvent::PlayerLocationChanged { x: cx, y: cy });
                }
            }

            // ===== 本地玩家攻击（服务器权威） =====
            NetworkEvent::AttackRequest { direction, .. } => {
                if !state.in_game {
                    return;
                }

                let (x, y) = state.player_grid;
                let (dx, dy) = match direction {
                    mir2_shared::enums::MirDirection::Up => (0, -1),
                    mir2_shared::enums::MirDirection::UpRight => (1, -1),
                    mir2_shared::enums::MirDirection::Right => (1, 0),
                    mir2_shared::enums::MirDirection::DownRight => (1, 1),
                    mir2_shared::enums::MirDirection::Down => (0, 1),
                    mir2_shared::enums::MirDirection::DownLeft => (-1, 1),
                    mir2_shared::enums::MirDirection::Left => (-1, 0),
                    mir2_shared::enums::MirDirection::UpLeft => (-1, -1),
                };

                let hit_cell = (x + dx, y + dy);

                // 真服常见语义：按方向取前方目标。这里做“一格命中”。
                let mut hit_monster_id: Option<u32> = None;
                for (mid, m) in state.monsters.iter() {
                    if m.hp > 0 && m.pos == hit_cell {
                        hit_monster_id = Some(*mid);
                        break;
                    }
                }

                let Some(mid) = hit_monster_id else {
                    return;
                };

                let damage = 10;
                if let Some(m) = state.monsters.get_mut(&mid) {
                    m.hp -= damage;

                    let _ = response_tx.send(NetworkEvent::ObjectStruck {
                        object_id: mid,
                        attacker_id: 1,
                        damage,
                    });

                    if m.hp <= 0 {
                        let _ = response_tx.send(NetworkEvent::ObjectDied { object_id: mid });
                        let _ = response_tx.send(NetworkEvent::ObjectRemove {
                            packet: mir2_shared::packets::server::ObjectRemove { object_id: mid },
                        });
                        state.monsters.remove(&mid);
                    }
                }
            }

            // 聊天请求
                NetworkEvent::ChatRequest {
                    message,
                    linked_items,
                } => {
                    tracing::info!(
                        "[MOCK] ChatRequest: message={:?} linked_items={} ",
                        message,
                        linked_items.len()
                    );
                // 回显消息
                let _ = response_tx.send(NetworkEvent::ChatMessage {
                    sender: "MockServer".to_string(),
                    message: format!("Echo: {}", message),
                    chat_type: mir2_shared::enums::ChatType::Normal,
                });
            }

            // ===== NPC 交互（Mock 权威服务器） =====
            NetworkEvent::NPCCallRequest { npc_object_id, key } => {
                println!(
                    "💬 [MOCK] NPCCallRequest npc_object_id={} key={:?}",
                    npc_object_id, key
                );
                tracing::info!(
                    "💬 [MOCK] NPCCallRequest npc_object_id={} key={:?}",
                    npc_object_id, key
                );

                let make_goods = || {
                    // 提供可验证的商品列表：
                    // - 1000 有两个版本（触发 BuySub 子商品窗口）
                    // - 1000/1001 为可堆叠（触发 MirAmountBox 等价物）
                    let mut items = Vec::new();
                    let mut uid: u64 = 1;

                    let mut make_item =
                        |idx: i32, is_shop_item: bool, price: u32, stack: u16, image: u16| {
                            let mut info = mir2_shared::data::item::ItemInfo::default();
                            info.index = idx;
                            info.name = format!("MockItem{}", idx);
                            info.price = price;
                            info.stack_size = stack;
                            info.image = image;

                            let mut it = mir2_shared::data::item::UserItem::with_info(info);
                            it.unique_id = uid;
                            uid += 1;
                            it.is_shop_item = is_shop_item;
                            it.count = 1;
                            it
                        };

                    items.push(make_item(1000, true, 100, 10, 116));
                    items.push(make_item(1000, false, 120, 10, 116));
                    items.push(make_item(1001, true, 80, 20, 116));
                    items.push(make_item(1002, true, 200, 1, 116));
                    items.push(make_item(1003, true, 300, 1, 116));
                    items.push(make_item(1004, true, 400, 1, 116));

                    items
                };

                let key = key.trim().to_string();
                // 对齐客户端：左键 NPC 默认发 [@Main]。
                // 这里把 "" 与 "[@Main]" 都视为“初次打开/主入口”。
                if key.is_empty() || key == "[@Main]" {
                    // 初次打开：返回带可点击选项的对话（对齐 C# 的 <text/@Action>）
                    let _ = response_tx.send(NetworkEvent::NpcDialog {
                        // 对齐真服：NPCResponse 只有 page，不带 object_id；客户端用 ActiveNpc 追踪
                        npc_id: 0,
                        dialog: "欢迎！\n请选择：<购买/@Shop>  <离开/@Exit>\n\n<<大按钮购买/@Shop/RoyalBlue>>\n<<大按钮离开/@Exit/Red>>\n(调试) 点击 <购买/@Shop> 会打开商店窗口。"
                            .to_string(),
                    });
                } else if key == "[@Shop]" {
                    // 打开商店
                    let _ = response_tx.send(NetworkEvent::NpcDialog {
                        npc_id: 0,
                        dialog: "已为你打开商店。{祝你购物愉快/Yellow}\n((官网/http://example.com))\n<继续购买/@Shop>  <离开/@Exit>"
                            .to_string(),
                    });

                    let items = make_goods();
                    state.last_shop_goods = items.clone();
                    let _ = response_tx.send(NetworkEvent::NPCGoods {
                        items,
                        rate: 1.0,
                        panel_type: mir2_shared::enums::PanelType::Buy,
                        hide_added_stats: false,
                    });
                } else {
                    let _ = response_tx.send(NetworkEvent::NpcDialog {
                        npc_id: 0,
                        dialog: format!("(MOCK) 收到选项 key={}\n<购买/@Shop>  <离开/@Exit>", key),
                    });
                }
            }

            NetworkEvent::BuyItemRequest {
                item_index,
                count,
                panel_type,
            } => {
                println!(
                    "🛒 [MOCK] BuyItemRequest item_index={} count={} panel_type={}",
                    item_index,
                    count,
                    panel_type
                );
                tracing::info!(
                    "🛒 [MOCK] BuyItemRequest item_index={} count={} panel_type={}",
                    item_index,
                    count,
                    panel_type
                );
                // 在最后一次下发的货单里按 unique_id 反查（对齐 C#：BuyItemRequest.item_index 携带 UniqueID）
                let Some(template) = state
                    .last_shop_goods
                    .iter()
                    .find(|it| it.unique_id == item_index)
                    .cloned()
                else {
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
                        message: format!(
                            "(MOCK) 金币不足：需要 {}，当前 {}",
                            total_cost, state.player_gold
                        ),
                    });
                    return;
                }

                state.player_gold -= total_cost;

                // 真服会发 LoseGold + GainedItem；这里用抽象事件驱动（会被 NetworkApplySystem 落地到 Inventory/Currency）
                let _ = response_tx.send(NetworkEvent::GoldChanged {
                    delta: -(total_cost as i32),
                });

                let mut purchased = template.clone();
                purchased.count = (count.min(u16::MAX as u32)) as u16;
                let _ = response_tx.send(NetworkEvent::ItemGained { item: purchased });

                let _ = response_tx.send(NetworkEvent::SystemMessage {
                    message: format!(
                        "(MOCK) 购买成功：unique_id={} x{} 花费={} (panel_type={})",
                        item_index, count, total_cost, panel_type
                    ),
                });
            }

            // 其他事件暂不处理
            _ => {
                tracing::debug!("⚠️ MockNetwork 暂不处理事件: {:?}", event);
            }
        }
    }

    fn tick_world(response_tx: &Sender<NetworkEvent>, state: &mut MockWorldState) {
        if !state.in_game {
            return;
        }

        // 远程玩家 AI：更高频率推进
        Self::tick_remote_players_ai(response_tx, state);

        // 刷怪：按区域补足数量
        Self::tick_zone_spawns(response_tx, state);

        // 怪物 AI：追击 + 攻击本地玩家（server-driven combat）
        Self::tick_monster_combat(response_tx, state);

        // 玩家死亡：回城复活（离线 mock 最小闭环）
        Self::tick_player_respawn(response_tx, state);

        // 怪物游荡：低频随机走动（避免刷屏/性能）
        Self::tick_monster_wander(response_tx, state);
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

    fn tick_monster_combat(response_tx: &Sender<NetworkEvent>, state: &mut MockWorldState) {
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
        let aggro_range = 8i32;
        let attack_cooldown = Duration::from_millis(900);
        let chase_interval = Duration::from_millis(240);

        for (mid, dist) in candidates {
            if acted >= limit {
                break;
            }
            if dist > aggro_range {
                continue;
            }

            let Some(m) = state.monsters.get(&mid).copied() else {
                continue;
            };
            if m.hp <= 0 {
                continue;
            }

            let (mx, my) = m.pos;
            let dx = (px - mx).signum();
            let dy = (py - my).signum();
            let dir = Self::dir_from_delta(dx, dy);

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
                    packet: mir2_shared::packets::server::ObjectAttack {
                        object_id: mid,
                        location_x: (mx.max(0) as u32),
                        location_y: (my.max(0) as u32),
                        direction: dir as u8,
                        spell: 0,
                        level: 0,
                        attack_type: 0,
                    },
                });

                let damage = 6 + (Self::rng_next_u32(&mut state.rng) % 7) as i32;
                state.player_hp_current = (state.player_hp_current - damage).max(0);

                // 用 ObjectStruck/ObjectDied 走统一落地（NetworkApplySystem 会给玩家播放受击/死亡音效 + 飘字）
                let _ = response_tx.send(NetworkEvent::ObjectStruck {
                    object_id: player_id,
                    attacker_id: mid,
                    damage,
                });
                let _ = response_tx.send(NetworkEvent::HealthChanged {
                    current: state.player_hp_current.max(0) as u32,
                    max: state.player_hp_max.max(1) as u32,
                });

                if state.player_hp_current <= 0 {
                    let _ = response_tx.send(NetworkEvent::ObjectDied { object_id: player_id });
                    let _ = response_tx.send(NetworkEvent::SystemMessage {
                        message: "(MOCK) You died".to_string(),
                    });

                    // 标记死亡开始时间，用于 respawn
                    if state.player_dead_since.is_none() {
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

    fn exp_for_next_level(level: u16) -> i64 {
        // 简单曲线：等级越高升级所需越高。离线 mock 不追求严格还原。
        (level as i64) * 60 + 40
    }

    fn dir_from_delta(dx: i32, dy: i32) -> mir2_shared::enums::MirDirection {
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

    fn send_object_player_update(response_tx: &Sender<NetworkEvent>, rp: &MockRemotePlayerState) {
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
                mount_type: 0,
                riding_mount: false,
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

    fn rng_next_u32(seed: &mut u64) -> u32 {
        // xorshift64*
        let mut x = *seed;
        x ^= x >> 12;
        x ^= x << 25;
        x ^= x >> 27;
        *seed = x;
        ((x.wrapping_mul(0x2545F4914F6CDD1D_u64)) >> 32) as u32
    }

    fn random_pos_in_zone(seed: &mut u64, zone: &MockZone) -> (i32, i32) {
        let r = zone.radius.max(1);
        let dx = (Self::rng_next_u32(seed) as i32 % (r * 2 + 1)) - r;
        let dy = (Self::rng_next_u32(seed) as i32 % (r * 2 + 1)) - r;
        (zone.center.0 + dx, zone.center.1 + dy)
    }

    fn spawn_monster_in_zone(response_tx: &Sender<NetworkEvent>, state: &mut MockWorldState, zone_idx: usize) {
        let Some(zone) = state.zones.get(zone_idx).cloned() else {
            return;
        };

        // 选择一个区域内随机点作为出生点
        let (x, y) = Self::random_pos_in_zone(&mut state.rng, &zone);
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
    }

    fn map_is_walkable(state: &MockWorldState, x: i32, y: i32) -> bool {
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

    fn tick_zone_spawns(response_tx: &Sender<NetworkEvent>, state: &mut MockWorldState) {
        // 每个区域按 respawn_interval 补怪
        for zone_idx in 0..state.zones.len() {
            let Some(zone) = state.zones.get_mut(zone_idx) else {
                continue;
            };
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

    fn tick_monster_wander(response_tx: &Sender<NetworkEvent>, state: &mut MockWorldState) {
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
            if (Self::rng_next_u32(&mut state.rng) % 4) != 0 {
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

    fn tick_remote_players_ai(response_tx: &Sender<NetworkEvent>, state: &mut MockWorldState) {
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
        }

        impl<'a> RemoteBtCtx<'a> {
            fn is_occupied(&self, tile: (i32, i32)) -> bool {
                self.occupied.contains(&tile)
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
                if self.rp.last_zone_eval.elapsed() <= Duration::from_millis(1800) {
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

                self.rp.direction = MockNetwork::dir_from_delta(dx, dy);

                let mut nx = rx + dx;
                let mut ny = ry + dy;

                // 简单边界：避免大量玩家跑出默认地图范围太远导致“看起来都挤一块”。
                // 可用环境变量覆盖 mock 地图尺寸。
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
                nx = nx.clamp(0, map_w.saturating_sub(1).max(0));
                ny = ny.clamp(0, map_h.saturating_sub(1).max(0));
                if self.is_occupied((nx, ny)) {
                    if dx != 0 && dy != 0 {
                        if !self.is_occupied((rx + dx, ry)) {
                            nx = rx + dx;
                            ny = ry;
                            self.rp.direction = MockNetwork::dir_from_delta(dx, 0);
                        } else if !self.is_occupied((rx, ry + dy)) {
                            nx = rx;
                            ny = ry + dy;
                            self.rp.direction = MockNetwork::dir_from_delta(0, dy);
                        } else {
                            // 恢复占位
                            self.occupied.insert(old_tile);
                            return BtStatus::Failure;
                        }
                    } else {
                        // 恢复占位
                        self.occupied.insert(old_tile);
                        return BtStatus::Failure;
                    }
                }

                self.rp.grid = (nx, ny);
                self.occupied.insert((nx, ny));

                if prefer_run {
                    let _ = self.response_tx.send(NetworkEvent::ObjectRun {
                        packet: mir2_shared::packets::server::ObjectRun {
                            object_id: self.rp.id,
                            location_x: nx,
                            location_y: ny,
                            direction: self.rp.direction,
                        },
                    });
                } else {
                    let _ = self.response_tx.send(NetworkEvent::ObjectWalk {
                        packet: mir2_shared::packets::server::ObjectWalk {
                            object_id: self.rp.id,
                            location_x: nx,
                            location_y: ny,
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
            ctx.pick_target_in_zone(18);
            BtStatus::Success
        }

        fn act_rest_gate(ctx: &mut RemoteBtCtx) -> BtStatus {
            // 正在休息：到点后恢复 Seek，并允许继续决策
            if ctx.rp.mode == RemoteAiMode::Rest {
                if ctx.rp.last_mode_change.elapsed() > Duration::from_millis(600) {
                    ctx.rp.mode = RemoteAiMode::Seek;
                    ctx.rp.last_mode_change = ctx.now;
                    return BtStatus::Failure;
                }
                return BtStatus::Running;
            }

            // 偶尔发呆（只在非战斗态）
            if matches!(ctx.rp.mode, RemoteAiMode::Roam | RemoteAiMode::Seek)
                && (MockNetwork::rng_next_u32(ctx.rng) % 50 == 0)
            {
                ctx.rp.mode = RemoteAiMode::Rest;
                ctx.rp.last_mode_change = ctx.now;
                return BtStatus::Running;
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
            if dist_far > 32 {
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

            if ctx.rp.last_attack.elapsed() < Duration::from_millis(650) {
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
            if dist_far > 32 {
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
            let (rx, ry) = ctx.rp.grid;
            let arrive_dist = (z.center.0 - rx).abs() + (z.center.1 - ry).abs();
            if arrive_dist <= 2 {
                ctx.rp.zone_idx = ctx.rp.goal_zone_idx;
                ctx.rp.mode = RemoteAiMode::Seek;
                ctx.rp.last_mode_change = ctx.now;
                return BtStatus::Success;
            }
            ctx.step_towards(z.center.0, z.center.1, true)
        }

        fn act_roam(ctx: &mut RemoteBtCtx) -> BtStatus {
            if ctx.rp.last_roam_pick.elapsed() > Duration::from_millis(1500) {
                ctx.rp.last_roam_pick = ctx.now;
                if let Some(z) = ctx.zones.get(ctx.rp.zone_idx) {
                    ctx.rp.roam_goal = MockNetwork::random_pos_in_zone(ctx.rng, z);
                }
            }
            ctx.rp.mode = RemoteAiMode::Roam;
            let prefer_run = matches!(ctx.rp.mode, RemoteAiMode::Travel | RemoteAiMode::Chase)
                || (MockNetwork::rng_next_u32(ctx.rng) % 4 == 0);
            ctx.step_towards(ctx.rp.roam_goal.0, ctx.rp.roam_goal.1, prefer_run)
        }

        let now = Instant::now();
        let zones = &state.zones;
        let monsters = &mut state.monsters;
        let mut rng = state.rng;

        // 预构建占位集合：避免 3000 人时每步 O(n) 扫描导致 O(n^2)
        let mut occupied_tiles: HashSet<(i32, i32)> = state.remote_players.iter().map(|p| p.grid).collect();

        let player_count = state.remote_players.len();
        for i in 0..player_count {
            let (_left, right) = state.remote_players.split_at_mut(i);
            let Some((rp, _rest)) = right.split_first_mut() else {
                break;
            };

            if rp.last_tick.elapsed() < Duration::from_millis(200) {
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

    /// 加载地图并发送 MapChanged 事件，并将可走性缓存到 MockWorldState。
    ///
    /// 返回实际采用的出生点（若原出生点不可走，会在附近寻找可走格）。
    fn load_and_send_map(
        response_tx: &Sender<NetworkEvent>,
        state: &mut MockWorldState,
        map_path: &str,
        map_index: i32,
        title: &str,
        spawn_x: i32,
        spawn_y: i32,
        direction: u8,
    ) -> (i32, i32) {
        let resolved_path = crate::resources::map_reader::resolve_map_path(map_path);
        tracing::info!("📂 尝试加载地图: {} -> {}", map_path, resolved_path);

        match MapReader::new(&resolved_path) {
            Ok(map_reader) => {
                tracing::info!(
                    "✅ 成功加载地图: {} ({}x{})",
                    resolved_path,
                    map_reader.width,
                    map_reader.height
                );

                // 缓存碰撞：map_cells[x][y] -> map_walkable[y * w + x]
                state.map_width = map_reader.width.max(0);
                state.map_height = map_reader.height.max(0);
                let w = state.map_width as usize;
                let h = state.map_height as usize;
                state.map_walkable.clear();
                state.map_walkable.reserve(w.saturating_mul(h));
                for y in 0..h {
                    for x in 0..w {
                        let walkable = map_reader
                            .map_cells
                            .get(x)
                            .and_then(|col| col.get(y))
                            .map(|c| c.is_walkable())
                            .unwrap_or(true);
                        state.map_walkable.push(if walkable { 1 } else { 0 });
                    }
                }

                // 如果出生点不可走（或地图加载异常导致越界），在附近找一个可走格。
                let mut final_spawn = (spawn_x, spawn_y);
                if !Self::map_is_walkable(state, final_spawn.0, final_spawn.1) {
                    let max_r: i32 = 12;
                    'outer: for r in 1..=max_r {
                        for dy in -r..=r {
                            for dx in -r..=r {
                                // 优先扫描“边框”以更快找到最近点
                                if dx.abs() != r && dy.abs() != r {
                                    continue;
                                }
                                let tx = spawn_x + dx;
                                let ty = spawn_y + dy;
                                if Self::map_is_walkable(state, tx, ty) {
                                    final_spawn = (tx, ty);
                                    break 'outer;
                                }
                            }
                        }
                    }
                }

                // 提取纯文件名（不含路径和扩展名）用于下发 MapChanged
                let file_name = std::path::Path::new(&resolved_path)
                    .file_stem()
                    .and_then(|s| s.to_str())
                    .unwrap_or("0")
                    .to_string();

                // 发送 MapChanged 事件 (与 C# Server 格式一致)
                let _ = response_tx.send(NetworkEvent::MapChanged {
                    packet: mir2_shared::packets::server::MapChanged {
                        map_index,
                        file_name, // 只发送纯文件名 "0"
                        title: title.to_string(),
                        minimap: 0,
                        big_map: 0,
                        lights: 0,
                        location_x: final_spawn.0,
                        location_y: final_spawn.1,
                        direction,
                        map_dark_light: 0,
                        music: 0,
                        weather: 0,
                    },
                });

                final_spawn
            }
            Err(e) => {
                tracing::error!("❌ 加载地图失败 {}: {:?}", map_path, e);
                // 失败时退化：不做碰撞校验
                state.map_width = 0;
                state.map_height = 0;
                state.map_walkable.clear();
                (spawn_x, spawn_y)
            }
        }
    }

    /// 停止模拟网络
    #[allow(dead_code)]
    pub fn stop(&self) {
        self.running.store(false, Ordering::Relaxed);
    }
}

impl Drop for MockNetwork {
    fn drop(&mut self) {
        self.running.store(false, Ordering::Relaxed);
        tracing::debug!("MockNetwork 实例销毁");
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_mock_network_connection() {
        let (tx, rx) = MockNetwork::new();

        // 等待自动发送的 Connected 事件
        thread::sleep(Duration::from_millis(200));

        // 应该收到连接成功事件
        let events: Vec<_> = rx.try_iter().collect();
        assert!(events.iter().any(|e| matches!(e, NetworkEvent::Connected)));

        // 发送断开请求
        tx.send(NetworkEvent::DisconnectRequest).unwrap();
        thread::sleep(Duration::from_millis(200));

        // 应该收到断开事件
        let events: Vec<_> = rx.try_iter().collect();
        assert!(events
            .iter()
            .any(|e| matches!(e, NetworkEvent::Disconnected { .. })));
    }

    #[test]
    fn test_mock_network_login() {
        let (tx, rx) = MockNetwork::new();

        // 发送登录请求
        tx.send(NetworkEvent::LoginRequest {
            username: "test_user".to_string(),
            password: "test_pass".to_string(),
        })
        .unwrap();

        thread::sleep(Duration::from_millis(300));

        // 应该收到登录成功事件
        let events: Vec<_> = rx.try_iter().collect();
        assert!(events
            .iter()
            .any(|e| matches!(e, NetworkEvent::LoginSuccess { .. })));
    }
}
