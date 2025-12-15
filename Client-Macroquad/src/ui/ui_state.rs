use std::sync::{RwLock, RwLockReadGuard, RwLockWriteGuard};

use macroquad::prelude::Vec2;

use crate::scenes::dialogs::game::{
    amount_box::AmountBoxResult,
    npc_dialog::NpcDialogAction,
    npc_goods_dialog::NpcGoodsDialogAction,
};

use mir2_shared::data::item::UserItem;
use mir2_shared::enums::PanelType;

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

    /// UI -> ECS：小地图点击产生的自动寻路目标（世界坐标像素 + run 标记）。
    pub pending_auto_path_target: Option<(f32, f32, bool)>,

    /// AmountBox 确认购买时需要的 uid（由表现层在打开时设置，逻辑层在确认后消费）。
    pub amount_box_buy_uid: Option<u64>,
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
