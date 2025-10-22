// ============================================================================
// UI 模块 - 游戏界面组件
// ============================================================================

pub mod components;
pub mod ui_renderer;
pub mod button_widget;     // 🆕 按钮部件辅助结构
pub mod main_dialog;       // 🆕 游戏主界面
pub mod inventory_dialog;  // 🆕 背包对话框
pub mod character_dialog;  // 🆕 角色对话框

// 重新导出常用类型
pub use components::*;
pub use ui_renderer::UIRenderer;
pub use button_widget::{ButtonWidget, ButtonGroup, ButtonState};
pub use main_dialog::{MainDialog, MainDialogButton};
pub use inventory_dialog::{InventoryDialog, InventoryAction};
pub use character_dialog::{CharacterDialog, CharacterAction, CharacterTab, EquipmentSlot};
