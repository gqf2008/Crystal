use std::sync::{RwLock, RwLockReadGuard, RwLockWriteGuard};

use macroquad::prelude::Vec2;

use crate::scenes::dialogs::game::{
    amount_box::AmountBoxResult,
    npc_dialog::NpcDialogAction,
    npc_goods_dialog::NpcGoodsDialogAction,
};

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

    /// 更新坐骑状态（mount_type, riding）
    UpdateMountState { mount_type: i16, riding: bool },

    /// 更新英雄行为模式
    UpdateHeroBehaviour { behaviour: u8 },

    /// 系统提示：英雄相关
    PushHeroSystemChat(String),

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
    SetHeroAutoPotItem { item_id: u32 },

    /// Buff 暂停/恢复
    SetBuffPaused { buff_id: u32, paused: bool },

    /// 更新罗盘位置
    UpdateCompass { location: (i32, i32) },

    /// 交易相关：打开/更新交易对话框
    OpenTradeDialog { partner: String },
    TradeGoldAdded { amount: u32 },
    TradeItemAdded,
    TradeConfirmed { locked: bool },
    TradeCancelled,

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
    GuildWarRequested,
    SetGuildName { name: String },

    /// 组队成员列表更新
    UpdateGroupMembers { members: Vec<crate::scenes::dialogs::game::group_dialog::GroupMember> },
    SetGroupAllowJoin { allow: bool },

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

    /// 更新邮件列表
    UpdateMailList { mails: Vec<crate::ui::ui_state::MailEntry> },
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

    /// 邮件：待显示的邮件列表
    pub mail_entries: Vec<MailEntry>,

    /// UI -> ECS：小地图点击产生的自动寻路目标（世界坐标像素 + run 标记）。
    pub pending_auto_path_target: Option<(f32, f32, bool)>,

    /// AmountBox 确认购买时需要的 uid（由表现层在打开时设置，逻辑层在确认后消费）。
    pub amount_box_buy_uid: Option<u64>,
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
}
