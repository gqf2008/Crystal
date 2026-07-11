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
pub mod group_dialog;
pub mod friend_dialog;
pub mod guild_dialog;
pub mod mentor_dialog;
pub mod relationship_dialog;
pub mod trade_dialog;
pub mod mount_dialog;
pub mod text_input_dialog;
pub mod big_map_dialog;

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
pub use group_dialog::{GroupDialogHybrid, GroupMember, GroupDialogAction};
pub use friend_dialog::{FriendDialogHybrid, FriendInfo, FriendDialogAction};
pub use guild_dialog::{GuildDialogHybrid, GuildInfo, GuildMember, GuildTab, GuildDialogAction};
pub use mentor_dialog::{MentorDialogHybrid, MentorInfo, MentorDialogAction};
pub use relationship_dialog::{RelationshipDialogHybrid, RelationshipInfo, RelationshipDialogAction};
pub use trade_dialog::{TradeDialogHybrid, TradeItemSlot, TradeAction, DragSource};
pub use mount_dialog::{MountDialogHybrid, MountEntry, MountDialogAction};
pub use hero_dialog::{HeroDialogHybrid, HeroInfo, HeroBehaviour, HeroDialogAction};
pub mod hero_dialog;
pub mod buff_dialog;
pub use buff_dialog::{BuffDialogHybrid, BuffEntry};
pub mod fishing_dialog;
pub use fishing_dialog::FishingDialogHybrid;
pub mod intelligent_creature_dialog;
pub use intelligent_creature_dialog::{IntelligentCreatureDialogHybrid, CreatureEntry};
pub mod compass_dialog;
pub use compass_dialog::{CompassDialogHybrid, CompassDirection};
pub mod socket_dialog;
pub use socket_dialog::{SocketDialogHybrid, SocketAction};
pub mod mail_dialog;
pub use mail_dialog::{MailDialogHybrid, MailTab, MailDialogAction};

pub mod ranking_dialog;
pub use ranking_dialog::{RankingDialogHybrid, RankingTab, RankingEntry, RankingDialogAction};

pub mod help_dialog;
pub use help_dialog::{HelpDialogHybrid, HelpDialogAction};

pub mod inspect_dialog;
pub use inspect_dialog::{InspectDialogHybrid, InspectEquipSlot, InspectDialogAction};

pub mod timer_dialog;
pub use timer_dialog::{TimerDialogHybrid, TimerEntry};

pub mod chat_notice_dialog;
pub use chat_notice_dialog::ChatNoticeDialogHybrid;

pub mod notice_dialog;
pub use notice_dialog::NoticeDialogHybrid;

pub mod roll_dialog;
pub use roll_dialog::RollDialogHybrid;

pub mod dura_status_dialog;
pub use dura_status_dialog::{DuraStatusDialogHybrid, DuraEntry};

pub mod npc_drop_dialog;
pub use npc_drop_dialog::NPCDropDialogHybrid;

pub mod guild_territory_dialog;
pub use guild_territory_dialog::{GuildTerritoryDialogHybrid, TerritoryEntry};

pub mod keyboard_layout_dialog;
pub use keyboard_layout_dialog::KeyboardLayoutDialogHybrid;

pub mod npc_awake_dialog;
pub use npc_awake_dialog::{NPCAwakeDialogHybrid, AwakeningMaterial};

pub mod craft_dialog;
pub use craft_dialog::{CraftDialogHybrid, CraftRecipe, CraftResult};

pub mod refine_dialog;
pub use refine_dialog::{RefineDialogHybrid, RefineStat};

pub mod item_rental_dialog;
pub use item_rental_dialog::{ItemRentalDialogHybrid, RentalItem};

pub mod trust_merchant_dialog;
pub use trust_merchant_dialog::{TrustMerchantDialogHybrid, MerchantItem, MerchantTab};

pub mod report_dialog;
pub use report_dialog::ReportDialogHybrid;

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
