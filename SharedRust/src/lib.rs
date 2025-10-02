pub mod binary;
pub mod data; // Data structures (client_data, item, stats, notice, shared_data)
pub mod enums;
pub mod globals;
pub mod map; // Basic map types (Point)
pub mod packets; // Packet infrastructure and definitions (base, ids, client, server)
pub mod utils; // Utility functions (direction, geometry)

// Re-export commonly used items directly
// Note: We don't glob re-export data::* and packets::* to avoid naming conflicts
// (both have an "item" submodule)
#[allow(unused_imports)]
pub use crate::{binary::*, enums::*, globals::*, map::*, packets::PacketHeader};

// Re-export data types explicitly
pub use data::{
    // From client_data
    ClientAuction, ClientBuff, ClientFriend, ClientHeroInformation, ClientIntelligentCreature,
    ClientMagic, ClientMail, ClientMapInfo, ClientMovementInfo, ClientNPCInfo, ClientQuestInfo,
    ClientQuestProgress, ClientRecipeInfo, GuildMember, GuildRank, GuildStorageItem,
    IntelligentCreatureItemFilter, IntelligentCreatureRules, SelectInfo,
    // From item
    GameShopItem, ItemInfo, ItemRentalInformation, ItemSetStatus, UserItem,
    // From stats
    BaseStat, BaseStats, SharedError, SharedResult, Stats,
    // From notice
    Notice,
    // From shared_data
    ClientGTMap, Door, QuestItemReward, RankCharacterInfo, WorldMapIcon, WorldMapSetup,
};
