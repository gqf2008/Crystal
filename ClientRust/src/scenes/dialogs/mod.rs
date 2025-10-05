// Dialog system modules
// Mirrors Client/MirScenes/Dialogs/

// Core infrastructure
pub mod dialog_manager;

// Main UI Dialogs
pub mod main_dialog;

// Character Management
pub mod hero_dialog;
pub mod mail_dialog;
pub mod trade_dialog;
pub mod quest_dialog;
pub mod intelligent_creature_dialog;

// Management Dialogs (Option E)
pub mod menu_dialog;
pub mod option_dialog;
pub mod keyboard_layout_dialog;
pub mod notice_dialog;
pub mod inspect_dialog;
pub mod report_dialog;

// TODO: Implement remaining dialogs

// Placeholder dialog list (to be implemented):
// - MainDialog (main UI)
// - ChatDialog (chat window)
// - InventoryDialog (inventory/backpack)
// - CharacterDialog (character stats)
// - SkillBarDialog (skill hotbar)
// - MiniMapDialog (minimap)
// - NPCDialog (NPC conversation)
// - TradeDialog (player trading)
// - StorageDialog (storage chest)
// - GuildDialog (guild management)
// - QuestListDialog (quest log)
// - MailListDialog (mail inbox)
// - GameShopDialog (cash shop)
// - GroupDialog (party/group)
// - FriendDialog (friends list)
// - OptionDialog (settings)
// - HelpDialog (help/tutorial)
// - RankingDialog (leaderboards)
// - MenuDialog (game menu)
// - InspectDialog (inspect other players)
// - RefineDialog (item refinement)
// - SocketDialog (gem socketing)
// - MountDialog (mount system)
// - FishingDialog (fishing minigame)
// - CraftDialog (crafting system)
// - BeltDialog (belt/quick items)
// - BuffDialog (buff/debuff display)
// - TimerDialog (event timers)
// - CompassDialog (compass/direction)
// - RollDialog (dice roll)
// - NoticeDialog (system notices)
// - KeyboardLayoutDialog (keybind settings)
// - HeroInventoryDialog (hero inventory)
// - HeroManageDialog (hero management)
// - IntelligentCreatureDialog (pet system)
// - ItemRentingDialog (item rental)
// - MentorDialog (mentor system)
// - RelationshipDialog (relationships)
// - ReportDialog (bug/player reports)
// - BigMapDialog (world map)
// - TrustMerchantDialog (merchant system)
// - DuraStatusDialog (equipment durability)
// - ChatNoticeDialog (chat notifications)

pub mod inventory_dialog;
pub mod character_dialog;
pub mod skillbar_dialog;
pub mod npc_dialog;
pub mod guild_dialog;
pub mod friend_dialog;
pub mod mentor_dialog;
pub mod ranking_dialog;
pub mod relationship_dialog;
pub mod group_dialog;
pub mod roll_dialog;
pub mod chat_option_dialog;
pub mod bigmap_dialog;
pub mod help_dialog;

// Game System Dialogs (Option D)
pub mod timer_dialog;
pub mod socket_dialog;
pub mod trust_merchant_dialog;
pub mod buff_dialog;
pub mod mount_dialog;
pub mod fishing_dialog;
pub mod refine_dialog;
pub mod craft_dialog;

// Layer 1 Basic Dialogs (Batch 1)
pub mod compass_dialog;
pub mod chat_notice_dialog;

// Layer 2 Trading Dialogs (Batch 4)
pub mod item_rental_dialog;
pub mod item_rent_dialog;
pub mod item_renting_dialog;

// Re-exports
pub use inventory_dialog::{InventoryDialog, InventoryTab};
pub use character_dialog::{CharacterDialog, CharacterPage, EquipmentSlot, CharacterStats, MagicInfo};
pub use hero_dialog::{HeroInventoryDialog, HeroBeltDialog, HeroMenuPanel, HeroInfoPanel, HeroAutoPotPreview, HeroBehaviourPanel, HeroManageDialog, HeroManageAvatar};
pub use skillbar_dialog::{SkillBarDialog, SkillSlot};
pub use npc_dialog::{NPCDialog, NPCDialogType, NPCOption, NPCPage, NPCGoodsDialog, NPCDropDialog, NPCAwakeDialog, CraftDialog, RefineDialog, StorageDialog, StorageType, BigButtonDialog, BigButton};
pub use trade_dialog::{TradeDialog, GuestTradeDialog};
pub use guild_dialog::{GuildDialog, GuildBuffButton};
pub use friend_dialog::{FriendDialog, FriendRow, MemoDialog};
pub use mentor_dialog::MentorDialog;
pub use ranking_dialog::{RankingDialog, RankClass, RankCharacterInfo, RankingRow};
pub use relationship_dialog::RelationshipDialog;
pub use group_dialog::{GroupDialog, GroupMember};
pub use roll_dialog::{RollDialog, RollType};
pub use chat_option_dialog::{ChatOptionDialog, ChatOptionTab};
pub use bigmap_dialog::{BigMapDialog, BigMapViewPort};
pub use mail_dialog::{MailListDialog, MailComposeLetterDialog, ClientMail, MailType, MailStatus};

// Game System Dialogs (Option D)
pub use inventory_dialog::{BeltDialog, BeltOrientation, BELT_SLOT_COUNT};
pub use timer_dialog::{TimerDialog, ClientTimer, TimerType};
pub use socket_dialog::{SocketDialog, MAX_SOCKET_SLOTS};
pub use buff_dialog::{BuffDialog, ClientBuff, BuffType, PoisonBuffDialog, ClientPoisonBuff, PoisonType};
pub use mount_dialog::{MountDialog, MountSlot, MountType};
pub use fishing_dialog::{FishingDialog, FishingStatusDialog, FishingSlot};
pub use trust_merchant_dialog::{TrustMerchantDialog, AuctionRow};
pub use help_dialog::{HelpDialog, HelpPage, HelpPageType};
pub use refine_dialog::{REFINE_SLOT_COUNT, REFINE_ROWS, REFINE_COLS};

// Re-exports for quest_dialog module
pub use quest_dialog::{
    QuestListDialog,
    QuestDetailDialog,
    QuestDiaryDialog,
    QuestTrackingDialog,
    QuestRow,
    QuestMessage,
    QuestRewards,
    QuestCell,
    QuestGroupQuestItem,
    QuestSingleQuestItem,
};

// Re-exports for intelligent_creature_dialog module
pub use intelligent_creature_dialog::{
    IntelligentCreatureDialog,
    CreatureButton,
    IntelligentCreatureOptionsDialog,
    IntelligentCreatureOptionsGradeDialog,
};

// Management Dialogs (Option E)
pub use menu_dialog::{MenuDialog, MenuButton};
pub use option_dialog::{OptionDialog, OptionType};
pub use keyboard_layout_dialog::{KeyboardLayoutDialog, KeybindOption, KeybindInfo, KeybindMode, KeybindRow, KeybindHeadingRow};
pub use notice_dialog::{NoticeDialog, Notice};
pub use inspect_dialog::{InspectDialog, InspectAction, MirClass, MirGender};
pub use report_dialog::{ReportDialog, ReportAction, ReportType};

// Core infrastructure
pub use dialog_manager::{
    Dialog, DialogManager, MouseButton, KeyCode
};

// Main UI Dialogs
pub use main_dialog::{
    MainDialog, MiniMapDialog, DuraStatusDialog, ChatDialog,
    ChatControlBar, SkillBarDialog as MainSkillBarDialog, InspectDialog as MainInspectDialog,
    OptionDialog as MainOptionDialog, MenuDialog as MainMenuDialog, MagicButton,
    AssignKeyPanel, CharacterDuraPanel
};

// Layer 1 Basic Dialogs
pub use compass_dialog::CompassDialog;
pub use chat_notice_dialog::{ChatNoticeDialog, ChatNoticeType};

// Layer 2 Trading Dialogs (Batch 4)
pub use item_rental_dialog::{ItemRentalDialog, ItemRow};
pub use item_rent_dialog::{ItemRentDialog, GuestItemRentDialog};
pub use item_renting_dialog::{ItemRentingDialog, GuestItemRentingDialog};
