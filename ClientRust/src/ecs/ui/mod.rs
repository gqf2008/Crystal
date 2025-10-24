// ============================================================================
// UI 模块 - 游戏界面组件
// ============================================================================

pub mod components;
pub mod ui_renderer;
pub mod button_widget;     // 🆕 按钮部件辅助结构
pub mod dialog_manager;    // 🆕 对话框管理器
pub mod main_dialog;       // 🆕 游戏主界面
pub mod inventory_dialog;  // 🆕 背包对话框
pub mod character_dialog;  // 🆕 角色对话框
pub mod skillbar_dialog;   // 🆕 技能栏对话框
pub mod chat_dialog;       // 🆕 聊天对话框
pub mod magic_learning_dialog; // 🆕 技能学习对话框
pub mod quest_dialog;      // 🆕 任务对话框
pub mod trade_dialog;      // 🆕 交易窗口
pub mod skills_dialog;     // 🆕 技能对话框
pub mod minimap_dialog;    // 🆕 小地图对话框
pub mod options_dialog;    // 🆕 选项对话框
pub mod friends_dialog;    // 🆕 好友对话框
pub mod group_dialog;      // 🆕 组队对话框
pub mod guild_dialog;      // 🆕 行会对话框

// 重新导出常用类型
pub use components::*;
pub use ui_renderer::UIRenderer;
pub use button_widget::{ButtonWidget, ButtonGroup, ButtonState};
pub use dialog_manager::{DialogManager, DialogType};
pub use main_dialog::{MainDialog, MainDialogButton};
pub use inventory_dialog::{InventoryDialog, InventoryAction};
pub use character_dialog::{CharacterDialog, CharacterAction, CharacterTab, EquipmentSlot};
pub use skillbar_dialog::{SkillBarDialog, SkillBarAction};
pub use chat_dialog::{ChatDialog, ChatType};
pub use magic_learning_dialog::{MagicLearningDialog, MagicLearningAction};
pub use quest_dialog::{QuestDialogComp, QuestAction, QuestViewMode};
pub use trade_dialog::{TradeDialogComp, TradeAction};
pub use skills_dialog::{SkillsDialog, SkillsDialogComp, SkillSlot};
pub use minimap_dialog::{MiniMapDialog, MiniMapDialogComp};
pub use options_dialog::{OptionsDialog, OptionsDialogComp, OptionsTab};
pub use friends_dialog::{FriendsDialog, FriendsDialogComp, FriendsTab, FriendInfo};
pub use group_dialog::{GroupDialog, GroupDialogComp, GroupMember};
pub use guild_dialog::{GuildDialog, GuildDialogComp, GuildTab, GuildMember};
