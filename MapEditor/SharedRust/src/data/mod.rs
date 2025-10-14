pub mod client_data;
pub mod item;
pub mod notice;
pub mod shared_data;
pub mod stats;

// Re-export main types (not modules) to avoid naming conflicts
// Users can access modules via `data::item::*` or `data::stats::*` etc.
pub use client_data::*;
pub use notice::*;
pub use shared_data::*;

// Export specific types from item and stats modules
pub use item::{GameShopItem, ItemInfo, ItemRentalInformation, ItemSets, UserItem};
pub use stats::{BaseStat, BaseStats, SharedError, SharedResult, Stats};
