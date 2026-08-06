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
    // From stats
    BaseStat,
    BaseStats,
    // From client_data
    ClientAuction,
    ClientBuff,
    ClientFriend,
    // From shared_data
    ClientGTMap,
    ClientHeroInformation,
    ClientIntelligentCreature,
    ClientMagic,
    ClientMail,
    ClientMapInfo,
    ClientMovementInfo,
    ClientNPCInfo,
    ClientQuestInfo,
    ClientQuestProgress,
    ClientRecipeInfo,
    Door,
    // From item
    GameShopItem,
    GuildBuff,
    GuildBuffInfo,
    GuildBuffOld,
    GuildMember,
    GuildRank,
    GuildStorageItem,
    IntelligentCreatureItemFilter,
    IntelligentCreatureRules,
    ItemInfo,
    ItemRentalInformation,
    ItemSets,
    // From notice
    Notice,
    QuestItemReward,
    RankCharacterInfo,
    SelectInfo,
    SharedError,
    SharedResult,
    Stats,
    UserItem,
    WorldMapIcon,
    WorldMapSetup,
};

// Re-export additional item-related data types
pub use data::item::ChatItem;
