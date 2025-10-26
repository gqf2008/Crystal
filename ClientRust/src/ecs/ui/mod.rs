// ============================================================================
// UI 模块 - 游戏界面组件
// ============================================================================

pub mod components;        // 基础UI组件
pub mod button_widget;     // 按钮部件辅助结构
pub mod dialog_manager;    // 对话框管理器
pub mod dialogs;           // 所有对话框组件
pub mod hotkey_help;       // 按键帮助面板

// 重新导出常用类型
pub use components::*;     // 基础UI组件(HealthBar, ManaBar等)
pub use button_widget::{ButtonWidget, ButtonGroup, ButtonState};
pub use dialog_manager::{DialogManager, DialogType};
pub use dialogs::*;        // 所有Dialog类型
pub use hotkey_help::HotkeyHelpPanel;
