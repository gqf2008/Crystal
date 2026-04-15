//! Global constants used throughout the game
//! Ported from Shared/Globals.cs

pub const PRODUCT_CODENAME: &str = "Crystal";

// Account and Character Limits
pub const MIN_ACCOUNT_ID_LENGTH: usize = 3;
pub const MAX_ACCOUNT_ID_LENGTH: usize = 15;

pub const MIN_PASSWORD_LENGTH: usize = 5;
pub const MAX_PASSWORD_LENGTH: usize = 15;

pub const MIN_CHARACTER_NAME_LENGTH: usize = 3;
pub const MAX_CHARACTER_NAME_LENGTH: usize = 15;
pub const MAX_CHARACTER_COUNT: usize = 4;

// Game Limits
pub const MAX_CHAT_LENGTH: usize = 80;
pub const STORAGE_GRID_SIZE: usize = 80;
pub const MAX_GROUP: usize = 15;
pub const MAX_PETS: usize = 5;
pub const MAX_ATTACK_RANGE: i32 = 9;
pub const MAX_DRAGON_LEVEL: u8 = 13;
pub const CLASS_WEAPON_COUNT: usize = 100;
pub const FLAG_INDEX_COUNT: u32 = 1999;
pub const MAX_CONCURRENT_QUESTS: usize = 20;
pub const LOG_DELAY: u32 = 10000;
pub const DATA_RANGE: i32 = 16;

// Trading and Economy
pub const COMMISSION: f32 = 0.05;
pub const SEARCH_DELAY: u32 = 500;
pub const CONSIGNMENT_LENGTH: u32 = 7;
pub const CONSIGNMENT_COST: u32 = 5000;
pub const MIN_CONSIGNMENT: u32 = 5000;
pub const MAX_CONSIGNMENT: u32 = 50000000;
pub const AUCTION_COST: u32 = 5000;
pub const MIN_STARTING_BID: u32 = 0;
pub const MAX_STARTING_BID: u32 = 50000;

// Item Shapes
pub const FISHING_ROD_SHAPES: &[i32] = &[49, 50];

// Ranged Spells
use crate::enums::Spell;

pub const RANGED_SPELLS: &[Spell] = &[
    Spell::FireBall,
    Spell::ThunderBolt,
    Spell::FireBang,
    Spell::FireWall,
    Spell::FrostCrunch,
    Spell::Vampirism,
    Spell::FlameDisruptor,
    Spell::IceStorm,
    Spell::MeteorStrike,
    Spell::Blizzard,
    Spell::SoulFireBall,
    Spell::StraightShot,
    Spell::ElementalShot,
    Spell::PoisonShot,
];

/// Check if a spell is ranged
pub fn is_ranged_spell(spell: Spell) -> bool {
    RANGED_SPELLS.contains(&spell)
}

/// Check if an item shape is a fishing rod
pub fn is_fishing_rod(shape: i16) -> bool {
    FISHING_ROD_SHAPES.contains(&(shape as i32))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_fishing_rod_detection() {
        assert!(is_fishing_rod(49));
        assert!(is_fishing_rod(50));
        assert!(!is_fishing_rod(1));
    }

    #[test]
    fn test_ranged_spell_detection() {
        assert!(is_ranged_spell(Spell::FireBall));
        assert!(is_ranged_spell(Spell::ThunderBolt));
        assert!(!is_ranged_spell(Spell::Healing));
    }
}
