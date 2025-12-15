// Packet Handlers - 数据包处理器模块
// 
// 将 GameClient 的大量 on_* 方法拆分成多个独立的 Handler
// 每个 Handler 只负责：协议 → NetworkEvent 的转换
// 不存储状态，状态由 ECS 组件和 Resources 管理

pub mod connection;
pub mod character;
pub mod movement;
pub mod combat;
pub mod chat;
pub mod group;
pub mod guild;
pub mod trade;
pub mod item;
pub mod npc;
pub mod quest;

// Re-export all handlers
pub use connection::ConnectionHandler;
pub use character::CharacterHandler;
pub use movement::MovementHandler;
pub use combat::CombatHandler;
pub use chat::ChatHandler;
pub use group::GroupHandler;
pub use guild::GuildHandler;
pub use trade::TradeHandler;
pub use item::ItemHandler;
pub use npc::NpcHandler;
pub use quest::QuestHandler;

use mir2_shared::packets::PacketHeader;
use crate::resources::LibraryName;

/// Network events - 网络事件（服务器 ↔ 客户端）
/// 
/// 设计原则：
/// - 服务器 → 客户端：以过去时态命名（Connected, LoginSuccess）
/// - 客户端 → 服务器：以 Request 后缀命名（LoginRequest, MoveRequest）
/// 
/// 职责边界：
/// - ✅ 负责：网络协议转换（Packet → Event）
/// - ❌ 不负责：客户端系统间通信（那是 GameLogicEvent）
/// 
/// 优点：
/// - 类型安全：编译时检查所有事件
/// - 双向统一：客户端和服务器事件统一管理
/// - 易于理解：命名清晰表明数据流向
#[derive(Debug, Clone)]
pub enum NetworkEvent {
    // ========================================================================
    // 连接事件（Connection Events）
    // ========================================================================
    
    // 服务器 → 客户端
    Connected,
    Disconnected { reason: String },
    KeepAliveReceived { time: i64 },
    ClientVersionResponse { result: u8 }, // 0: Wrong Version, 1: Correct Version
    
    // 客户端 → 服务器
    DisconnectRequest,
    KeepAliveSend { time: i64 },
    ClientVersionSend { version_hash: Vec<u8> },
    
    // ========================================================================
    // 认证事件（Authentication Events）
    // ========================================================================
    
    // 客户端 → 服务器
    LoginRequest { username: String, password: String },
    NewAccountRequest { 
        account_id: String, 
        password: String, 
        birth_date: i64,
        username: String,
        secret_question: String,
        secret_answer: String,
        email: String,
    },
    ChangePasswordRequest {
        account_id: String,
        current_password: String,
        new_password: String,
    },
    
    // 服务器 → 客户端
    LoginSuccess { characters: Vec<mir2_shared::SelectInfo> },
    LoginFailed { reason: String },
    NewAccountSuccess,
    NewAccountFailed { reason: String },
    ChangePasswordSuccess,
    ChangePasswordFailed { reason: String },
    
    // ========================================================================
    // 角色管理事件（Character Management Events）
    // ========================================================================
    
    // 客户端 → 服务器
    NewCharacterRequest { name: String, class: u8, gender: u8 },
    DeleteCharacterRequest { index: i32 },
    StartGameRequest { character_index: i32 },
    
    // 服务器 → 客户端
    CharacterCreated { name: String },
    CharacterDeleted { index: u32 },
    StartGame { packet: mir2_shared::packets::server::StartGame },
    StartGameDelay { packet: mir2_shared::packets::server::StartGameDelay },
    StartGameBanned { packet: mir2_shared::packets::server::StartGameBanned },
    UserInformation { packet: mir2_shared::packets::server::UserInformation },
    
    // ========================================================================
    // 地图事件（Map Events）
    // ========================================================================
    
    // 服务器 → 客户端
    MapInformation { packet: mir2_shared::packets::server::MapInformation },
    MapChanged { packet: mir2_shared::packets::server::MapChanged },
    
    // ========================================================================
    // 玩家移动事件（Player Movement Events）
    // ========================================================================
    
    // 客户端 → 服务器
    MoveRequest { direction: mir2_shared::enums::MirDirection },
    WalkRequest { direction: mir2_shared::enums::MirDirection },
    RunRequest { direction: mir2_shared::enums::MirDirection },
    TurnRequest { direction: mir2_shared::enums::MirDirection },
    
    // 服务器 → 客户端
    PlayerLocationChanged { x: i32, y: i32 },
    ObjectMoved { object_id: u32, x: i32, y: i32, direction: mir2_shared::enums::MirDirection },
    
    // ========================================================================
    // 玩家状态事件（Player State Events）
    // ========================================================================
    
    // 服务器 → 客户端
    HealthChanged { current: u32, max: u32 },
    ManaChanged { current: u32, max: u32 },
    ExperienceGained { amount: i64 },
    LevelUp { new_level: u16 },
    /// 金币变化（对齐协议：GainedGold / LoseGold）。
    /// 语义是“变化量”，可能为负数。
    GoldChanged { delta: i32 },
    
    // ========================================================================
    // 战斗事件（Combat Events）
    // ========================================================================
    
    // 客户端 → 服务器
    AttackRequest { direction: mir2_shared::enums::MirDirection, spell: u8 },
    MagicRequest { spell: u8, direction: mir2_shared::enums::MirDirection, target_id: u32, location: Option<(i32, i32)> },
    
    // 服务器 → 客户端
    PlayerStruck { attacker_id: u32, damage: i32 },
    PlayerDied,
    ObjectStruck { object_id: u32, attacker_id: u32, damage: i32 },
    ObjectDied { object_id: u32 },
    ObjectAttack { packet: mir2_shared::packets::server::ObjectAttack },
    
    // ========================================================================
    // 聊天事件（Chat Events）
    // ========================================================================
    
    // 客户端 → 服务器
    ChatRequest { message: String, linked_items: Vec<mir2_shared::ChatItem> },
    
    // 服务器 → 客户端
    ChatMessage { sender: String, message: String, chat_type: mir2_shared::enums::ChatType },
    SystemMessage { message: String },
    
    // ========================================================================
    // 物体事件（Object Events）
    // ========================================================================
    
    // 服务器 → 客户端
    ObjectSpawned { object_id: u32, object_type: ObjectType },
    ObjectRemoved { object_id: u32 },

    // 服务器 → 客户端（真实对象包：用于 server-driven 世界同步）
    ObjectPlayer { packet: mir2_shared::packets::server::ObjectPlayer },
    ObjectMonster { packet: mir2_shared::packets::server::ObjectMonster },
    ObjectNpc { packet: mir2_shared::packets::server::ObjectNpc },
    ObjectRemove { packet: mir2_shared::packets::server::ObjectRemove },
    ObjectTurn { packet: mir2_shared::packets::server::ObjectTurn },
    ObjectWalk { packet: mir2_shared::packets::server::ObjectWalk },
    ObjectRun { packet: mir2_shared::packets::server::ObjectRun },

    // Mock / 离线可视化：直接携带渲染所需数据（不代表真实协议）
    MockLibrarySpriteSpawn {
        object_id: u32,
        object_type: ObjectType,
        library: LibraryName,
        index: i32,
        location_x: i32,
        location_y: i32,
    },
    MockLibrarySpriteDespawn { object_id: u32 },
    
    // ========================================================================
    // 物品事件（Item Events）
    // ========================================================================
    
    // 客户端 → 服务器
    PickupItemRequest { location: (i32, i32) },
    MoveItemRequest { grid: u8, from: u32, to: u32 },
    DropItemRequest { unique_id: u64, count: u32 },
    UseItemRequest { unique_id: u64 },
    
    // 服务器 → 客户端
    ItemGained { item: mir2_shared::UserItem },
    ItemLost { unique_id: u64 },
    ItemMoved { from: u32, to: u32 },
    
    // ========================================================================
    // 组队事件（Group Events）
    // ========================================================================
    
    // 客户端 → 服务器
    GroupInviteRequest { player_name: String },
    GroupAcceptRequest,
    GroupDeclineRequest,
    GroupLeaveRequest,
    
    // 服务器 → 客户端
    GroupInvite { inviter: String },
    GroupMemberAdded { name: String },
    GroupMemberRemoved { name: String },
    GroupDisbanded,
    
    // ========================================================================
    // 公会事件（Guild Events）
    // ========================================================================
    
    // 客户端 → 服务器
    GuildInviteRequest { player_name: String },
    GuildAcceptRequest,
    GuildDeclineRequest,
    GuildLeaveRequest,
    
    // 服务器 → 客户端
    GuildInvite { inviter: String, guild_name: String },
    GuildJoined { guild_name: String },
    GuildLeft,
    
    // ========================================================================
    // 交易事件（Trade Events）
    // ========================================================================
    
    // 客户端 → 服务器
    TradeRequest,
    TradeReplyRequest { accept: bool },
    TradeGoldRequest { amount: u32 },
    TradeConfirmRequest { locked: bool },
    TradeCancelRequest,
    
    // 服务器 → 客户端
    TradeRequested { requester: String },
    TradeStarted { partner: String },
    TradeCompleted,
    TradeCancelled,
    
    // ========================================================================
    // 任务事件（Quest Events）
    // ========================================================================
    
    // 客户端 → 服务器
    AcceptQuestRequest { npc_index: u32, quest_index: u32 },
    FinishQuestRequest { quest_index: u32, selected_item: u32 },
    AbandonQuestRequest { quest_index: u32 },
    ShareQuestRequest { quest_index: u32 },
    
    // 服务器 → 客户端
    QuestAccepted { quest_id: u32 },
    QuestCompleted { quest_id: u32 },
    QuestProgress { quest_id: u32, progress: String },
    
    // ========================================================================
    // NPC 事件（NPC Events）
    // ========================================================================
    
    // 客户端 → 服务器
    /// 对齐 C#：ClientPackets.CallNPC { ObjectID, Key }
    /// - 初次点击 NPC：key 通常为空字符串
    /// - 点击对话选项：key 通常为 "[@Action]"
    NPCCallRequest { npc_object_id: u32, key: String },
    BuyItemRequest { item_index: u64, count: u32, panel_type: u8 },
    SellItemRequest { unique_id: u64, count: u32 },
    RepairItemRequest { unique_id: u64 },
    
    // 服务器 → 客户端
    NpcDialog { npc_id: u32, dialog: String },
    NPCGoods {
        items: Vec<mir2_shared::UserItem>,
        rate: f32,
        panel_type: mir2_shared::enums::PanelType,
        hide_added_stats: bool,
    },
    
    // ========================================================================
    // 通用事件（Generic Events）
    // ========================================================================
    
    // 未处理的数据包（用于调试）
    UnhandledPacket { opcode: i16 },
}

/// Object type for spawned objects
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ObjectType {
    Player,
    Monster,
    Npc,
    Item,
    Spell,
}

/// Handler trait - all packet handlers implement this
/// 
/// Handlers are stateless and only convert packets to events
#[allow(dead_code)]
pub trait PacketHandler {
    /// Process a packet and generate events
    fn handle(&self, header: &PacketHeader, payload: &[u8]) -> Vec<NetworkEvent>;
}
