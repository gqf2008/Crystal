// Hero Dialog Module - 英雄对话框模块
// 对应C#的HeroDialogs.cs文件，包含多个英雄相关对话框

pub mod hero_inventory_dialog;
pub mod hero_belt_dialog;
pub mod hero_menu_panel;
pub mod hero_info_panel;
pub mod hero_auto_pot_preview;
pub mod hero_behaviour_panel;
pub mod hero_manage_dialog;
pub mod hero_manage_avatar;

// Re-exports
pub use hero_inventory_dialog::HeroInventoryDialog;
pub use hero_belt_dialog::HeroBeltDialog;
pub use hero_menu_panel::HeroMenuPanel;
pub use hero_info_panel::HeroInfoPanel;
pub use hero_auto_pot_preview::HeroAutoPotPreview;
pub use hero_behaviour_panel::HeroBehaviourPanel;
pub use hero_manage_dialog::HeroManageDialog;
pub use hero_manage_avatar::HeroManageAvatar;