// ============================================================================
// Dialogs 模块 — 游戏对话框组件 (完整版)
// ============================================================================
//
// Phase 1 ~ Phase 4 全部对话框实现。

// --- Phase 1: 核心 UI (高优先级) ---
pub mod npc_dialog;

// --- Phase 2: 社交系统 ---
pub mod mail_dialog;
pub mod big_map_dialog;
pub mod socket_dialog;

// --- Phase 3: 特色功能 ---
pub mod gameshop_dialog;
pub mod fishing_dialog;
pub mod mount_dialog;
pub mod mentor_dialog;
pub mod relationship_dialog;
pub mod trust_merchant_dialog;
pub mod ranking_dialog;

// --- Phase 4: 低优先级辅助功能 ---
pub mod misc_dialogs;

// 重新导出所有对话框类型
pub use npc_dialog::{NpcDialog, NpcDropDialog, NpcDialogAction, NpcTextLine};
pub use mail_dialog::{MailDialog, MailView, MailAction, MailSummary, MailContent};
pub use big_map_dialog::BigMapDialog;
pub use socket_dialog::SocketDialog;
pub use gameshop_dialog::{GameShopDialog, ShopCategory, ShopItem};
pub use fishing_dialog::{FishingDialog, FishingStatusDialog, FishingState};
pub use mount_dialog::MountDialog;
pub use mentor_dialog::MentorDialog;
pub use relationship_dialog::RelationshipDialog;
pub use trust_merchant_dialog::{TrustMerchantDialog, ConsignmentItem};
pub use ranking_dialog::{RankingDialog, RankingCategory, RankingEntry};
pub use misc_dialogs::{
    HeroDialog, HelpDialog, NoticeDialog, ChatNoticeDialog,
    ReportDialog, KeyboardLayoutDialog, RollDialog, TimerDialog,
    CompassDialog, IntelligentCreatureDialog, ChatOptionDialog,
    ItemRentalDialog, ItemRentDialog, ItemRentingDialog,
};
