//! Player Status Packets
//!
//! This module contains all player status-related packet definitions and parsers.

use super::super::base::Packet;
use crate::{
    binary::{read_dotnet_string, write_dotnet_string},
    data::item::UserItem,
    data::stats::{SharedError, SharedResult},
    enums::{AttackMode, MirClass, MirGender, PetMode, ServerPacketIds},
};
use byteorder::{LittleEndian, ReadBytesExt, WriteBytesExt};
use chrono::{TimeZone, Utc};
use std::io::{Read, Write};

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

impl Packet for PlayerUpdate {
    const OPCODE: i16 = ServerPacketIds::PlayerUpdate as i16;

    fn read_body<R: Read>(reader: &mut R) -> SharedResult<Self> {
        Ok(PlayerUpdate {
            object_id: reader.read_u32::<LittleEndian>()?,
            light: reader.read_u8()?,
            weapon: reader.read_i16::<LittleEndian>()?,
            weapon_effect: reader.read_i16::<LittleEndian>()?,
            armor: reader.read_i16::<LittleEndian>()?,
            wings_effect: reader.read_u8()?,
        })
    }

    fn write_body<W: Write>(&self, writer: &mut W) -> SharedResult<()> {
        writer.write_u32::<LittleEndian>(self.object_id)?;
        writer.write_u8(self.light)?;
        writer.write_i16::<LittleEndian>(self.weapon)?;
        writer.write_i16::<LittleEndian>(self.weapon_effect)?;
        writer.write_i16::<LittleEndian>(self.armor)?;
        writer.write_u8(self.wings_effect)?;
        Ok(())
    }
}

impl Packet for PlayerInspect {
    const OPCODE: i16 = ServerPacketIds::PlayerInspect as i16;

    fn read_body<R: Read>(reader: &mut R) -> SharedResult<Self> {
        let name = read_dotnet_string(reader)?;
        let guild_name = read_dotnet_string(reader)?;
        let guild_rank = read_dotnet_string(reader)?;

        let count = reader.read_i32::<LittleEndian>()? as usize;
        let mut equipment = Vec::with_capacity(count);
        for _ in 0..count {
            let has_item = reader.read_u8()? != 0;
            if has_item {
                let item = UserItem::read_from(reader, i32::MAX, i32::MAX)?;
                equipment.push(Some(item));
            } else {
                equipment.push(None);
            }
        }

        let class_byte = reader.read_u8()?;
        let class = MirClass::try_from(class_byte)?;
        let gender_byte = reader.read_u8()?;
        let gender = MirGender::try_from(gender_byte)?;
        let hair = reader.read_u8()?;
        let level = reader.read_u16::<LittleEndian>()?;
        let lover_name = read_dotnet_string(reader)?;

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

    fn write_body<W: Write>(&self, writer: &mut W) -> SharedResult<()> {
        write_dotnet_string(writer, &self.name)?;
        write_dotnet_string(writer, &self.guild_name)?;
        write_dotnet_string(writer, &self.guild_rank)?;

        writer.write_i32::<LittleEndian>(self.equipment.len() as i32)?;
        for item_opt in &self.equipment {
            if let Some(item) = item_opt {
                writer.write_u8(1)?;
                item.write_to(writer)?;
            } else {
                writer.write_u8(0)?;
            }
        }

        writer.write_u8(self.class as u8)?;
        writer.write_u8(self.gender as u8)?;
        writer.write_u8(self.hair)?;
        writer.write_u16::<LittleEndian>(self.level)?;
        write_dotnet_string(writer, &self.lover_name)?;
        Ok(())
    }
}

impl Packet for LogOutSuccess {
    const OPCODE: i16 = ServerPacketIds::LogOutSuccess as i16;

    fn read_body<R: Read>(reader: &mut R) -> SharedResult<Self> {
        let count = reader.read_i32::<LittleEndian>()? as usize;
        let mut characters = Vec::with_capacity(count);

        for _ in 0..count {
            // Inline CharacterSummary parsing
            let index = reader.read_i32::<LittleEndian>()?;
            let name = read_dotnet_string(reader)?;
            let level = reader.read_u16::<LittleEndian>()?;
            let class_byte = reader.read_u8()?;
            let class = MirClass::try_from(class_byte)?;
            let gender_byte = reader.read_u8()?;
            let gender = MirGender::try_from(gender_byte)?;

            // Read .NET DateTime ticks and convert
            let ticks = reader.read_i64::<LittleEndian>()?;
            let unix_epoch_ticks = 621355968000000000i64;
            let unix_seconds = (ticks - unix_epoch_ticks) / 10000000;
            let last_access = Utc
                .timestamp_opt(unix_seconds, 0)
                .single()
                .ok_or(SharedError::InvalidDateTime)?;

            characters.push(super::super::CharacterSummary {
                index,
                name,
                level,
                class,
                gender,
                last_access,
            });
        }

        Ok(LogOutSuccess { characters })
    }

    fn write_body<W: Write>(&self, writer: &mut W) -> SharedResult<()> {
        writer.write_i32::<LittleEndian>(self.characters.len() as i32)?;

        for char in &self.characters {
            writer.write_i32::<LittleEndian>(char.index)?;
            write_dotnet_string(writer, &char.name)?;
            writer.write_u16::<LittleEndian>(char.level)?;
            writer.write_u8(char.class as u8)?;
            writer.write_u8(char.gender as u8)?;

            // Convert chrono DateTime to .NET ticks
            let unix_epoch_ticks = 621355968000000000i64;
            let ticks = unix_epoch_ticks + (char.last_access.timestamp() * 10000000);
            writer.write_i64::<LittleEndian>(ticks)?;
        }

        Ok(())
    }
}

impl Packet for TimeOfDay {
    const OPCODE: i16 = ServerPacketIds::TimeOfDay as i16;

    fn read_body<R: Read>(reader: &mut R) -> SharedResult<Self> {
        Ok(TimeOfDay {
            lights: reader.read_u8()?,
        })
    }

    fn write_body<W: Write>(&self, writer: &mut W) -> SharedResult<()> {
        writer.write_u8(self.lights)?;
        Ok(())
    }
}

impl Packet for ChangeAMode {
    const OPCODE: i16 = ServerPacketIds::ChangeAMode as i16;

    fn read_body<R: Read>(reader: &mut R) -> SharedResult<Self> {
        let mode_byte = reader.read_u8()?;
        let mode = AttackMode::try_from(mode_byte)?;
        Ok(ChangeAMode { mode })
    }

    fn write_body<W: Write>(&self, writer: &mut W) -> SharedResult<()> {
        writer.write_u8(self.mode as u8)?;
        Ok(())
    }
}

impl Packet for ChangePMode {
    const OPCODE: i16 = ServerPacketIds::ChangePMode as i16;

    fn read_body<R: Read>(reader: &mut R) -> SharedResult<Self> {
        let mode_byte = reader.read_u8()?;
        let mode = PetMode::try_from(mode_byte)?;
        Ok(ChangePMode { mode })
    }

    fn write_body<W: Write>(&self, writer: &mut W) -> SharedResult<()> {
        writer.write_u8(self.mode as u8)?;
        Ok(())
    }
}

impl Packet for ObjectName {
    const OPCODE: i16 = ServerPacketIds::ObjectName as i16;

    fn read_body<R: Read>(reader: &mut R) -> SharedResult<Self> {
        Ok(ObjectName {
            object_id: reader.read_u32::<LittleEndian>()?,
            name: read_dotnet_string(reader)?,
        })
    }

    fn write_body<W: Write>(&self, writer: &mut W) -> SharedResult<()> {
        writer.write_u32::<LittleEndian>(self.object_id)?;
        write_dotnet_string(writer, &self.name)?;
        Ok(())
    }
}

impl Packet for UserStorage {
    const OPCODE: i16 = ServerPacketIds::UserStorage as i16;

    fn read_body<R: Read>(reader: &mut R) -> SharedResult<Self> {
        let count = reader.read_i32::<LittleEndian>()? as usize;
        let mut storage = Vec::with_capacity(count);

        for _ in 0..count {
            let has_item = reader.read_u8()? != 0;
            if has_item {
                let item = UserItem::read_from(reader, i32::MAX, i32::MAX)?;
                storage.push(Some(item));
            } else {
                storage.push(None);
            }
        }

        Ok(UserStorage { storage })
    }

    fn write_body<W: Write>(&self, writer: &mut W) -> SharedResult<()> {
        writer.write_i32::<LittleEndian>(self.storage.len() as i32)?;

        for item_opt in &self.storage {
            if let Some(item) = item_opt {
                writer.write_u8(1)?;
                item.write_to(writer)?;
            } else {
                writer.write_u8(0)?;
            }
        }

        Ok(())
    }
}
