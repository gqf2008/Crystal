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
pub mod player;
pub mod group;
pub mod guild;
pub mod trade;
pub mod item;
pub mod npc;
pub mod quest;
pub mod friend;
pub mod ui_events;
pub mod hero;
pub mod mail;
pub mod market;
pub mod creature;
pub mod social;

// Re-export all handlers
pub use connection::ConnectionHandler;
pub use character::CharacterHandler;
pub use movement::MovementHandler;
pub use combat::CombatHandler;
pub use chat::ChatHandler;
pub use player::PlayerHandler;
pub use group::GroupHandler;
pub use guild::GuildHandler;
pub use trade::TradeHandler;
pub use item::ItemHandler;
pub use npc::NpcHandler;
pub use quest::QuestHandler;
pub use friend::FriendHandler;
pub use ui_events::UiEventsHandler;
pub use hero::HeroHandler;
pub use mail::MailHandler;
pub use market::MarketHandler;
pub use creature::CreatureHandler;
pub use social::SocialHandler;

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
    CharacterCreated { character: mir2_shared::SelectInfo },
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
    PlayerDied { x: u32, y: u32, direction: u8 },
    ObjectStruck { object_id: u32, attacker_id: u32, damage: i32, location_x: u32, location_y: u32, direction: u8 },
    ObjectDied { object_id: u32, location_x: u32, location_y: u32, direction: u8, death_type: u8 },
    ObjectAttack { object_id: u32, location_x: u32, location_y: u32, direction: u8, spell: u8, level: u16, attack_type: u8 },

    /// 伤害数值提示（真实协议：DamageIndicator）
    ///
    /// 说明：部分服务器用它承载实际 damage，而 ObjectStruck 只表示“受击”。
    DamageIndicator { object_id: u32, damage: i32, damage_type: u8 },

    /// 物体血量百分比同步（真实协议：ObjectHealth）
    ///
    /// 说明：多数情况下只下发 percent，不下发 max。渲染端可用 0..100 的虚拟血条。
    ObjectHealthPercent { object_id: u32, percent: u8, expire: u16 },

    /// 物体 mana 百分比同步（真实协议：ObjectMana）
    ObjectManaPercent { object_id: u32, percent: u8 },
    
    // ========================================================================
    // 聊天事件（Chat Events）
    // ========================================================================
    
    // 客户端 → 服务器
    ChatRequest { message: String, linked_items: Vec<mir2_shared::ChatItem> },
    /// 查看/检查另一个对象（通常用于查看其他玩家装备）
    InspectRequest { object_id: u32 },
    
    // 服务器 → 客户端
    ChatMessage { sender: String, message: String, chat_type: mir2_shared::enums::ChatType },
    SystemMessage { message: String },
    /// 服务器返回的玩家查看数据（装备栏等）
    PlayerInspect { packet: mir2_shared::packets::server::PlayerInspect },
    
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
    ObjectRemove { object_id: u32 },
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
    ItemLost { unique_id: u64, count: u32 },
    ItemMoved { grid: mir2_shared::enums::MirGridType, from: u32, to: u32, success: bool },
    ItemEquipped { grid: mir2_shared::enums::MirGridType, unique_id: u64, slot: u8, success: bool },
    ItemMerged { grid_from: mir2_shared::enums::MirGridType, grid_to: mir2_shared::enums::MirGridType, id_from: u64, id_to: u64, success: bool },
    ItemRemoved { grid: mir2_shared::enums::MirGridType, unique_id: u64, to: i32, success: bool },
    ItemSlotRemoved { grid: mir2_shared::enums::MirGridType, grid_to: mir2_shared::enums::MirGridType, slot: u32, unique_id: u64, success: bool },
    ItemTakenBack { from: i32, to: i32, success: bool },
    ItemStored { from: i32, to: i32, success: bool },
    ItemSplit { grid: mir2_shared::enums::MirGridType, unique_id: u64, count: u32 },
    ItemUsed { unique_id: u64 },
    ItemDropped { unique_id: u64, count: u32, success: bool },
    ItemRefreshed { item: mir2_shared::UserItem },
    ItemSlotSizeChanged { slot: u32, size: u32 },
    ItemSealed { unique_id: u64, expiry_date: i64 },
    ItemSlotEquipped { grid: mir2_shared::enums::MirGridType, grid_to: mir2_shared::enums::MirGridType, slot: u32, unique_id: u64, success: bool },
    ItemCombined { grid: mir2_shared::enums::MirGridType, id_from: u64, id_to: u64, success: bool, destroy: bool },
    ItemUpgraded { item: mir2_shared::UserItem },
    GroundItem { packet: mir2_shared::packets::server::ObjectItem },
    CreditChanged { delta: i32 },
    ObjectHarvested { object_id: u32, location_x: i32, location_y: i32, direction: mir2_shared::enums::MirDirection },
    RefineItemDeposited { from: i32, to: i32, success: bool },
    RefineItemRetrieved { from: i32, to: i32, success: bool },
    RefineCancelled { unlock: bool },
    RefineItemCompleted { unique_id: u64 },
    TradeItemDeposited { from_slot: i32, success: bool },
    TradeItemRetrieved { from_slot: i32, success: bool },
    HeroItemTakenBack { from: i32, to: i32, success: bool },
    HeroItemTransferred { from: i32, to: i32, success: bool },
    NewItemInfoReceived { item_index: i32, item_name: String },
    NewChatItemReceived { item_id: i32 },
    /// 地面金币（ObjectGold）
    ObjectGoldReceived { packet: mir2_shared::packets::server::ObjectGold },

    // 客户端 → 服务器（物品操作）
    EquipItemRequest { unique_id: u64 },
    RemoveItemRequest { unique_id: u64 },
    RemoveSlotItemRequest { slot: u32 },
    SplitItemRequest { unique_id: u64, count: u32 },
    MergeItemRequest { from: u64, to: u64 },
    StoreItemRequest { unique_id: u64 },
    TakeBackItemRequest { unique_id: u64 },
    DropGoldRequest { amount: u32 },
    EquipSlotItemRequest { slot: u32, unique_id: u64 },
    CombineItemRequest { from: u64, to: u64 },
    DropItemStackRequest { unique_id: u64, count: u32 },
    
    // ========================================================================
    // 组队事件（Group Events）
    // ========================================================================
    
    // 客户端 → 服务器
    GroupInviteRequest { player_name: String },
    GroupAcceptRequest,
    GroupDeclineRequest,
    GroupLeaveRequest { player_name: String },
    GroupKickRequest { player_name: String },
    
    // 服务器 → 客户端
    GroupInvite { inviter: String },
    GroupMemberAdded { name: String },
    GroupMemberRemoved { name: String },
    GroupDisbanded,
    GroupModeChanged { allow_group: u8 },
    GroupMembersMapUpdated { player_name: String, player_map: String },
    GroupMemberLocationUpdated { name: String, x: i32, y: i32 },
    
    // ========================================================================
    // 公会事件（Guild Events）
    // ========================================================================
    
    // 客户端 → 服务器
    GuildInviteRequest { player_name: String },
    GuildAcceptRequest,
    GuildDeclineRequest,
    GuildLeaveRequest { player_name: String },
    
    // 服务器 → 客户端
    GuildInvite { inviter: String, guild_name: String },
    GuildJoined {
        guild_name: String,
        rank_name: String,
        level: u8,
        experience: i64,
        max_experience: i64,
        gold: u32,
        spare_points: u8,
        member_count: i32,
        max_members: i32,
        voting: bool,
        item_count: u8,
        buff_count: u8,
        my_options: u8,
        my_rank_id: i32,
    },
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

    // 坐骑更新（来自服务器；通常作用于本地玩家）
    MountUpdated {
        object_id: u32,
        mount_type: i16,
        riding_mount: bool,
    },

    // 客户端 -> 服务器（坐骑操作）
    MountRideRequest { mount_type: i16 },
    MountDismountRequest,

    // UI / 表现层事件（来自服务器）
    PlaySound { sound_id: i32 },
    // ========================================================================
    // 魔法/技能事件（Magic Events）
    // ========================================================================

    // 服务器 -> 客户端
    MagicListReceived { spell: mir2_shared::enums::Spell, target_id: u32, target_x: i32, target_y: i32, cast: bool, level: u8 },
    MagicLearned { magic: mir2_shared::data::client_data::ClientMagic, hero: bool },
    MagicRemoved { spell: mir2_shared::enums::Spell, hero: bool },
    MagicLeveledUp { spell: mir2_shared::enums::Spell, level: u8, hero: bool },
    MagicDelayReceived { object_id: u32, spell: mir2_shared::enums::Spell, delay: u32 },
    MagicCastEvent { spell: mir2_shared::enums::Spell },
    ObjectMagicCast { object_id: u32, location_x: i32, location_y: i32, direction: mir2_shared::enums::MirDirection, spell: mir2_shared::enums::Spell, target_id: u32, target_x: i32, target_y: i32, cast: bool, level: u8 },
    ObjectEffectReceived { object_id: u32, effect: u16, effect_type: u8, delay_time: u32, time: u32 },
    ObjectProjectileReceived { spell: mir2_shared::enums::Spell, source: u32, destination: u32 },
    SpellToggled { spell: mir2_shared::enums::Spell, can_use: bool, hero: bool },

    // 客户端 -> 服务器
    MagicKeySet,

    // ========================================================================
    // Buff 事件（Buff Events）
    // ========================================================================

    // 服务器 -> 客户端
    BuffAdded { object_id: u32, buff_id: u32, visible: bool, expire_time: i64, infinite: bool, paused: bool },
    BuffRemoved { object_id: u32, buff_id: u32 },
    BuffPaused { object_id: u32, buff_id: u32, paused: bool },

    // ========================================================================
    // 移动扩展事件（Movement Extension Events）
    // ========================================================================

    // 服务器 -> 客户端
    ObjectHeroSpawned { packet: mir2_shared::packets::server::ObjectHero },
    ObjectHidden { object_id: u32, hidden: bool },
    ObjectShown { object_id: u32 },
    ObjectTeleportingOut { object_id: u32, teleport_type: u8 },
    ObjectTeleportingIn { object_id: u32, teleport_type: u8 },
    PlayerTeleportedIn,
    ObjectBackStepped { object_id: u32, location_x: i32, location_y: i32, direction: mir2_shared::enums::MirDirection, distance: i32 },
    PlayerBackStepped { x: i32, y: i32 },
    ObjectDashing { object_id: u32, location_x: u32, location_y: u32, direction: u8 },
    PlayerDashing { x: i32, y: i32 },
    ObjectDashFailed { object_id: u32, location_x: u32, location_y: u32, direction: u8 },
    PlayerDashFailed { location_x: u32, location_y: u32, direction: u8 },
    ObjectSatDown { object_id: u32, direction: u8, location: (i32, i32) },
    NewMapInfoReceived { packet: mir2_shared::packets::server::NewMapInfo },
    WorldMapSetupReceived { icons: Vec<mir2_shared::packets::server::WorldMapIcon> },
    SearchMapResultReceived { map_index: i32, location_x: u32, location_y: u32 },
    TimeOfDayChanged { time_of_day: u8 },

    // ========================================================================
    // 玩家状态事件（Player State Events）
    // ========================================================================

    // 服务器 -> 客户端
    PlayerUpdated { object_id: u32, light: u8, weapon: i16, weapon_effect: i16, armor: i16, wings_effect: u8 },
    AttackModeChanged { mode: u8 },
    PetModeChanged { mode: u8 },
    PlayerColourChanged { colour: u32 },
    ObjectColourChanged { object_id: u32, colour: u32 },
    ObjectGuildNameChanged2 { object_id: u32, guild_name: String },
    PlayerNameUpdated { object_id: u32, name: String },
    UserNameUpdated { object_id: u32, name: String },
    UserInventoryReceived { items: Vec<mir2_shared::UserItem> },
    UserEquipmentReceived { items: Vec<mir2_shared::UserItem> },

    // ========================================================================
    // 交易扩展事件（Trade Extended Events）
    // ========================================================================

    // 服务器 -> 客户端
    TradeGoldAdded { amount: u32 },
    TradeItemAdded { items: Vec<Option<mir2_shared::data::item::UserItem>> },
    TradeConfirmedEvent { locked: bool },
    TradeCancelledEvent { unlock: bool },

    // ========================================================================
    // 任务扩展事件（Quest Extended Events）
    // ========================================================================

    // 服务器 -> 客户端
    QuestListUpdated,
    QuestItemGained { item_id: i32 },
    QuestItemLost { unique_id: u64 },
    QuestShared { quest_id: u32 },
    QuestProgressUpdated { quest_id: u32, progress: String },
    QuestInfoReceived {
        quest_id: u32,
        name: String,
        group: String,
        description: String,
        level_req: u32,
        reward_exp: u64,
        reward_gold: u32,
    },

    // ========================================================================
    // 好友事件（Friend Events）
    // ========================================================================

    // 服务器 -> 客户端
    FriendUpdated { friends: Vec<crate::ui::ui_state::FriendEntry> },

    // 客户端 -> 服务器
    AddFriendRequest { name: String },
    RemoveFriendRequest { object_id: u32 },
    RefreshFriendsRequest,
    AddMemoRequest { object_id: u32, memo: String },

    // ========================================================================
    // 公会扩展事件（Guild Extended Events）
    // ========================================================================

    // 服务器 -> 客户端
    GuildNoticeUpdated { notice: String },
    GuildMemberUpdated { name: String, rank: u8, online: bool },
    GuildExpGained { amount: i64 },
    GuildNameReceived { name: String },
    GuildStorageGoldChanged { delta: i64, total: u32 },
    GuildStorageItemChanged { change_type: u8, slot: i32 },
    GuildStorageListReceived { items: Vec<Option<mir2_shared::data::client_data::GuildStorageItem>> },
    GuildWarRequested { guild_name: String },
    GuildBuffListReceived { buff_ids: Vec<i32> },
    GuildTerritoryPageReceived { territories: Vec<mir2_shared::packets::server::special_systems::TerritoryInfo> },
    GuildTerritoryPurchased { success: bool },

    // 客户端 -> 服务器
    EditGuildMember { member_name: String, rank: u8 },
    EditGuildNotice { notice: String },
    GuildNameReturn { name: String },
    RequestGuildInfo,
    GuildStorageGoldChange { amount: i64 },
    GuildStorageItemChangeRequest,
    GuildWarReturn,
    GuildBuffUpdate { buff_id: u32, action: u8 },
    GuildTerritoryPageRequest { page: i32 },
    PurchaseGuildTerritoryRequest { owner: String },

    // --- Warehouse password (PR #1169) — 客户端发起 ---
    UnlockStorageRequest { password: String },
    SetStoragePasswordRequest { current_password: String, new_password: String },
    RemoveStoragePasswordRequest { current_password: String },

    // --- Warehouse password (PR #1169) — 服务器回应 ---
    StorageUnlockResultReceived { result: u8, has_password: bool },
    StoragePasswordResultReceived {
        result: u8,
        removing: bool,
        has_password: bool,
        last_set_time: i64,
    },

    // --- Detailed info request (PR #1126) — 客户端发起 ---
    RequestMonsterInfoEvent { monster_index: i32 },
    RequestNPCInfoEvent { npc_index: i32 },
    RequestItemInfoEvent { item_index: i32 },

    // --- Detailed info reply (PR #1126) — 服务器回应 ---
    NewMonsterInfoReceived {
        info: mir2_shared::data::client_data::ClientMonsterInfo,
    },
    NewNPCInfoReceived {
        info: mir2_shared::data::client_data::ClientNPCInfo,
    },

    // 攻击/玩家模式
    ChangeAModeRequest { mode: mir2_shared::enums::AttackMode },
    ChangePModeRequest { mode: mir2_shared::enums::PetMode },
    ChangeTradeToggle,

    // 精炼系统
    DepositRefineItemRequest { from: i32, to: i32 },
    RetrieveRefineItemRequest { from: i32, to: i32 },
    RefineCancelRequest,
    RefineItemRequest { unique_id: u64 },
    DepositTradeItemRequest { from: i32, to: i32 },
    RetrieveTradeItemRequest { from: i32, to: i32 },

    // 英雄物品
    TakeBackHeroItemRequest { from: i32, to: i32 },
    TransferHeroItemRequest { from: i32, to: i32 },

    // 组队
    SwitchGroupRequest { allow: bool },

    // 魔法/战斗
    SpellToggleRequest { spell: mir2_shared::enums::Spell, can_use: bool },
    TownReviveRequest,

    // 社交
    RequestUserNameQuery { user_id: u32 },
    RequestChatItemQuery { chat_item_id: u64 },

    // 物品觉醒
    AwakeningNeedMaterialsRequest { unique_id: u64, awake_type: mir2_shared::enums::AwakeType },
    AwakeningLockedItemRequest { unique_id: u64, locked: bool },
    AwakeningRequest { unique_id: u64, awake_type: mir2_shared::enums::AwakeType, position_idx: u32 },
    DisassembleItemRequest { unique_id: u64 },
    DowngradeAwakeningRequest { unique_id: u64 },
    ResetAddedItemRequest { unique_id: u64 },

    // 邮件
    MailLockedItemRequest { unique_id: u64, locked: bool },
    MailCostRequest { gold: u32, items_idx: [u64; 5], stamped: bool },

    // 物品租赁
    ItemRentalRequestEvent,
    ItemRentalFeeRequest { amount: u32 },
    ItemRentalPeriodRequest { days: u32 },
    ItemRentalLockFeeEvent,
    ItemRentalLockItemEvent,

    // ========================================================================
    // NPC 扩展事件（NPC Extended Events）
    // ========================================================================

    // 服务器 -> 客户端
    NPCSellReceived,
    NPCRepairReceived { rate: f32 },
    NPCSRepairReceived { rate: f32 },
    NPCRefineReceived { rate: f32, refining: bool },
    NPCCheckRefineReceived,
    NPCCollectRefineReceived { success: bool },
    NPCReplaceWedRingReceived { rate: f32 },
    NPCStorageReceived,
    NPCConsignReceived,
    SellItemReceived { unique_id: u64, count: u16, success: bool },
    CraftItemReceived { unique_id: u64, count: u16, success: bool },
    NewRecipeInfoReceived { recipe_id: i32 },
    RepairItemReceived { unique_id: u64 },
    ItemRepairedEvent { unique_id: u64, max_dura: u16, current_dura: u16 },
    DefaultNPCReceived { npc_id: u32, message: String },
    NPCUpdated { npc_id: u32 },
    NPCImageUpdated { npc_id: u32, image: u16 },
    NPCAwakeningReceived,
    NPCDisassembleReceived,
    NPCDowngradeReceived,
    NPCResetReceived,
    AwakeningNeedMaterialsReceived { item_id: i32, materials: Vec<(i32, i32)> },
    AwakeningLockedItemReceived { unique_id: u64, locked: bool },
    AwakeningReceived { result: i32, remove_id: i64 },
    NPCPearlGoodsReceived { rate: i32, item_list: Vec<i32> },
    NPCRequestInputReceived { npc_id: u32, prompt: String, max_length: u8 },

    // 客户端 -> 服务器
    LogOutRequest,
    HarvestRequest,
    BuyItemBackRequest,
    SRepairItemRequest { unique_id: u64 },
    CheckRefineRequest,
    ReplaceWedRingRequest,
    NPCConfirmInput { npc_id: u32, input: String },

    // ========================================================================
    // 英雄事件（Hero Events）
    // ========================================================================

    // 服务器 -> 客户端
    HeroCreateRequested { can_create_class: Vec<bool> },
    NewHeroCreated { result: u8 },
    HeroInfoReceived { hero_id: u32 },
    HeroSpawnStateUpdated { state: u8 },
    HeroAutoPotUnlocked { unlocked: bool },
    HeroAutoPotSet { pot_type: u8, value: u32 },
    HeroAutoPotItemSet { slot: i32, item_id: u32 },
    HeroBehaviourSet { behaviour: u8 },
    HeroManageReceived { heroes: Vec<mir2_shared::data::client_data::ClientHeroInformation> },
    HeroChanged { success: bool },
    HeroBaseStatsReceived { stats: Vec<i32> },
    NewHeroInfoReceived { info: String },
    HeroExperienceGained { amount: i64 },
    HeroLevelUp { new_level: u16 },

    // 客户端 -> 服务器
    CreateHeroRequest { name: String },
    SetHeroAutoPotValue { pot_type: u8, value: u32 },
    SetHeroAutoPotItem { item_id: u32 },
    SetHeroBehaviourRequest { behaviour: u8 },
    ChangeHeroRequest { hero_index: u8 },

    // ========================================================================
    // 邮件事件（Mail Events）
    // ========================================================================

    // 服务器 -> 客户端
    MailReceived { mails: Vec<mir2_shared::packets::server::MailInfo> },
    MailLockedItemReceived { unique_id: u64, locked: bool },
    MailSendRequestReceived { mail_id: u64 },
    MailSentEvent { result: i8 },
    ParcelCollectedEvent { success: bool },
    MailCostReceived { cost: u32 },

    // 客户端 -> 服务器
    SendMailRequest { to: String, subject: String, body: String },
    ReadMailRequest { mail_id: u64 },
    CollectParcelRequest { mail_id: u64 },
    DeleteMailRequest { mail_id: u64 },
    LockMailRequest { mail_id: u64 },

    // ========================================================================
    // 市场/寄售事件（Market Events）
    // ========================================================================

    // 服务器 -> 客户端
    NPCConsignEvent,
    NPCMarketEvent2 { pages: Vec<String> },
    NPCMarketPageEvent2 { listings: Vec<mir2_shared::packets::server::MarketListing> },
    ConsignItemEvent { unique_id: u64, success: bool },
    MarketFailedEvent2 { reason: String },
    MarketSuccessEvent2 { message: String },

    // 客户端 -> 服务器
    ConsignItemRequest { item_id: u64, price: u64 },
    MarketSearchRequest { query: String },
    MarketRefreshRequest,
    MarketPageRequest { page: u32 },
    MarketBuyRequest { listing_id: u64 },
    MarketGetBackRequest { listing_id: u64 },
    MarketSellNowRequest { item_id: u64 },

    // ========================================================================
    // 智能宠物事件（Intelligent Creature Events）
    // ========================================================================

    // 服务器 -> 客户端
    NewIntelligentCreatureReceived { creature_type: u8 },
    IntelligentCreatureListUpdated { creatures: Vec<mir2_shared::packets::server::special_systems::IntelligentCreatureInfo> },
    IntelligentCreatureRenameEnabled { can_rename: bool },
    IntelligentCreaturePickupReceived { enabled: bool },

    // 客户端 -> 服务器
    UpdateIntelligentCreatureRequest,
    IntelligentCreaturePickupRequest,
    RequestIntelligentCreatureUpdates,

    // ========================================================================
    // 婚姻/师徒事件（Social Events）
    // ========================================================================

    // 服务器 -> 客户端
    MarriageRequested2 { requester: String },
    DivorceRequested2 { lover_name: String },
    MentorRequested2 { mentor_name: String },
    LoverUpdated { lover_name: String, date: i64 },
    MentorUpdated { mentor_name: String, mentor_level: i32, mentor_online: bool },

    // 客户端 -> 服务器
    MarriageRequestSend { target: String },
    MarriageReply { accept: bool },
    ChangeMarriageRequest,
    DivorceRequestSend,
    DivorceReply { accept: bool },
    AddMentorRequest { name: String },
    MentorReply { accept: bool },
    AllowMentorRequest { enabled: bool },
    CancelMentorRequest,

    // ========================================================================
    // 物品租赁事件（Item Rental Events）
    // ========================================================================

    // 服务器 -> 客户端
    RentalItemsReceived { items: Vec<mir2_shared::packets::server::rental_system::RentalItemInfo> },
    ItemRentalRequested,
    ItemRentalFeeReceived { fee: u32 },
    ItemRentalPeriodReceived { period: u32 },
    RentalItemDeposited { unique_id: u64, success: bool },
    RentalItemRetrieved { unique_id: u64, success: bool },
    RentalItemUpdated { fee: u32, period: i32 },
    ItemRentalCancelled { success: bool },
    ItemRentalLocked { locked: bool },
    ItemRentalPartnerLocked { locked: bool },
    ItemRentalConfirmable { can_confirm: bool },
    ItemRentalConfirmed { success: bool },

    // 客户端 -> 服务器
    GetRentedItemsRequest,
    RentalItemDepositRequest { from_slot: i32, to_slot: i32 },
    RentalItemRetrieveRequest { from_slot: i32, to_slot: i32 },
    ItemRentalConfirm,
    ItemRentalCancel,
    CraftItemRequest { recipe_unique_id: u64, count: u16, slots: Vec<i32> },

    // ========================================================================
    // 钓鱼事件（Fishing Events）
    // ========================================================================

    // 服务器 -> 客户端
    FishingStatusUpdated { state: u8, success: bool },

    // 客户端 -> 服务器
    FishingCastRequest,
    FishingAutocastToggle { enabled: bool },

    // ========================================================================
    // 转生事件（Reincarnation Events）
    // ========================================================================

    // 服务器 -> 客户端
    ReincarnationRequested,
    ReincarnationCancelled,
    HeroHealthChanged { hp: i32, mp: i32 },
    LogOutSuccess { characters: Vec<mir2_shared::SelectInfo> },
    LogOutFailed,
    ReturnToLogin,

    // 客户端 -> 服务器
    AcceptReincarnationRequest,
    CancelReincarnationRequest,

    // ========================================================================
    // 排名/游戏商店事件（Ranking & Game Shop Events）
    // ========================================================================

    // 服务器 -> 客户端
    RankingsReceived { rankings: Vec<mir2_shared::packets::server::special_systems::RankInfo> },
    /// 排行榜数据（Mock 模式用，携带实际排行数据）
    RankingsReceivedWithEntries {
        tab: u8, // 0=Level, 1=Gold, 2=Reputation
        entries: Vec<(u32, String, String)>, // (rank, name, value)
    },
    GameShopInfoReceived { items: Vec<mir2_shared::packets::server::GameShopItem>, credit: u32, gold: u32 },
    GameShopStockReceived { item_index: i32, stock: i32 },

    // 客户端 -> 服务器
    GameShopBuyRequest { item_id: u32, count: u32 },
    ReportIssueRequest { issue: String },
    GetRankingRequest { ranking_type: u8 },

    // ========================================================================
    // 计时器/UI 事件（Timer & UI Events）
    // ========================================================================

    // 服务器 -> 客户端
    TimerSet { timer_id: u8, seconds: u32 },
    TimerExpired { timer_id: u8 },
    NoticeUpdated { notice: String },
    RollReceivedEvent { value: u32 },
    CompassUpdated { location: (i32, i32) },
    BrowserOpened { url: String },

    // ========================================================================
    // 门事件（Door Events）
    // ========================================================================

    // 服务器 -> 客户端 / 客户端 -> 服务器
    DoorOpened { door_id: u32, close: bool },

    // 客户端 -> 服务器
    OpenDoorRequest { door_id: u32 },
    RequestMapInfoRequest,
    TeleportToNPCRequest { npc_name: String },
    SearchMapRequest { query: String },
    ObserveRequest { target: String },

    // ========================================================================
    // 杂项事件（Misc Events）
    // ========================================================================

    // 服务器 -> 客户端
    DuraChanged { unique_id: u64, durability: i32 },
    PlayerPoisoned { object_id: u32, poison_type: u8 },
    ObjectPoisonedEvent { object_id: u32, poison_type: u8 },
    RangeAttacked { target_id: u32, target_x: u32, target_y: u32, spell: u16, spell_level: u16 },
    ObjectRangeAttacked { object_id: u32, location_x: u32, location_y: u32, direction: u8, target_id: u32, target_x: u32, target_y: u32, spell: u16, spell_level: u16 },
    PushedEvent { object_id: u32, x: i32, y: i32, direction: u8 },
    ObjectPushedEvent { object_id: u32, x: i32, y: i32, direction: u8 },
    UserDashAttacked { x: i32, y: i32, direction: u8 },
    ObjectDashAttacked { object_id: u32, location_x: i32, location_y: i32, direction: mir2_shared::enums::MirDirection, distance: i32 },
    UserAttackMoved { x: i32, y: i32 },
    PlayerRevived,
    ObjectRevivedEvent { object_id: u32, effect: u8 },
    ObjectLeveled { object_id: u32, level: u16 },
    TrapRockEntered { in_trap: bool },
    BaseStatsReceived { stats: Vec<i32> },
    InventoryResized { new_size: u32 },
    StorageResized { new_size: u32 },
    TransformUpdated { form: u8 },
    MapEffectReceived { effect: u8, location_x: i32, location_y: i32, value: i32 },
    ObserveAllowed { allowed: bool },
    ObjectSpellReceived { object_id: u32, location_x: i32, location_y: i32, spell: mir2_shared::enums::Spell },
    ObjectDecoReceived { object_id: u32, deco: u16, remove: bool },
    ObjectSneakingReceived { object_id: u32, sneaking: bool },
    ObjectLevelEffectsReceived { object_id: u32, level_effects: u16 },
    BindingShotSet { enabled: bool },
    OutputMessageReceived { message: String, message_type: u8 },
    UserStorageReceived { items: Vec<mir2_shared::data::item::UserItem> },
    ChatItemStatsReceived { unique_id: u64, stats: String },
    ConcentrationSet { object_id: u32, enabled: bool, interrupted: bool },
    ElementalSet { object_id: u32, enabled: bool, value: u32, element: u8, expire_time: i64 },
    DelayedExplosionRemoved { object_id: u32 },

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
