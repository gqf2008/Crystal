// NPC Dialog Module - NPC对话框模块
// 对应C#的NPCDialogs.cs文件，包含多个NPC相关对话框

pub mod npc_dialog;
pub mod npc_goods_dialog;
pub mod npc_drop_dialog;
pub mod npc_awake_dialog;
pub mod craft_dialog;
pub mod refine_dialog;
pub mod storage_dialog;
pub mod big_button_dialog;
pub mod big_button;

// Re-exports
pub use npc_dialog::{NPCDialog, NPCDialogType, NPCOption, NPCPage};
pub use npc_goods_dialog::NPCGoodsDialog;
pub use npc_drop_dialog::NPCDropDialog;
pub use npc_awake_dialog::NPCAwakeDialog;
pub use craft_dialog::CraftDialog;
pub use refine_dialog::RefineDialog;
pub use storage_dialog::{StorageDialog, StorageType};
pub use big_button_dialog::BigButtonDialog;
pub use big_button::BigButton;