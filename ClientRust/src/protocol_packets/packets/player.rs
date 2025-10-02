//! Player Status Packets
//!
//! This module contains all player status-related packet definitions and parsers.

use byteorder::{LittleEndian, ReadBytesExt};
use mir2_shared::{
    binary::read_dotnet_string,
    enums::{AttackMode, MirClass, MirGender, PetMode},
    UserItem,
};
use std::io::Cursor;

// Helper for parsing character summary (shared with account.rs)
pub(crate) fn parse_character_summary(
    cursor: &mut Cursor<&[u8]>,
) -> Result<super::super::CharacterSummary, String> {
    let index = cursor
        .read_i32::<LittleEndian>()
        .map_err(|e| format!("Failed to read index: {}", e))?;
    let name = read_dotnet_string(cursor)?;
    let level = cursor
        .read_u16::<LittleEndian>()
        .map_err(|e| format!("Failed to read level: {}", e))?;
    let class_byte = cursor
        .read_u8()
        .map_err(|e| format!("Failed to read class: {}", e))?;
    let class =
        MirClass::try_from(class_byte).map_err(|_| format!("Unknown class: {}", class_byte))?;
    let gender_byte = cursor
        .read_u8()
        .map_err(|e| format!("Failed to read gender: {}", e))?;
    let gender =
        MirGender::try_from(gender_byte).map_err(|_| format!("Unknown gender: {}", gender_byte))?;
    let last_access = cursor
        .read_i64::<LittleEndian>()
        .map_err(|e| format!("Failed to read last_access: {}", e))?;

    Ok(super::super::CharacterSummary {
        index,
        name,
        level,
        class,
        gender,
        last_access,
    })
}

// ============================================================================
// Packet Structures
// ============================================================================

/// Player object visual update
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct PlayerUpdate {
    pub object_id: u32,
    pub light: u8,
    pub weapon: i16,
    pub weapon_effect: i16,
    pub armor: i16,
    pub wings_effect: u8,
}

/// Player inspection data
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PlayerInspect {
    pub name: String,
    pub guild_name: String,
    pub guild_rank: String,
    pub equipment: Vec<Option<UserItem>>,
    pub class: MirClass,
    pub gender: MirGender,
    pub hair: u8,
    pub level: u16,
    pub lover_name: String,
}

/// Logout successful with character list
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LogOutSuccess {
    pub characters: Vec<super::super::CharacterSummary>,
}

/// Time of day / light setting changed
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct TimeOfDay {
    pub lights: u8,
}

/// Attack mode changed
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ChangeAMode {
    pub mode: AttackMode,
}

/// Pet mode changed
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ChangePMode {
    pub mode: PetMode,
}

/// Object name update
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ObjectName {
    pub object_id: u32,
    pub name: String,
}

/// User storage update
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct UserStorage {
    pub storage: Vec<Option<UserItem>>,
}

// ============================================================================
// Parser Functions
// ============================================================================

pub(crate) fn parse_player_update(payload: &[u8]) -> Result<PlayerUpdate, String> {
    let mut cursor = Cursor::new(payload);
    let object_id = cursor
        .read_u32::<LittleEndian>()
        .map_err(|e| format!("Failed to read object_id: {}", e))?;
    let light = cursor
        .read_u8()
        .map_err(|e| format!("Failed to read light: {}", e))?;
    let weapon = cursor
        .read_i16::<LittleEndian>()
        .map_err(|e| format!("Failed to read weapon: {}", e))?;
    let weapon_effect = cursor
        .read_i16::<LittleEndian>()
        .map_err(|e| format!("Failed to read weapon_effect: {}", e))?;
    let armor = cursor
        .read_i16::<LittleEndian>()
        .map_err(|e| format!("Failed to read armor: {}", e))?;
    let wings_effect = cursor
        .read_u8()
        .map_err(|e| format!("Failed to read wings_effect: {}", e))?;
    Ok(PlayerUpdate {
        object_id,
        light,
        weapon,
        weapon_effect,
        armor,
        wings_effect,
    })
}

pub(crate) fn parse_player_inspect(payload: &[u8]) -> Result<PlayerInspect, String> {
    let mut cursor = Cursor::new(payload);
    let name = read_dotnet_string(&mut cursor)?;
    let guild_name = read_dotnet_string(&mut cursor)?;
    let guild_rank = read_dotnet_string(&mut cursor)?;

    let count = cursor
        .read_i32::<LittleEndian>()
        .map_err(|e| format!("Failed to read equipment count: {}", e))?;
    let mut equipment = Vec::new();
    for _ in 0..count {
        let has_item = cursor
            .read_u8()
            .map_err(|e| format!("Failed to read has_item flag: {}", e))?
            != 0;
        if has_item {
            let item = UserItem::read_from(&mut cursor)?;
            equipment.push(Some(item));
        } else {
            equipment.push(None);
        }
    }

    let class_byte = cursor
        .read_u8()
        .map_err(|e| format!("Failed to read class: {}", e))?;
    let class =
        MirClass::try_from(class_byte).map_err(|_| format!("Unknown class: {}", class_byte))?;
    let gender_byte = cursor
        .read_u8()
        .map_err(|e| format!("Failed to read gender: {}", e))?;
    let gender =
        MirGender::try_from(gender_byte).map_err(|_| format!("Unknown gender: {}", gender_byte))?;
    let hair = cursor
        .read_u8()
        .map_err(|e| format!("Failed to read hair: {}", e))?;
    let level = cursor
        .read_u16::<LittleEndian>()
        .map_err(|e| format!("Failed to read level: {}", e))?;
    let lover_name = read_dotnet_string(&mut cursor)?;

    Ok(PlayerInspect {
        name,
        guild_name,
        guild_rank,
        equipment,
        class,
        gender,
        hair,
        level,
        lover_name,
    })
}

pub(crate) fn parse_logout_success(payload: &[u8]) -> Result<LogOutSuccess, String> {
    let mut cursor = Cursor::new(payload);
    let count = cursor
        .read_i32::<LittleEndian>()
        .map_err(|e| format!("Failed to read character count: {}", e))?;
    let mut characters = Vec::new();
    for _ in 0..count {
        characters.push(parse_character_summary(&mut cursor)?);
    }
    Ok(LogOutSuccess { characters })
}

pub(crate) fn parse_time_of_day(payload: &[u8]) -> Result<TimeOfDay, String> {
    let mut cursor = Cursor::new(payload);
    let lights = cursor
        .read_u8()
        .map_err(|e| format!("Failed to read lights: {}", e))?;
    Ok(TimeOfDay { lights })
}

pub(crate) fn parse_change_amode(payload: &[u8]) -> Result<ChangeAMode, String> {
    let mut cursor = Cursor::new(payload);
    let mode_byte = cursor
        .read_u8()
        .map_err(|e| format!("Failed to read mode: {}", e))?;
    let mode = AttackMode::try_from(mode_byte)
        .map_err(|_| format!("Unknown attack mode: {}", mode_byte))?;
    Ok(ChangeAMode { mode })
}

pub(crate) fn parse_change_pmode(payload: &[u8]) -> Result<ChangePMode, String> {
    let mut cursor = Cursor::new(payload);
    let mode_byte = cursor
        .read_u8()
        .map_err(|e| format!("Failed to read mode: {}", e))?;
    let mode =
        PetMode::try_from(mode_byte).map_err(|_| format!("Unknown pet mode: {}", mode_byte))?;
    Ok(ChangePMode { mode })
}

pub(crate) fn parse_object_name(payload: &[u8]) -> Result<ObjectName, String> {
    let mut cursor = Cursor::new(payload);
    let object_id = cursor
        .read_u32::<LittleEndian>()
        .map_err(|e| format!("Failed to read object_id: {}", e))?;
    let name = read_dotnet_string(&mut cursor)?;
    Ok(ObjectName { object_id, name })
}

pub(crate) fn parse_user_storage(payload: &[u8]) -> Result<UserStorage, String> {
    let mut cursor = Cursor::new(payload);
    let count = cursor
        .read_i32::<LittleEndian>()
        .map_err(|e| format!("Failed to read storage count: {}", e))?;
    let mut storage = Vec::new();
    for _ in 0..count {
        let has_item = cursor
            .read_u8()
            .map_err(|e| format!("Failed to read has_item flag: {}", e))?
            != 0;
        if has_item {
            let item = UserItem::read_from(&mut cursor)?;
            storage.push(Some(item));
        } else {
            storage.push(None);
        }
    }
    Ok(UserStorage { storage })
}
