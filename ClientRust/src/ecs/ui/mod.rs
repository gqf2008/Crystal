// ============================================================================
// UI 模块 - 游戏界面组件
// ============================================================================

pub mod components;
pub mod ui_renderer;
pub mod button_widget;     // 🆕 按钮部件辅助结构
pub mod main_dialog;       // 🆕 游戏主界面
pub mod inventory_dialog;  // 🆕 背包对话框
pub mod character_dialog;  // 🆕 角色对话框
pub mod skillbar_dialog;   // 🆕 技能栏对话框
pub mod chat_dialog;       // 🆕 聊天对话框
pub mod magic_learning_dialog; // 🆕 技能学习对话框
pub mod quest_dialog;      // 🆕 任务对话框
pub mod trade_dialog;      // 🆕 交易窗口

// 重新导出常用类型
pub use components::*;
pub use ui_renderer::UIRenderer;
pub use button_widget::{ButtonWidget, ButtonGroup, ButtonState};
pub use main_dialog::{MainDialog, MainDialogButton};
pub use inventory_dialog::{InventoryDialog, InventoryAction};
pub use character_dialog::{CharacterDialog, CharacterAction, CharacterTab, EquipmentSlot};
pub use skillbar_dialog::{SkillBarDialog, SkillBarAction};
pub use chat_dialog::{ChatDialog, ChatType};
pub use magic_learning_dialog::{MagicLearningDialog, MagicLearningAction};
pub use quest_dialog::{QuestDialogComp, QuestAction, QuestViewMode};
pub use trade_dialog::{TradeDialogComp, TradeAction};
