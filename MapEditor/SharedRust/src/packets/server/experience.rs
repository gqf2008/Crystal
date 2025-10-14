//! Experience and Level Packets
//!
//! Packets related to gaining experience and leveling up.

use std::io::{Read, Write};
use byteorder::{LittleEndian, ReadBytesExt, WriteBytesExt};
use crate::data::stats::SharedResult;
use crate::enums::ServerPacketIds;
use super::super::base::Packet;

/// GainExperience packet - experience gained notification
#[derive(Debug, Clone)]
pub struct GainExperience {
    pub amount: u32,
}

impl Packet for GainExperience {
    const OPCODE: i16 = ServerPacketIds::GainExperience as i16;

    fn read_body<R: Read>(reader: &mut R) -> SharedResult<Self> {
        let amount = reader.read_u32::<LittleEndian>()?;
        Ok(Self { amount })
    }

    fn write_body<W: Write>(&self, writer: &mut W) -> SharedResult<()> {
        writer.write_u32::<LittleEndian>(self.amount)?;
        Ok(())
    }
}

/// GainHeroExperience packet - hero experience gained notification
#[derive(Debug, Clone)]
pub struct GainHeroExperience {
    pub amount: u32,
}

impl Packet for GainHeroExperience {
    const OPCODE: i16 = ServerPacketIds::GainHeroExperience as i16;

    fn read_body<R: Read>(reader: &mut R) -> SharedResult<Self> {
        let amount = reader.read_u32::<LittleEndian>()?;
        Ok(Self { amount })
    }

    fn write_body<W: Write>(&self, writer: &mut W) -> SharedResult<()> {
        writer.write_u32::<LittleEndian>(self.amount)?;
        Ok(())
    }
}

/// LevelChanged packet - player level up notification
#[derive(Debug, Clone)]
pub struct LevelChanged {
    pub level: u16,
    pub experience: i64,
    pub max_experience: i64,
}

impl Packet for LevelChanged {
    const OPCODE: i16 = ServerPacketIds::LevelChanged as i16;

    fn read_body<R: Read>(reader: &mut R) -> SharedResult<Self> {
        let level = reader.read_u16::<LittleEndian>()?;
        let experience = reader.read_i64::<LittleEndian>()?;
        let max_experience = reader.read_i64::<LittleEndian>()?;
        Ok(Self { level, experience, max_experience })
    }

    fn write_body<W: Write>(&self, writer: &mut W) -> SharedResult<()> {
        writer.write_u16::<LittleEndian>(self.level)?;
        writer.write_i64::<LittleEndian>(self.experience)?;
        writer.write_i64::<LittleEndian>(self.max_experience)?;
        Ok(())
    }
}

/// HeroLevelChanged packet - hero level up notification
#[derive(Debug, Clone)]
pub struct HeroLevelChanged {
    pub level: u16,
    pub experience: i64,
    pub max_experience: i64,
}

impl Packet for HeroLevelChanged {
    const OPCODE: i16 = ServerPacketIds::HeroLevelChanged as i16;

    fn read_body<R: Read>(reader: &mut R) -> SharedResult<Self> {
        let level = reader.read_u16::<LittleEndian>()?;
        let experience = reader.read_i64::<LittleEndian>()?;
        let max_experience = reader.read_i64::<LittleEndian>()?;
        Ok(Self { level, experience, max_experience })
    }

    fn write_body<W: Write>(&self, writer: &mut W) -> SharedResult<()> {
        writer.write_u16::<LittleEndian>(self.level)?;
        writer.write_i64::<LittleEndian>(self.experience)?;
        writer.write_i64::<LittleEndian>(self.max_experience)?;
        Ok(())
    }
}

/// ObjectLeveled packet - notification that an object leveled up
#[derive(Debug, Clone)]
pub struct ObjectLeveled {
    pub object_id: u32,
    pub level: u16,
}

impl Packet for ObjectLeveled {
    const OPCODE: i16 = ServerPacketIds::ObjectLeveled as i16;

    fn read_body<R: Read>(reader: &mut R) -> SharedResult<Self> {
        let object_id = reader.read_u32::<LittleEndian>()?;
        let level = reader.read_u16::<LittleEndian>()?;
        Ok(Self { object_id, level })
    }

    fn write_body<W: Write>(&self, writer: &mut W) -> SharedResult<()> {
        writer.write_u32::<LittleEndian>(self.object_id)?;
        writer.write_u16::<LittleEndian>(self.level)?;
        Ok(())
    }
}

/// DuraChanged packet - item durability changed
#[derive(Debug, Clone)]
pub struct DuraChanged {
    pub unique_id: u64,
    pub current_dura: u16,
}

impl Packet for DuraChanged {
    const OPCODE: i16 = ServerPacketIds::DuraChanged as i16;

    fn read_body<R: Read>(reader: &mut R) -> SharedResult<Self> {
        let unique_id = reader.read_u64::<LittleEndian>()?;
        let current_dura = reader.read_u16::<LittleEndian>()?;
        Ok(Self { unique_id, current_dura })
    }

    fn write_body<W: Write>(&self, writer: &mut W) -> SharedResult<()> {
        writer.write_u64::<LittleEndian>(self.unique_id)?;
        writer.write_u16::<LittleEndian>(self.current_dura)?;
        Ok(())
    }
}

/// DeleteItem packet - item deleted notification
#[derive(Debug, Clone)]
pub struct DeleteItem {
    pub unique_id: u64,
    pub count: u32,
}

impl Packet for DeleteItem {
    const OPCODE: i16 = ServerPacketIds::DeleteItem as i16;

    fn read_body<R: Read>(reader: &mut R) -> SharedResult<Self> {
        let unique_id = reader.read_u64::<LittleEndian>()?;
        let count = reader.read_u32::<LittleEndian>()?;
        Ok(Self { unique_id, count })
    }

    fn write_body<W: Write>(&self, writer: &mut W) -> SharedResult<()> {
        writer.write_u64::<LittleEndian>(self.unique_id)?;
        writer.write_u32::<LittleEndian>(self.count)?;
        Ok(())
    }
}
