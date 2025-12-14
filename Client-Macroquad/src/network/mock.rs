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
use std::collections::HashMap;

#[derive(Debug, Clone, Copy)]
struct MockMonsterState {
    pos: (i32, i32),
    hp: i32,
}

#[derive(Debug, Clone)]
struct MockWorldState {
    in_game: bool,

    // 本地玩家（server-authoritative）
    player_gold: u32,
    inventory_capacity: usize,
    player_grid: (i32, i32),

    // NPC 商店：最近一次下发给客户端的货单（用于 BuyItemRequest 通过 unique_id 反查）
    last_shop_goods: Vec<mir2_shared::data::item::UserItem>,

    last_player_move_req: Instant,

    // server-authoritative monsters (position + HP)
    monsters: HashMap<u32, MockMonsterState>,

    moving_monster_id: u32,
    moving_monster_pos: (i32, i32),
    moving_monster_path_idx: usize,
    last_world_tick: Instant,
}

impl Default for MockWorldState {
    fn default() -> Self {
        Self {
            in_game: false,

            player_gold: 1000,
            inventory_capacity: 40,
            player_grid: (330, 330),
            last_shop_goods: Vec::new(),

            last_player_move_req: Instant::now(),

            monsters: HashMap::new(),

            moving_monster_id: 3001,
            moving_monster_pos: (338, 332),
            moving_monster_path_idx: 0,
            last_world_tick: Instant::now(),
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
                Self::load_and_send_map(
                    &response_tx,
                    "Map/n0.map",
                    0,
                    "盟重土城",
                    330,
                    330,
                    mir2_shared::enums::MirDirection::Down as u8,
                );

                // 模拟玩家信息
                // 关键：下发初始背包（None = 不下发，会导致后续 ItemGained 没 UI 承载）
                state.player_gold = 1000;
                state.inventory_capacity = 40;
                state.player_grid = (330, 330);

                let _ = response_tx.send(NetworkEvent::UserInformation {
                    packet: mir2_shared::packets::server::UserInformation {
                        object_id: 1,
                        real_id: 1,
                        name: "TestUser".to_string(),
                        guild_name: "".to_string(),
                        guild_rank: "".to_string(),
                        name_colour: 0,
                        class: mir2_shared::enums::MirClass::Warrior,
                        gender: mir2_shared::enums::MirGender::Male,
                        level: 1,
                        location_x: 330,
                        location_y: 330,
                        direction: mir2_shared::enums::MirDirection::Down,
                        hair: 0,
                        hp: 100,
                        mp: 50,
                        experience: 0,
                        max_experience: 0,
                        level_effects: mir2_shared::enums::LevelEffects::empty(),
                        has_hero: false,
                        hero_behaviour: mir2_shared::enums::HeroBehaviour::Follow,
                        inventory: Some(vec![None; state.inventory_capacity]),
                        equipment: None,
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

                // ====== Mock(权威服务器)：用真实 server packet 形状生成 NPC/怪物 ======
                // 坐标为格子坐标（与 UserInformation/MapChanged 一致）
                let _ = response_tx.send(NetworkEvent::ObjectNpc {
                    packet: mir2_shared::packets::server::ObjectNpc {
                        object_id: 2001,
                        name: "TestNPC".to_string(),
                        name_colour: 0,
                        image: 0,
                        colour: 0,
                        // 原位置在该地图点位会被前景树完全遮挡，挪到更空旷的位置便于测试交互。
                        location_x: 332,
                        location_y: 330,
                        direction: mir2_shared::enums::MirDirection::Down,
                    },
                });

                let _ = response_tx.send(NetworkEvent::ObjectMonster {
                    packet: mir2_shared::packets::server::ObjectMonster {
                        object_id: 3001,
                        name: "TestMonsterA".to_string(),
                        name_colour: 0,
                        location_x: 338,
                        location_y: 332,
                        image: 0,
                        direction: mir2_shared::enums::MirDirection::Down,
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

                let _ = response_tx.send(NetworkEvent::ObjectMonster {
                    packet: mir2_shared::packets::server::ObjectMonster {
                        object_id: 3002,
                        name: "TestMonsterB".to_string(),
                        name_colour: 0,
                        location_x: 342,
                        location_y: 334,
                        image: 1,
                        direction: mir2_shared::enums::MirDirection::Left,
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

                // 启动世界 tick：让怪物持续移动（完全由服务器推送）
                state.in_game = true;

                // 初始化怪物（位置需与 ObjectMonster 下发一致）
                state.monsters.clear();
                state.monsters.insert(
                    3001,
                    MockMonsterState {
                        pos: (338, 332),
                        hp: 30,
                    },
                );
                state.monsters.insert(
                    3002,
                    MockMonsterState {
                        pos: (342, 334),
                        hp: 20,
                    },
                );
                state.moving_monster_id = 3001;
                state.moving_monster_pos = (338, 332);
                state.moving_monster_path_idx = 0;
                state.last_world_tick = Instant::now();
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
                state.player_grid = (nx, ny);

                let _ = response_tx.send(NetworkEvent::PlayerLocationChanged { x: nx, y: ny });
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

        // 1000ms 一步，模拟服务器驱动的 ObjectWalk（慢一点，观感更接近原版）
        if state.last_world_tick.elapsed() < Duration::from_millis(1000) {
            return;
        }
        state.last_world_tick = Instant::now();

        // 如果巡逻怪已经死亡/移除，就不再推进
        if !state.monsters.contains_key(&state.moving_monster_id) {
            return;
        }

        // 简单巡逻路径
        let path: [(i32, i32, mir2_shared::enums::MirDirection); 8] = [
            (339, 332, mir2_shared::enums::MirDirection::Right),
            (340, 332, mir2_shared::enums::MirDirection::Right),
            (340, 333, mir2_shared::enums::MirDirection::Down),
            (340, 334, mir2_shared::enums::MirDirection::Down),
            (339, 334, mir2_shared::enums::MirDirection::Left),
            (338, 334, mir2_shared::enums::MirDirection::Left),
            (338, 333, mir2_shared::enums::MirDirection::Up),
            (338, 332, mir2_shared::enums::MirDirection::Up),
        ];

        let (nx, ny, dir) = path[state.moving_monster_path_idx % path.len()];
        state.moving_monster_path_idx = (state.moving_monster_path_idx + 1) % path.len();
        state.moving_monster_pos = (nx, ny);

        if let Some(m) = state.monsters.get_mut(&state.moving_monster_id) {
            m.pos = (nx, ny);
        }

        let _ = response_tx.send(NetworkEvent::ObjectWalk {
            packet: mir2_shared::packets::server::ObjectWalk {
                object_id: state.moving_monster_id,
                location_x: nx,
                location_y: ny,
                direction: dir,
            },
        });
    }

    /// 加载地图并发送 MapChanged 事件
    fn load_and_send_map(
        response_tx: &Sender<NetworkEvent>,
        map_path: &str,
        map_index: i32,
        title: &str,
        spawn_x: i32,
        spawn_y: i32,
        direction: u8,
    ) {
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
                        location_x: spawn_x,
                        location_y: spawn_y,
                        direction,
                        map_dark_light: 0,
                        music: 0,
                        weather: 0,
                    },
                });

                // TODO: 这里需要将 MapReader 数据发送给客户端
                // 目前暂时只发送事件，MapReader 需要在游戏循环中处理
            }
            Err(e) => {
                tracing::error!("❌ 加载地图失败 {}: {:?}", map_path, e);
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
