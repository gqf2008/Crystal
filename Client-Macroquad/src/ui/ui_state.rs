use std::collections::HashMap;
use std::sync::{RwLock, RwLockReadGuard, RwLockWriteGuard};

use macroquad::prelude::Vec2;

use crate::scenes::dialogs::game::{
    amount_box::AmountBoxResult,
    npc_dialog::NpcDialogAction,
    npc_goods_dialog::NpcGoodsDialogAction,
};

use mir2_shared::data::client_data::{ClientMonsterInfo, ClientNPCInfo};
use mir2_shared::data::item::UserItem;
use mir2_shared::enums::PanelType;

/// 好友条目（从服务器 FriendUpdate 转换而来）
#[derive(Debug, Clone)]
pub struct FriendEntry {
    pub object_id: u32,
    pub name: String,
    pub memo: String,
    pub online: bool,
}

/// 邀请类型
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum InviteKind {
    Group,
    Guild,
    Trade,
    Mentor,
    Divorce,
}

#[derive(Debug, Clone)]
pub enum UiAction {
    NpcDialog(NpcDialogAction),
    NpcGoods(NpcGoodsDialogAction),
    NpcSubGoods(NpcGoodsDialogAction),
    AmountBox(AmountBoxResult),
}

#[derive(Debug, Clone)]
pub enum UiCommand {
    CloseNpcRelatedDialogs,
    CloseAllPopups,
    OpenInventory,
    ActivateChatInput,
    ToggleMinimap,
    ToggleMinimapSize,

    PushSystemChatLine(String),
    PushChatLine(String),
    PushWhisperLine(String),

    ShowNpcDialog { dialog: String },
    ShowNpcGoods {
        items: Vec<UserItem>,
        rate: f32,
        panel_type: PanelType,
        hide_added_stats: bool,
        is_sub: bool,
        use_pearls: bool,
    },

    ShowAmountBox {
        title: String,
        image_index: u16,
        max_quantity: u32,
        min_quantity: u32,
        default_amount: u32,
        buy_uid: u64,
    },

    HideAmountBox,
    HideNpcGoodsSub,
    HideNpcGoods,

    /// 打开举报 / Bug 反馈对话框
    ShowReport,

    /// 更新坐骑状态（mount_type, riding）
    UpdateMountState { mount_type: i16, riding: bool },

    /// 更新英雄行为模式
    UpdateHeroBehaviour { behaviour: u8 },

    /// 系统提示：英雄相关
    PushHeroSystemChat(String),

    /// 更新英雄 HP/MP
    UpdateHeroHealth { hp: i32, mp: i32 },

    UpdateHeroSpawnState { state: u8 },

    /// 英雄已切换
    HeroChanged,

    /// 玩家升级
    PlayerLevelUp { new_level: u16 },

    /// 更新钓鱼状态
    UpdateFishingState { state: u8, chance: f32, progress: f32 },

    /// 更新钓鱼自动抛竿
    SetFishingAutoCast { enabled: bool },

    /// 更新宠物列表
    UpdateCreatureList { creatures: Vec<crate::scenes::dialogs::game::intelligent_creature_dialog::CreatureEntry> },

    /// 更新好友列表
    UpdateFriendList { friends: Vec<FriendEntry> },

    /// 更新Buff
    AddBuff { buff: crate::scenes::dialogs::game::buff_dialog::BuffEntry },
    RemoveBuff { buff_type: u32 },

    /// 英雄自动喝药
    SetHeroAutoPotUnlocked,
    SetHeroAutoPotValue { pot_type: u8, value: u32 },
    SetHeroAutoPotItem { slot: i32, item_id: u32 },

    /// Buff 暂停/恢复
    SetBuffPaused { buff_id: u32, paused: bool },

    /// 更新罗盘位置
    UpdateCompass { location: (i32, i32) },

    /// 交易相关：打开/更新交易对话框
    OpenTradeDialog { partner: String },
    TradeGoldAdded { amount: u32 },
    TradeItemAdded { items: Vec<Option<mir2_shared::data::item::UserItem>> },
    TradeItemDeposited { from_slot: i32, success: bool },
    TradeItemRetrieved { from_slot: i32, success: bool },
    TradeConfirmed { locked: bool },
    TradeCancelled { unlock: bool },
    TradeCompleted,

    /// 任务相关
    QuestAccepted { quest_id: u32, name: String, description: String },
    QuestCompleted { quest_id: u32 },
    QuestProgressUpdated { quest_id: u32, progress_text: String },
    QuestInfoReceived {
        quest_id: u32,
        name: String,
        group: String,
        description: String,
        level_req: u32,
        reward_exp: u64,
        reward_gold: u32,
    },

    /// 公会相关
    GuildMemberUpdated { name: String, rank: String, online: bool },
    GuildNoticeUpdated { notice: String },
    GuildExpGained { amount: i64 },
    GuildWarRequested { guild_name: String },
    SetGuildName { name: String },
    UpdateGuildStatus {
        rank_name: String,
        level: u8,
        experience: i64,
        max_experience: i64,
        gold: u32,
        spare_points: u8,
        member_count: i32,
        max_members: i32,
        my_rank_id: i32,
    },
    /// 更新行会仓库金币
    UpdateGuildStorageGold { gold: u32 },
    /// 更新行会仓库物品列表
    UpdateGuildStorageItems { items: Vec<crate::scenes::dialogs::game::guild_dialog::GuildStorageItem> },
    /// 更新行会仓库单个物品
    UpdateGuildStorageItem { slot: i32, name: String, quantity: i32 },
    /// 清空行会仓库物品
    ClearGuildStorageItems,
    UpdateGuildBuffs { buff_ids: Vec<i32> },

    /// 组队成员列表更新
    UpdateGroupMembers { members: Vec<crate::scenes::dialogs::game::group_dialog::GroupMember> },
    /// 组队成员地图信息更新
    UpdateGroupMemberMap { player_name: String, player_map: String },
    UpdateGroupMemberLocation { player_name: String, x: i32, y: i32 },
    SetGroupAllowJoin { allow: bool },
    AddGroupMember { name: String },
    RemoveGroupMember { name: String },
    ClearGroupMembers,

    /// 小地图：邮件按钮
    OpenMailDialog,
    CloseMailDialog,

    /// 小地图：大地图按钮
    OpenBigMap,

    /// 仓库对话框
    OpenStorage,
    /// 更新仓库物品列表（左侧面板：仓库物品）
    UpdateStorageItems { items: Vec<mir2_shared::data::item::UserItem> },
    /// 更新仓库对话框中的背包物品列表（右侧面板）
    UpdateStorageInventoryItems { items: Vec<mir2_shared::data::item::UserItem> },

    /// 婚姻/师徒
    SetMarriageRequester { requester: String },
    ClearMarriageRequester,
    UpdateLover { name: String, date: i64 },
    UpdateMentor { name: String, level: i32, online: bool },

    /// 邀请确认弹窗
    ShowInviteConfirm { kind: InviteKind, inviter: String, detail: String },
    HideInviteConfirm,

    /// 通用文本输入对话框（组队邀请/添加好友/拜师）
    ShowTextInput {
        kind: crate::scenes::dialogs::game::main_dialog::TextInputKind,
        title: String,
        placeholder: String,
        max_length: usize,
    },
    HideTextInput,

    /// 更新排行榜数据
    UpdateRankings { tab: u8, entries: Vec<(u32, String, String)> },

    /// 更新商城商品列表（服务器数据）
    UpdateGameShopItems { items: Vec<mir2_shared::packets::server::GameShopItem>, credit: u32, gold: u32 },
    /// 更新商城单个商品库存
    UpdateGameShopStock { item_index: i32, stock: i32 },

    /// 更新邮件列表
    UpdateMailList { mails: Vec<crate::ui::ui_state::MailEntry> },

    /// 更新攻击模式（服务器推送 ChangeAMode）
    UpdateAttackMode { mode: u8 },
    /// 更新宠物模式（服务器推送 ChangePMode）
    UpdatePetMode { mode: u8 },

    /// 服务器倒计时
    SetTimer { timer_id: u8, seconds: u32 },
    TimerExpired { timer_id: u8 },

    /// 屏幕中央 transient 通知
    PushChatNotice { text: String },

    /// 服务器公告
    ShowNotice { text: String },
    CloseNotice,

    /// 骰子结果
    ShowRollResult { value: u32 },

    /// 耐久度状态
    UpdateDuraStatus { items: Vec<crate::scenes::dialogs::game::dura_status_dialog::DuraEntry> },
    ToggleDuraStatus,

    /// NPC 赠送物品
    ShowNPCDrop { npc_name: String, items: Vec<mir2_shared::data::item::UserItem> },

    /// 行会领地
    ShowGuildTerritory,
    UpdateGuildTerritory { entries: Vec<crate::scenes::dialogs::game::guild_territory_dialog::TerritoryEntry>, page: i32, total: i32 },

    /// 键位设置
    ToggleKeyboardLayout,

    /// 装备觉醒
    ShowNPCAwake { item_name: String, materials: Vec<crate::scenes::dialogs::game::npc_awake_dialog::AwakeningMaterial> },
    SetAwakeLocked { locked: bool },

    /// 合成
    ShowCraft { recipes: Vec<crate::scenes::dialogs::game::craft_dialog::CraftRecipe> },

    /// 精炼
    ShowRefine { item_name: String, stats: Vec<crate::scenes::dialogs::game::refine_dialog::RefineStat>, material_name: String, material_have: u32, material_need: u32 },

    /// 物品租赁
    OpenItemRental { partner: String },
    UpdateRentalFee { fee: u32 },
    UpdateRentalPeriod { period: u32 },
    SetRentalLocked { locked: bool },
    SetRentalPartnerLocked { locked: bool },
    CloseItemRental,
    UpdateRentalItemList { items: Vec<mir2_shared::packets::server::rental_system::RentalItemInfo> },

    /// 寄售行
    OpenTrustMerchant,
    UpdateMerchantItems { items: Vec<crate::scenes::dialogs::game::trust_merchant_dialog::MerchantItem>, page: i32, total: i32 },
    CloseTrustMerchant,

    /// 合成：玩家点击合成按钮（含完整材料槽位数据）
    CraftItemRequest { recipe_unique_id: u64, count: u16, slots: Vec<i32> },

    /// 物品租赁：确认交易
    ConfirmItemRental,

    /// 场景切换请求（服务器 LogOutSuccess / ReturnToLogin）
    RequestSceneTransition { target: crate::scenes::SceneTransition },

    /// 英雄升级
    HeroLevelUp { new_level: u16 },

    /// 英雄管理列表更新 (from ManageHeroes packet)
    UpdateHeroManageList { heroes: Vec<crate::scenes::dialogs::game::hero_dialog::ManageHeroEntry> },

    /// 英雄信息接收 (hero_id)
    HeroInfoReceived { hero_id: u32 },

    /// 装备耐久度变化
    ItemDuraChanged { unique_id: u64, current_dura: i32 },
    /// 物品已从背包移除，清理耐久度追踪
    RemoveDuraEntry { unique_id: u64 },

    /// 背包大小调整
    SetInventorySize { size: u32 },
    /// 仓库大小调整
    SetStorageSize { size: u32 },

    /// 时段变更
    SetTimeOfDay { time: u8 },

    /// 技能状态切换
    SetBindingShot { enabled: bool },
    SetConcentration { enabled: bool },
    SetElement { element: u8 },
    SetObserveAllowed { allowed: bool },

    // Hero stats
    SetHeroBaseStats { stats: Vec<i32> },

    // Big map / world map
    UpdateBigMapInfo { map_index: i32, title: String, width: i32, height: i32 },
    UpdateWorldMapIcons { icons: Vec<mir2_shared::packets::server::WorldMapIcon> },
    NavigateToMapLocation { map_index: i32, x: u32, y: u32 },

    // Magic/Spells
    MagicLearned { spell: u8, name: String, level: u8, icon: u8, hero: bool },
    MagicLeveledUp { spell: u8, level: u8, experience: u16 },
    MagicRemoved { spell: u8, hero: bool },
    SpellToggled { spell: u8, can_use: bool, hero: bool },

    // Experience
    ExperienceGained { amount: i64 },
    HeroExperienceGained { amount: i64 },

    // Transform
    SetTransformForm { form: u8 },

    // Sound & effects
    TriggerMapEffect { effect: u8 },

    // Character stats
    SetBaseStats { stats: Vec<i32> },

    // Creature/Pet
    SetCreatureCanRename { can_rename: bool },
    SetCreatureAutoPickup { enabled: bool },

    // Doors
    OpenDoor { door_id: u32 },
}

#[derive(Debug, Clone, Default)]
pub struct UiStateData {
    /// draw 阶段收集的 UI action（在表现层处理：发包/弹窗等）。
    pub pending_actions: Vec<UiAction>,

    /// 渲染层产出的命令（表现层写入，渲染层消费以驱动具体 UI 组件）。
    pub pending_commands: Vec<UiCommand>,

    /// UI 在上一帧（draw 阶段）是否消耗了鼠标事件。
    pub ui_consumed_last_frame: bool,

    /// UI 上是否有输入框激活（聊天输入等）。
    pub ui_input_active: bool,

    /// 是否存在任何弹窗/对话框打开（用于 ESC 退出 gating）。
    pub any_modal_or_popup_open: bool,

    /// UI 是否捕获鼠标（按下拖拽中）。
    pub ui_mouse_captured: bool,

    /// 小地图：世界尺寸（格子数），用于点击反算到世界坐标。
    pub minimap_world_size: Option<Vec2>,

    /// 小地图：玩家指示器（世界坐标像素）。
    pub minimap_player_pos: Option<Vec2>,
    pub minimap_player_dir_radians: f32,

    /// 大地图：当前地图名称
    pub big_map_map_name: Option<String>,

    /// PR #1126: KR NPC link tooltip 缓存
    /// server 发 NewMonsterInfo / NewNPCInfo 时写入;
    /// npc_dialog hover link 时读取以渲染丰富 tooltip。
    /// 缺失的 idx 走"加载中"占位符文本(PR #1126 master 行为)。
    pub monster_info_cache: HashMap<i32, ClientMonsterInfo>,
    /// npc_info_cache 用 object_id (u32) 做 key — server 发的 NewNPCInfo
    /// payload 用 object_id 而非 index。idx 路径 (i32) 在 read helper 内
    /// 先尝试 cast 失败再 fall back to direct lookup。
    pub npc_info_cache: HashMap<u32, ClientNPCInfo>,

    /// 邮件：待显示的邮件列表
    pub mail_entries: Vec<MailEntry>,

    /// UI -> ECS：小地图点击产生的自动寻路目标（世界坐标像素 + run 标记）。
    pub pending_auto_path_target: Option<(f32, f32, bool)>,

    /// AmountBox 确认购买时需要的 uid（由表现层在打开时设置，逻辑层在确认后消费）。
    pub amount_box_buy_uid: Option<u64>,

    /// 商城：服务器商品列表（GameShopInfoReceived 写入，dialog_system 读取同步到 GameShopDialog）。
    pub shop_items: Vec<mir2_shared::packets::server::GameShopItem>,
    pub shop_credit: u32,
    pub shop_gold: u32,

    /// 请求的场景切换（服务器 LogOutSuccess / ReturnToLogin 触发）。
    pub request_scene_transition: Option<crate::scenes::SceneTransition>,

    /// 背包容量（由 InventoryResized 更新）
    pub inventory_size: u32,
    /// 仓库容量（由 StorageResized 更新）
    pub storage_size: u32,

    /// 当前时段 (0=白天, 1=黄昏, etc.)
    pub time_of_day: u8,

    /// 技能/效果状态
    pub binding_shot_enabled: bool,
    pub concentration_enabled: bool,
    pub element_type: u8,
    pub observe_allowed: bool,

    /// 变身形态 (0=normal)
    pub transform_form: u8,

    /// 最近触发的地图特效 (0=none)
    pub pending_map_effect: u8,

    /// 已打开的门 (door_id set)
    pub open_doors: std::collections::HashSet<u32>,
}

/// 邮件条目（从服务器 MailReceived 转换而来）
#[derive(Debug, Clone, Default)]
pub struct MailEntry {
    pub mail_id: u64,
    pub sender: String,
    pub subject: String,
    pub body: String,
    pub date: String,
    pub has_parcel: bool,
    pub is_read: bool,
}

impl UiStateData {
    pub fn new() -> Self {
        Self::default()
    }
}

/// ECS World 单例组件：存放 UI 表现层数据（A/B 类）。
///
/// 说明：RenderSystem::draw 只有 &World，因此这里用 RwLock 提供内部可变性，
/// 允许在 draw 阶段写入 actions/消耗标记。
#[derive(Debug)]
pub struct UiState(pub RwLock<UiStateData>);

impl Default for UiState {
    fn default() -> Self {
        Self::new()
    }
}

impl UiState {
    pub fn new() -> Self {
        Self(RwLock::new(UiStateData::new()))
    }

    pub fn borrow(&self) -> RwLockReadGuard<'_, UiStateData> {
        self.0.read().expect("UiState RwLock poisoned")
    }

    pub fn borrow_mut(&self) -> RwLockWriteGuard<'_, UiStateData> {
        self.0.write().expect("UiState RwLock poisoned")
    }

    /// 在 World 中查找 UiState 并执行可变回调（无则返回 None）。
    pub fn with_mut_in_world<R>(
        world: &mut hecs::World,
        f: impl FnOnce(&mut UiStateData) -> R,
    ) -> Option<R> {
        let mut q = world.query::<&UiState>();
        let s = q.iter().next()?;
        let mut data = s.borrow_mut();
        Some(f(&mut data))
    }

    /// 在 World 中查找 UiState 并执行回调（无则静默跳过）。
    pub fn with_in_world(world: &hecs::World, f: impl FnOnce(&mut UiStateData)) {
        if let Some(s) = world.query::<&UiState>().iter().next() {
            f(&mut s.borrow_mut());
        }
    }

    /// 在 World 中查找 UiState 并执行只读回调 (返回 Option,None 表示 UiState 不存在)
    /// 用于只读访问如 npc_dialog tooltip cache lookup。
    pub fn peek_in_world<R>(world: &hecs::World, f: impl FnOnce(&UiStateData) -> R) -> Option<R> {
        world.query::<&UiState>().iter().next().map(|s| f(&s.borrow()))
    }
}

