// Main Dialog Module - 主对话框模块
// 对应C#的MainDialogs.cs文件，包含多个主UI对话框

pub mod main_dialog;
pub mod chat_dialog;
pub mod chat_control_bar;
pub mod skill_bar_dialog;
pub mod mini_map_dialog;
pub mod inspect_dialog;
pub mod option_dialog;
pub mod menu_dialog;
pub mod magic_button;
pub mod assign_key_panel;
pub mod dura_status_dialog;
pub mod character_dura_panel;

// Re-exports
pub use main_dialog::MainDialog;
pub use chat_dialog::ChatDialog;
pub use chat_control_bar::ChatControlBar;
pub use skill_bar_dialog::SkillBarDialog;
pub use mini_map_dialog::MiniMapDialog;
pub use inspect_dialog::InspectDialog;
pub use option_dialog::OptionDialog;
pub use menu_dialog::MenuDialog;
pub use magic_button::MagicButton;
pub use assign_key_panel::AssignKeyPanel;
pub use dura_status_dialog::DuraStatusDialog;
pub use character_dura_panel::CharacterDuraPanel;