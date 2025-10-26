// ============================================================================
// Dialogs 模块 - 游戏对话框组件
// ============================================================================

pub mod main_dialog;
pub mod inventory_dialog;
pub mod character_dialog;
pub mod skillbar_dialog;
pub mod chat_dialog;
pub mod magic_learning_dialog;
pub mod quest_dialog;
pub mod trade_dialog;
pub mod skills_dialog;
pub mod minimap_dialog;
pub mod options_dialog;
pub mod friends_dialog;
pub mod group_dialog;
pub mod guild_dialog;
pub mod buff_dialog;

// 重新导出所有 Dialog 类型
pub use main_dialog::{MainDialog, MainDialogButton};
pub use inventory_dialog::{InventoryDialog, InventoryAction};
pub use character_dialog::{CharacterDialog, CharacterAction, CharacterTab, EquipmentSlot};
pub use skillbar_dialog::{SkillBarDialog, SkillBarAction};
pub use chat_dialog::{ChatDialog, ChatType};
pub use magic_learning_dialog::{MagicLearningDialog, MagicLearningAction};
pub use quest_dialog::{QuestDialog, QuestAction, QuestViewMode};
pub use trade_dialog::{TradeDialog, TradeAction};
pub use skills_dialog::{SkillsDialog, SkillSlot};
pub use minimap_dialog::MiniMapDialog;
pub use options_dialog::{OptionsDialog, OptionsTab};
pub use friends_dialog::{FriendsDialog, FriendsTab, FriendInfo};
pub use group_dialog::{GroupDialog, GroupMember};
pub use guild_dialog::{GuildDialog, GuildTab, GuildMember};
pub use buff_dialog::{BuffDialog, BuffItem};
