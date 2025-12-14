// Game dialogs module - 纯 Native 版本 (无 egui)

pub mod native_ui_utils;
pub mod belt_dialog_native;
pub mod belt_dialog_mqui;
pub mod belt_dialog_hybrid;
pub mod character_dialog_hybrid;
pub mod chat_control_bar_hybrid;
pub mod chat_dialog_hybrid;
pub mod chat_option_dialog_hybrid;
pub mod amount_box_hybrid;
pub mod game_shop_dialog_hybrid;  // 原版 hybrid 版本
pub mod npc_dialog_hybrid;
pub mod npc_goods_dialog_hybrid;
pub mod inventory_dialog_hybrid;
// pub mod inventory_persistence; // 暂时禁用，需要为 hybrid 类型添加 serde 支持
pub mod main_dialog;
pub mod menu_dialog_hybrid;
pub mod minimap_dialog_hybrid;
pub mod option_dialog_hybrid;
pub mod quest_log_dialog_hybrid;

// 导出 hybrid 版本作为主要实现
pub use belt_dialog_native::{BeltDialogNative, BeltLayout, BeltItem};
pub use belt_dialog_mqui::{BeltDialogMqui, BeltItem as BeltItemMqui, BeltLayout as BeltLayoutMqui};
pub use belt_dialog_hybrid::{BeltDialogHybrid, BeltItemHybrid, BeltLayoutHybrid};
pub use character_dialog_hybrid::{CharacterDialogHybrid, CharacterTabHybrid, EquipmentItemHybrid, EquipSlot};
pub use chat_control_bar_hybrid::{ChatControlBarHybrid, ChatFilterHybrid};
pub use chat_dialog_hybrid::ChatDialogHybrid;
pub use chat_dialog_hybrid::ChatMessageKind;
pub use chat_option_dialog_hybrid::{ChatOptionDialogHybrid, ChatOptionSettingsHybrid};
pub use amount_box_hybrid::{AmountBoxHybrid, AmountBoxResult};
pub use game_shop_dialog_hybrid::{GameShopDialogHybrid, ShopSectionHybrid, ShopClassHybrid, ShopCategoryHybrid, ShopItemHybrid};
pub use npc_dialog_hybrid::{NpcDialogHybrid, NpcDialogAction};
pub use npc_goods_dialog_hybrid::NpcGoodsDialogHybrid;
pub use inventory_dialog_hybrid::{InventoryDialogHybrid, ItemSlotHybrid, InventoryTabHybrid};
pub use main_dialog::MainDialog;
pub use menu_dialog_hybrid::{MenuDialogHybrid, MenuAction};
pub use minimap_dialog_hybrid::MiniMapDialogHybrid;
pub use option_dialog_hybrid::OptionDialogHybrid;
pub use quest_log_dialog_hybrid::QuestLogDialogHybrid;

// 为了兼容性，创建别名
pub type BeltDialog = BeltDialogHybrid;
pub type CharacterDialog = CharacterDialogHybrid;
pub type ChatControlBar = ChatControlBarHybrid;
pub type ChatDialog = ChatDialogHybrid;
pub type ChatOptionDialog = ChatOptionDialogHybrid;
pub type GameShopDialog = GameShopDialogHybrid;
pub type InventoryDialog = InventoryDialogHybrid;
pub type MenuDialog = MenuDialogHybrid;
pub type MiniMapDialog = MiniMapDialogHybrid;
pub type OptionDialog = OptionDialogHybrid;
pub type QuestLogDialog = QuestLogDialogHybrid;
