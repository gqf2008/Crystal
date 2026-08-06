//! Account & Character Management Packets
//!
//! This module contains account and character management packet definitions and parsers.

use super::super::base::Packet;
use crate::data::stats::SharedResult;
use crate::{
    binary::{read_dotnet_string, write_dotnet_string},
    enums::{MirClass, MirGender, ServerPacketIds},
};
use byteorder::{LittleEndian, ReadBytesExt, WriteBytesExt};
use std::io::{Read, Write};

// ============================================================================
// Packet Structures
// ============================================================================

/// New character creation response
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct NewCharacter {
    pub result: u8,
}

/// New character creation successful
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NewCharacterSuccess {
    pub character: super::super::CharacterSummary,
}

/// Delete character request response
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct DeleteCharacter {
    pub result: u8,
}

/// Delete character successful
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct DeleteCharacterSuccess {
    pub character_index: i32,
}

// ============================================================================
// PacketMessage Implementations
// ============================================================================

impl Packet for NewCharacter {
    const OPCODE: i16 = ServerPacketIds::NewCharacter as i16;

    fn read_body<R: Read>(reader: &mut R) -> SharedResult<Self> {
        let result = reader.read_u8()?;
        Ok(Self { result })
    }

    fn write_body<W: Write>(&self, writer: &mut W) -> SharedResult<()> {
        writer.write_u8(self.result)?;
        Ok(())
    }
}

impl Packet for NewCharacterSuccess {
    const OPCODE: i16 = ServerPacketIds::NewCharacterSuccess as i16;

    fn read_body<R: Read>(reader: &mut R) -> SharedResult<Self> {
        let index = reader.read_i32::<LittleEndian>()?;
        let name = read_dotnet_string(reader)?;
        let level = reader.read_u16::<LittleEndian>()?;
        let class = MirClass::try_from(reader.read_u8()?)?;
        let gender = MirGender::try_from(reader.read_u8()?)?;
        let ticks = reader.read_i64::<LittleEndian>()?;
        let unix_epoch_ticks = 621355968000000000i64;
        let unix_seconds = (ticks - unix_epoch_ticks) / 10000000;
        use chrono::{TimeZone, Utc};
        let last_access = Utc
            .timestamp_opt(unix_seconds, 0)
            .single()
            .ok_or(crate::data::stats::SharedError::InvalidDateTime)?;

        Ok(Self {
            character: super::super::CharacterSummary {
                index,
                name,
                level,
                class,
                gender,
                last_access,
            },
        })
    }

    fn write_body<W: Write>(&self, writer: &mut W) -> SharedResult<()> {
        writer.write_i32::<LittleEndian>(self.character.index)?;
        write_dotnet_string(writer, &self.character.name)?;
        writer.write_u16::<LittleEndian>(self.character.level)?;
        writer.write_u8(self.character.class as u8)?;
        writer.write_u8(self.character.gender as u8)?;
        let unix_epoch_ticks = 621355968000000000i64;
        let ticks = self.character.last_access.timestamp() * 10000000 + unix_epoch_ticks;
        writer.write_i64::<LittleEndian>(ticks)?;
        Ok(())
    }
}

impl Packet for DeleteCharacter {
    const OPCODE: i16 = ServerPacketIds::DeleteCharacter as i16;

    fn read_body<R: Read>(reader: &mut R) -> SharedResult<Self> {
        let result = reader.read_u8()?;
        Ok(Self { result })
    }

    fn write_body<W: Write>(&self, writer: &mut W) -> SharedResult<()> {
        writer.write_u8(self.result)?;
        Ok(())
    }
}

impl Packet for DeleteCharacterSuccess {
    const OPCODE: i16 = ServerPacketIds::DeleteCharacterSuccess as i16;

    fn read_body<R: Read>(reader: &mut R) -> SharedResult<Self> {
        let character_index = reader.read_i32::<LittleEndian>()?;
        Ok(Self { character_index })
    }

    fn write_body<W: Write>(&self, writer: &mut W) -> SharedResult<()> {
        writer.write_i32::<LittleEndian>(self.character_index)?;
        Ok(())
    }
}
