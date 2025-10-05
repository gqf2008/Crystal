// Sound List - Sound effect and music index definitions
// Mirrors Client.MirSounds.SoundList

use std::collections::HashMap;
use std::fs::File;
use std::io::{BufRead, BufReader};
use std::path::Path;

use anyhow::{Context, Result};

/// Sound ID type
pub type SoundId = i32;

/// Sound index mapping (ID -> filename)
pub type SoundIndexMap = HashMap<SoundId, String>;

/// Load sound list from SoundList.lst file
/// 
/// File format: "index:filename" or "index\tfilename"
/// Example:
/// ```text
/// 10100:LoginEffect
/// 10103:ButtonA
/// ```
pub fn load_sound_list<P: AsRef<Path>>(sound_path: P) -> Result<SoundIndexMap> {
    let file_path = sound_path.as_ref().join("SoundList.lst");
    
    if !file_path.exists() {
        return Ok(HashMap::new());
    }

    let file = File::open(&file_path)
        .with_context(|| format!("Failed to open sound list: {:?}", file_path))?;
    
    let reader = BufReader::new(file);
    let mut map = HashMap::new();

    for line in reader.lines() {
        let line = line?;
        let line = line.replace(" ", "");
        
        // Split by ':' or '\t'
        let parts: Vec<&str> = line.split(&[':', '\t'][..]).collect();
        
        if parts.len() <= 1 {
            continue;
        }

        // Parse index
        if let Ok(index) = parts[0].parse::<SoundId>() {
            let filename = parts[parts.len() - 1].to_string();
            
            if !map.contains_key(&index) {
                map.insert(index, filename);
            }
        }
    }

    Ok(map)
}

/// Generate filename from sound index if not found in list
/// 
/// Rules (from C# SoundManager.cs):
/// - If index > 20000: Format as "M{major}-{minor}" where major = (index - 20000) / 10, minor = index % 10
/// - Otherwise: Format as "{major:000}-{minor:0}" where major = index / 10, minor = index % 10
pub fn generate_filename(index: SoundId) -> String {
    if index > 20000 {
        let major = (index - 20000) / 10;
        let minor = index % 10;
        format!("M{}-{}", major, minor)
    } else {
        let major = index / 10;
        let minor = index % 10;
        format!("{:03}-{}", major, minor)
    }
}

// Sound effect and music constants
// Mirrors C# SoundList class constants

pub const NONE: SoundId = 0;
pub const MUSIC: SoundId = 0;

// Login and UI
pub const INTRO_MUSIC: SoundId = 10146;
pub const SELECT_MUSIC: SoundId = 10147;
pub const LOGIN_EFFECT: SoundId = 10100;

// UI Sounds
pub const BUTTON_A: SoundId = 10103;
pub const BUTTON_B: SoundId = 10104;
pub const BUTTON_C: SoundId = 10105;
pub const GOLD: SoundId = 10106;
pub const EAT_DRUG: SoundId = 10107;
pub const CLICK_DRUG: SoundId = 10108;

pub const TELEPORT: SoundId = 10110;
pub const LEVEL_UP: SoundId = 10156;

// Item Click Sounds
pub const CLICK_WEAPON: SoundId = 10111;
pub const CLICK_ARMOUR: SoundId = 10112;
pub const CLICK_RING: SoundId = 10113;
pub const CLICK_BRACELET: SoundId = 10114;
pub const CLICK_NECKLACE: SoundId = 10115;
pub const CLICK_HELMET: SoundId = 10116;
pub const CLICK_BOOTS: SoundId = 10117;
pub const CLICK_ITEM: SoundId = 10118;

// Movement - Ground
pub const WALK_GROUND_L: SoundId = 10001;
pub const WALK_GROUND_R: SoundId = 10002;
pub const RUN_GROUND_L: SoundId = 10003;
pub const RUN_GROUND_R: SoundId = 10004;

// Movement - Stone
pub const WALK_STONE_L: SoundId = 10005;
pub const WALK_STONE_R: SoundId = 10006;
pub const RUN_STONE_L: SoundId = 10007;
pub const RUN_STONE_R: SoundId = 10008;

// Movement - Lawn
pub const WALK_LAWN_L: SoundId = 10009;
pub const WALK_LAWN_R: SoundId = 10010;
pub const RUN_LAWN_L: SoundId = 10011;
pub const RUN_LAWN_R: SoundId = 10012;

// Movement - Rough
pub const WALK_ROUGH_L: SoundId = 10013;
pub const WALK_ROUGH_R: SoundId = 10014;
pub const RUN_ROUGH_L: SoundId = 10015;
pub const RUN_ROUGH_R: SoundId = 10016;

// Movement - Wood
pub const WALK_WOOD_L: SoundId = 10017;
pub const WALK_WOOD_R: SoundId = 10018;
pub const RUN_WOOD_L: SoundId = 10019;
pub const RUN_WOOD_R: SoundId = 10020;

// Movement - Cave
pub const WALK_CAVE_L: SoundId = 10021;
pub const WALK_CAVE_R: SoundId = 10022;
pub const RUN_CAVE_L: SoundId = 10023;
pub const RUN_CAVE_R: SoundId = 10024;

// Movement - Room
pub const WALK_ROOM_L: SoundId = 10025;
pub const WALK_ROOM_R: SoundId = 10026;
pub const RUN_ROOM_L: SoundId = 10027;
pub const RUN_ROOM_R: SoundId = 10028;

// Movement - Water
pub const WALK_WATER_L: SoundId = 10029;
pub const WALK_WATER_R: SoundId = 10030;
pub const RUN_WATER_L: SoundId = 10031;
pub const RUN_WATER_R: SoundId = 10032;

// Movement - Horse
pub const HORSE_WALK_L: SoundId = 10033;
pub const HORSE_WALK_R: SoundId = 10034;
pub const HORSE_RUN: SoundId = 10035;

// Movement - Snow
pub const WALK_SNOW_L: SoundId = 10036;
pub const WALK_SNOW_R: SoundId = 10037;
pub const RUN_SNOW_L: SoundId = 10038;
pub const RUN_SNOW_R: SoundId = 10039;

// Weapon Swing
pub const SWING_SHORT: SoundId = 10050;
pub const SWING_WOOD: SoundId = 10051;
pub const SWING_SWORD: SoundId = 10052;
pub const SWING_SWORD2: SoundId = 10053;
pub const SWING_AXE: SoundId = 10054;
pub const SWING_CLUB: SoundId = 10055;
pub const SWING_LONG: SoundId = 10056;
pub const SWING_FIST: SoundId = 10056;

// Struck - Weapon
pub const STRUCK_SHORT: SoundId = 10060;
pub const STRUCK_WOODEN: SoundId = 10061;
pub const STRUCK_SWORD: SoundId = 10062;
pub const STRUCK_SWORD2: SoundId = 10063;
pub const STRUCK_AXE: SoundId = 10064;
pub const STRUCK_CLUB: SoundId = 10065;

// Struck - Body
pub const STRUCK_BODY_SWORD: SoundId = 10070;
pub const STRUCK_BODY_AXE: SoundId = 10071;
pub const STRUCK_BODY_LONG_STICK: SoundId = 10072;
pub const STRUCK_BODY_FIST: SoundId = 10073;

// Struck - Armour
pub const STRUCK_ARMOUR_SWORD: SoundId = 10080;
pub const STRUCK_ARMOUR_AXE: SoundId = 10081;
pub const STRUCK_ARMOUR_LONG_STICK: SoundId = 10082;
pub const STRUCK_ARMOUR_FIST: SoundId = 10083;

pub const STRUCK_EVIL_MIR: SoundId = 10090;

// Character Sounds
pub const MALE_FLINCH: SoundId = 10138;
pub const FEMALE_FLINCH: SoundId = 10139;
pub const MALE_DIE: SoundId = 10144;
pub const FEMALE_DIE: SoundId = 10145;

pub const REVIVE: SoundId = 20791;
pub const ZOMBIE_REVIVE: SoundId = 705;

// Mounts
pub const MOUNT_WALK_L: SoundId = 10176;
pub const MOUNT_WALK_R: SoundId = 10177;
pub const MOUNT_RUN: SoundId = 10178;
pub const TIGER_STRUCK1: SoundId = 10179;
pub const TIGER_STRUCK2: SoundId = 10180;
pub const TIGER_ATTACK1: SoundId = 10181;
pub const TIGER_ATTACK2: SoundId = 10182;
pub const TIGER_ATTACK3: SoundId = 10183;

// Fishing
pub const FISHING_THROW: SoundId = 10184;
pub const FISHING_PULL: SoundId = 10185;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_generate_filename() {
        // Normal format (matches C# logic: index / 10, index % 10)
        assert_eq!(generate_filename(10103), "1010-3");
        assert_eq!(generate_filename(10000), "1000-0");
        assert_eq!(generate_filename(10456), "1045-6");
        assert_eq!(generate_filename(123), "012-3");
        
        // Music format (> 20000) - generates M prefix
        assert_eq!(generate_filename(20791), "M79-1");
        assert_eq!(generate_filename(21000), "M100-0");
    }

    #[test]
    fn test_load_sound_list_nonexistent() {
        // Should return empty map if file doesn't exist
        let result = load_sound_list("/nonexistent/path");
        assert!(result.is_ok());
        assert!(result.unwrap().is_empty());
    }

    #[test]
    fn test_sound_constants() {
        assert_eq!(BUTTON_A, 10103);
        assert_eq!(INTRO_MUSIC, 10146);
        assert_eq!(WALK_GROUND_L, 10001);
        assert_eq!(REVIVE, 20791);
    }
}
