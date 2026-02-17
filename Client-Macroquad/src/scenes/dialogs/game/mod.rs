// Game dialogs module - 纯 Native 版本 (无 egui)

pub mod native_ui_utils;
// pub mod belt_dialog_native;
pub mod belt_dialog;
pub mod bigmap_dialog;
pub mod buff_dialog;
pub mod character_dialog;
pub mod chat_control_bar;
pub mod chat_dialog;
pub mod chat_option_dialog;
pub mod amount_box;
pub mod dura_status_dialog;
pub mod friend_dialog;
pub mod game_shop_dialog; 
pub mod group_dialog;
pub mod inspect_dialog;
pub mod npc_dialog;
pub mod npc_drop_dialog;
pub mod npc_goods_dialog;
pub mod inventory_dialog;
pub mod main_dialog;
pub mod menu_dialog;
pub mod minimap_dialog;
pub mod option_dialog;
pub mod quest_log_dialog;
pub mod skillbar_dialog;
pub mod storage_dialog;
pub mod trade_dialog;
pub mod ui_controls;
pub mod mail_dialog;
pub mod hero_dialog;
pub mod relationship_dialog;
pub mod craft_dialog;
pub mod mount_dialog;
pub mod misc_dialog;

// 导出 hybrid 版本作为主要实现
//pub use belt_dialog_native::{BeltDialogNative, BeltLayout, BeltItem};
pub use belt_dialog::{BeltDialogHybrid, BeltItemHybrid, BeltLayoutHybrid};
pub use bigmap_dialog::{BigMapDialogHybrid, BigMapAction, MapNpc, MapPlayer};
pub use buff_dialog::{BuffDialogHybrid, BuffType, ClientBuff};
pub use character_dialog::{CharacterDialogHybrid, CharacterTabHybrid, EquipmentItemHybrid, EquipSlot};
pub use chat_control_bar::{ChatControlBarHybrid, ChatFilterHybrid};
pub use chat_dialog::ChatDialogHybrid;
pub use chat_dialog::ChatMessageKind;
pub use chat_option_dialog::{ChatOptionDialogHybrid, ChatOptionSettingsHybrid};
pub use amount_box::{AmountBoxHybrid, AmountBoxResult};
pub use dura_status_dialog::{DuraStatusDialogHybrid, EquipDurability};
pub use friend_dialog::{FriendDialogHybrid, FriendAction, FriendEntry, FriendTab};
pub use game_shop_dialog::{GameShopDialogHybrid, ShopSectionHybrid, ShopClassHybrid, ShopCategoryHybrid, ShopItemHybrid};
pub use group_dialog::{GroupDialogHybrid, GroupAction, GroupMember};
pub use inspect_dialog::{InspectDialogHybrid, InspectAction, InspectEquipSlot, InspectEquipItem};
pub use npc_dialog::{NpcDialogHybrid, NpcDialogAction};
pub use npc_drop_dialog::{NPCDropDialogHybrid, DropAction, DropItem, PanelType};
pub use npc_goods_dialog::NpcGoodsDialogHybrid;
pub use inventory_dialog::{InventoryDialogHybrid, ItemSlotHybrid, InventoryTabHybrid};
pub use main_dialog::MainDialog;
pub use menu_dialog::{MenuDialogHybrid, MenuAction};
pub use minimap_dialog::MiniMapDialogHybrid;
pub use option_dialog::OptionDialogHybrid;
pub use quest_log_dialog::QuestLogDialogHybrid;
pub use skillbar_dialog::{SkillBarDialogHybrid, SkillBarAction, SkillSlot};
pub use storage_dialog::{StorageDialogHybrid, StorageAction, StorageItem};
pub use trade_dialog::{TradeDialogHybrid, TradeAction, TradeItem};
pub use ui_controls::{CheckBoxHybrid, TextBoxHybrid, DropDownBoxHybrid, ScrollingLabelHybrid, GoodsCellHybrid, ShopGoodsItem};
pub use mail_dialog::{MailListDialogHybrid, MailComposeDialogHybrid, MailReadDialogHybrid, MailAction, MailEntry};
pub use hero_dialog::{HeroManageDialogHybrid, HeroInventoryDialogHybrid, HeroBeltDialogHybrid, HeroAction, HeroInfo};
pub use relationship_dialog::{RelationshipDialogHybrid, MentorDialogHybrid, RelationshipAction};
pub use craft_dialog::{CraftDialogHybrid, RefineDialogHybrid, SocketDialogHybrid, CraftAction, CraftRecipe};
pub use mount_dialog::{MountDialogHybrid, FishingDialogHybrid, FishingStatusDialogHybrid, RankingDialogHybrid, MountAction, FishingAction, RankingAction, RankingTab, RankingEntry};
pub use misc_dialog::{
    HelpDialogHybrid, CompassDialogHybrid, TimerDialogHybrid, RollDialogHybrid,
    ReportDialogHybrid, KeyboardLayoutDialogHybrid, NoticeDialogHybrid,
    NewCharacterDialogHybrid, ItemRentalDialogHybrid, TrustMerchantDialogHybrid,
    IntelligentCreatureDialogHybrid, NPCAwakeDialogHybrid, ChatNoticeDialogHybrid,
    HelpAction, ReportAction, NewCharacterAction, RentalAction, TrustMerchantAction,
    CreatureAction, AwakeAction,
};

// 为了兼容性，创建别名
pub type BeltDialog = BeltDialogHybrid;
pub type BigMapDialog = BigMapDialogHybrid;
pub type BuffDialog = BuffDialogHybrid;
pub type CharacterDialog = CharacterDialogHybrid;
pub type ChatControlBar = ChatControlBarHybrid;
pub type ChatDialog = ChatDialogHybrid;
pub type ChatOptionDialog = ChatOptionDialogHybrid;
pub type DuraStatusDialog = DuraStatusDialogHybrid;
pub type FriendDialog = FriendDialogHybrid;
pub type GameShopDialog = GameShopDialogHybrid;
pub type GroupDialog = GroupDialogHybrid;
pub type InspectDialog = InspectDialogHybrid;
pub type InventoryDialog = InventoryDialogHybrid;
pub type MenuDialog = MenuDialogHybrid;
pub type MiniMapDialog = MiniMapDialogHybrid;
pub type NPCDropDialog = NPCDropDialogHybrid;
pub type OptionDialog = OptionDialogHybrid;
pub type QuestLogDialog = QuestLogDialogHybrid;
pub type SkillBarDialog = SkillBarDialogHybrid;
pub type StorageDialog = StorageDialogHybrid;
pub type TradeDialog = TradeDialogHybrid;
pub type MailListDialog = MailListDialogHybrid;
pub type MailComposeDialog = MailComposeDialogHybrid;
pub type MailReadDialog = MailReadDialogHybrid;
pub type HeroManageDialog = HeroManageDialogHybrid;
pub type HeroInventoryDialog = HeroInventoryDialogHybrid;
pub type HeroBeltDialog = HeroBeltDialogHybrid;
pub type RelationshipDialog = RelationshipDialogHybrid;
pub type MentorDialog = MentorDialogHybrid;
pub type CraftDialog = CraftDialogHybrid;
pub type RefineDialog = RefineDialogHybrid;
pub type SocketDialog = SocketDialogHybrid;
pub type MountDialog = MountDialogHybrid;
pub type FishingDialog = FishingDialogHybrid;
pub type FishingStatusDialog = FishingStatusDialogHybrid;
pub type RankingDialog = RankingDialogHybrid;
pub type HelpDialog = HelpDialogHybrid;
pub type CompassDialog = CompassDialogHybrid;
pub type TimerDialog = TimerDialogHybrid;
pub type RollDialog = RollDialogHybrid;
pub type ReportDialog = ReportDialogHybrid;
pub type KeyboardLayoutDialog = KeyboardLayoutDialogHybrid;
pub type NoticeDialog = NoticeDialogHybrid;
pub type NewCharacterDialog = NewCharacterDialogHybrid;
pub type ItemRentalDialog = ItemRentalDialogHybrid;
pub type TrustMerchantDialog = TrustMerchantDialogHybrid;
pub type IntelligentCreatureDialog = IntelligentCreatureDialogHybrid;
pub type NPCAwakeDialog = NPCAwakeDialogHybrid;
pub type ChatNoticeDialog = ChatNoticeDialogHybrid;
