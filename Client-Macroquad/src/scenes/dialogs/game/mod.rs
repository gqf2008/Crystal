// Game dialogs module - 纯 Native 版本 (无 egui)

pub mod native_ui_utils;
// pub mod belt_dialog_native;
pub mod belt_dialog;
pub mod character_dialog;
pub mod chat_control_bar;
pub mod chat_dialog;
pub mod chat_option_dialog;
pub mod amount_box;
pub mod game_shop_dialog; 
pub mod npc_dialog;
pub mod npc_goods_dialog;
pub mod inventory_dialog;
pub mod main_dialog;
pub mod menu_dialog;
pub mod minimap_dialog;
pub mod option_dialog;
pub mod quest_log_dialog;

// 导出 hybrid 版本作为主要实现
//pub use belt_dialog_native::{BeltDialogNative, BeltLayout, BeltItem};
pub use belt_dialog::{BeltDialogHybrid, BeltItemHybrid, BeltLayoutHybrid};
pub use character_dialog::{CharacterDialogHybrid, CharacterTabHybrid, EquipmentItemHybrid, EquipSlot};
pub use chat_control_bar::{ChatControlBarHybrid, ChatFilterHybrid};
pub use chat_dialog::ChatDialogHybrid;
pub use chat_dialog::ChatMessageKind;
pub use chat_option_dialog::{ChatOptionDialogHybrid, ChatOptionSettingsHybrid};
pub use amount_box::{AmountBoxHybrid, AmountBoxResult};
pub use game_shop_dialog::{GameShopDialogHybrid, ShopSectionHybrid, ShopClassHybrid, ShopCategoryHybrid, ShopItemHybrid};
pub use npc_dialog::{NpcDialogHybrid, NpcDialogAction};
pub use npc_goods_dialog::NpcGoodsDialogHybrid;
pub use inventory_dialog::{InventoryDialogHybrid, ItemSlotHybrid, InventoryTabHybrid};
pub use main_dialog::MainDialog;
pub use menu_dialog::{MenuDialogHybrid, MenuAction};
pub use minimap_dialog::MiniMapDialogHybrid;
pub use option_dialog::OptionDialogHybrid;
pub use quest_log_dialog::QuestLogDialogHybrid;

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
