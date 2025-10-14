//! Buff/Status System Packets
//!
//! This module contains buff/status effect-related packet definitions and parsers.

use std::io::{Read, Write};
use byteorder::{LittleEndian, ReadBytesExt, WriteBytesExt};
use crate::{
    enums::{BuffType, PoisonType, ServerPacketIds},
    binary::{read_dotnet_string, write_dotnet_string},
};
use super::super::base::Packet;
use crate::data::stats::{SharedResult, Stats};

// ClientBuff 结构体
#[derive(Debug, Clone)]
pub struct ClientBuff {
    pub buff_type: BuffType,
    pub visible: bool,
    pub object_id: u32,
    pub expire_time: i64,
    pub infinite: bool,
    pub paused: bool,
    pub stats: Stats,
    pub values: Vec<i32>,
}

// ==================== Packet Structures & PacketMessage Implementations ====================

#[derive(Debug, Clone)]
pub struct AddBuff {
    pub buff: ClientBuff,
}

impl Packet for AddBuff {
    const OPCODE: i16 = ServerPacketIds::AddBuff as i16;

    fn read_body<R: Read>(reader: &mut R) -> SharedResult<Self> {
        let buff_type = BuffType::try_from(reader.read_u8()?)?;
        let visible = reader.read_u8()? != 0;
        let object_id = reader.read_u32::<LittleEndian>()?;
        let expire_time = reader.read_i64::<LittleEndian>()?;
        let infinite = reader.read_u8()? != 0;
        let paused = reader.read_u8()? != 0;
        let stats = Stats::read_from(reader)?;
        
        let values_count = reader.read_i32::<LittleEndian>()? as usize;
        let mut values = Vec::with_capacity(values_count);
        for _ in 0..values_count {
            values.push(reader.read_i32::<LittleEndian>()?);
        }

        Ok(Self {
            buff: ClientBuff {
                buff_type,
                visible,
                object_id,
                expire_time,
                infinite,
                paused,
                stats,
                values,
            },
        })
    }

    fn write_body<W: Write>(&self, writer: &mut W) -> SharedResult<()> {
        writer.write_u8(self.buff.buff_type as u8)?;
        writer.write_u8(if self.buff.visible { 1 } else { 0 })?;
        writer.write_u32::<LittleEndian>(self.buff.object_id)?;
        writer.write_i64::<LittleEndian>(self.buff.expire_time)?;
        writer.write_u8(if self.buff.infinite { 1 } else { 0 })?;
        writer.write_u8(if self.buff.paused { 1 } else { 0 })?;
        self.buff.stats.write_to(writer)?;
        
        writer.write_i32::<LittleEndian>(self.buff.values.len() as i32)?;
        for value in &self.buff.values {
            writer.write_i32::<LittleEndian>(*value)?;
        }
        Ok(())
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct RemoveBuff {
    pub buff_type: BuffType,
    pub object_id: u32,
}

impl Packet for RemoveBuff {
    const OPCODE: i16 = ServerPacketIds::RemoveBuff as i16;

    fn read_body<R: Read>(reader: &mut R) -> SharedResult<Self> {
        let buff_type = BuffType::try_from(reader.read_u8()?)?;
        let object_id = reader.read_u32::<LittleEndian>()?;
        Ok(Self { buff_type, object_id })
    }

    fn write_body<W: Write>(&self, writer: &mut W) -> SharedResult<()> {
        writer.write_u8(self.buff_type as u8)?;
        writer.write_u32::<LittleEndian>(self.object_id)?;
        Ok(())
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct PauseBuff {
    pub buff_type: BuffType,
    pub object_id: u32,
    pub paused: bool,
}

impl Packet for PauseBuff {
    const OPCODE: i16 = ServerPacketIds::PauseBuff as i16;

    fn read_body<R: Read>(reader: &mut R) -> SharedResult<Self> {
        let buff_type = BuffType::try_from(reader.read_u8()?)?;
        let object_id = reader.read_u32::<LittleEndian>()?;
        let paused = reader.read_u8()? != 0;
        Ok(Self { buff_type, object_id, paused })
    }

    fn write_body<W: Write>(&self, writer: &mut W) -> SharedResult<()> {
        writer.write_u8(self.buff_type as u8)?;
        writer.write_u32::<LittleEndian>(self.object_id)?;
        writer.write_u8(if self.paused { 1 } else { 0 })?;
        Ok(())
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ColourChanged {
    pub name_colour_argb: i32,
}

impl Packet for ColourChanged {
    const OPCODE: i16 = ServerPacketIds::ColourChanged as i16;

    fn read_body<R: Read>(reader: &mut R) -> SharedResult<Self> {
        let name_colour_argb = reader.read_i32::<LittleEndian>()?;
        Ok(Self { name_colour_argb })
    }

    fn write_body<W: Write>(&self, writer: &mut W) -> SharedResult<()> {
        writer.write_i32::<LittleEndian>(self.name_colour_argb)?;
        Ok(())
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ObjectColourChanged {
    pub object_id: u32,
    pub name_colour_argb: i32,
}

impl Packet for ObjectColourChanged {
    const OPCODE: i16 = ServerPacketIds::ObjectColourChanged as i16;

    fn read_body<R: Read>(reader: &mut R) -> SharedResult<Self> {
        let object_id = reader.read_u32::<LittleEndian>()?;
        let name_colour_argb = reader.read_i32::<LittleEndian>()?;
        Ok(Self { object_id, name_colour_argb })
    }

    fn write_body<W: Write>(&self, writer: &mut W) -> SharedResult<()> {
        writer.write_u32::<LittleEndian>(self.object_id)?;
        writer.write_i32::<LittleEndian>(self.name_colour_argb)?;
        Ok(())
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ObjectGuildNameChanged {
    pub object_id: u32,
    pub guild_name: String,
}

impl Packet for ObjectGuildNameChanged {
    const OPCODE: i16 = ServerPacketIds::ObjectGuildNameChanged as i16;

    fn read_body<R: Read>(reader: &mut R) -> SharedResult<Self> {
        let object_id = reader.read_u32::<LittleEndian>()?;
        let guild_name = read_dotnet_string(reader)?;
        Ok(Self { object_id, guild_name })
    }

    fn write_body<W: Write>(&self, writer: &mut W) -> SharedResult<()> {
        writer.write_u32::<LittleEndian>(self.object_id)?;
        write_dotnet_string(writer, &self.guild_name)?;
        Ok(())
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Poisoned {
    pub poison: PoisonType,
}

impl Packet for Poisoned {
    const OPCODE: i16 = ServerPacketIds::Poisoned as i16;

    fn read_body<R: Read>(reader: &mut R) -> SharedResult<Self> {
        let poison_value = reader.read_u16::<LittleEndian>()?;
        let poison = PoisonType::from_bits_truncate(poison_value);
        Ok(Self { poison })
    }

    fn write_body<W: Write>(&self, writer: &mut W) -> SharedResult<()> {
        writer.write_u16::<LittleEndian>(self.poison.bits())?;
        Ok(())
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ObjectPoisoned {
    pub object_id: u32,
    pub poison: PoisonType,
}

impl Packet for ObjectPoisoned {
    const OPCODE: i16 = ServerPacketIds::ObjectPoisoned as i16;

    fn read_body<R: Read>(reader: &mut R) -> SharedResult<Self> {
        let object_id = reader.read_u32::<LittleEndian>()?;
        let poison_value = reader.read_u16::<LittleEndian>()?;
        let poison = PoisonType::from_bits_truncate(poison_value);
        Ok(Self { object_id, poison })
    }

    fn write_body<W: Write>(&self, writer: &mut W) -> SharedResult<()> {
        writer.write_u32::<LittleEndian>(self.object_id)?;
        writer.write_u16::<LittleEndian>(self.poison.bits())?;
        Ok(())
    }
}
