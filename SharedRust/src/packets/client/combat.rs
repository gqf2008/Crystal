//! Combat Packets (Client → Server)

use super::super::base::Packet;
use crate::data::stats::SharedResult;
use crate::enums::{ClientPacketIds, MirDirection, Spell};
use crate::Point;
use byteorder::{LittleEndian, ReadBytesExt, WriteBytesExt};
use std::io::{Read, Write};

/// Client performs an attack
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Attack {
    pub direction: MirDirection,
    pub spell: Spell,
}

impl Packet for Attack {
    const OPCODE: i16 = ClientPacketIds::Attack as i16;

    fn read_body<R: Read>(reader: &mut R) -> SharedResult<Self> {
        let direction = MirDirection::try_from(reader.read_u8()?).unwrap_or(MirDirection::Up);
        let spell = Spell::try_from(reader.read_u8()?).unwrap_or(Spell::None);
        Ok(Self { direction, spell })
    }

    fn write_body<W: Write>(&self, writer: &mut W) -> SharedResult<()> {
        writer.write_u8(self.direction as u8)?;
        writer.write_u8(self.spell as u8)?;
        Ok(())
    }
}

/// Client performs a ranged attack
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct RangeAttack {
    pub direction: MirDirection,
    pub location: Point,
    pub target_id: u32,
    pub target_location: Point,
}

impl Packet for RangeAttack {
    const OPCODE: i16 = ClientPacketIds::RangeAttack as i16;

    fn read_body<R: Read>(reader: &mut R) -> SharedResult<Self> {
        let direction = MirDirection::try_from(reader.read_u8()?).unwrap_or(MirDirection::Up);
        let location = Point {
            x: reader.read_i32::<LittleEndian>()?,
            y: reader.read_i32::<LittleEndian>()?,
        };
        let target_id = reader.read_u32::<LittleEndian>()?;
        let target_location = Point {
            x: reader.read_i32::<LittleEndian>()?,
            y: reader.read_i32::<LittleEndian>()?,
        };
        Ok(Self {
            direction,
            location,
            target_id,
            target_location,
        })
    }

    fn write_body<W: Write>(&self, writer: &mut W) -> SharedResult<()> {
        writer.write_u8(self.direction as u8)?;
        writer.write_i32::<LittleEndian>(self.location.x)?;
        writer.write_i32::<LittleEndian>(self.location.y)?;
        writer.write_u32::<LittleEndian>(self.target_id)?;
        writer.write_i32::<LittleEndian>(self.target_location.x)?;
        writer.write_i32::<LittleEndian>(self.target_location.y)?;
        Ok(())
    }
}

/// Client requests to harvest
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Harvest {
    pub direction: MirDirection,
}

impl Packet for Harvest {
    const OPCODE: i16 = ClientPacketIds::Harvest as i16;

    fn read_body<R: Read>(reader: &mut R) -> SharedResult<Self> {
        let direction = MirDirection::try_from(reader.read_u8()?).unwrap_or(MirDirection::Up);
        Ok(Self { direction })
    }

    fn write_body<W: Write>(&self, writer: &mut W) -> SharedResult<()> {
        writer.write_u8(self.direction as u8)?;
        Ok(())
    }
}

/// Client requests to cast a spell/magic
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Magic {
    pub spell: Spell,
    pub direction: MirDirection,
    pub target_id: u32,
    pub location: Point,
}

impl Packet for Magic {
    const OPCODE: i16 = ClientPacketIds::Magic as i16;

    fn read_body<R: Read>(reader: &mut R) -> SharedResult<Self> {
        let spell = Spell::try_from(reader.read_u8()?).unwrap_or(Spell::None);
        let direction = MirDirection::try_from(reader.read_u8()?).unwrap_or(MirDirection::Up);
        let target_id = reader.read_u32::<LittleEndian>()?;
        let location = Point {
            x: reader.read_i32::<LittleEndian>()?,
            y: reader.read_i32::<LittleEndian>()?,
        };
        Ok(Self {
            spell,
            direction,
            target_id,
            location,
        })
    }

    fn write_body<W: Write>(&self, writer: &mut W) -> SharedResult<()> {
        writer.write_u8(self.spell as u8)?;
        writer.write_u8(self.direction as u8)?;
        writer.write_u32::<LittleEndian>(self.target_id)?;
        writer.write_i32::<LittleEndian>(self.location.x)?;
        writer.write_i32::<LittleEndian>(self.location.y)?;
        Ok(())
    }
}

/// Client requests to toggle spell on/off
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SpellToggle {
    pub spell: Spell,
    pub can_use: bool,
}

impl Packet for SpellToggle {
    const OPCODE: i16 = ClientPacketIds::SpellToggle as i16;

    fn read_body<R: Read>(reader: &mut R) -> SharedResult<Self> {
        let spell = Spell::try_from(reader.read_u8()?).unwrap_or(Spell::None);
        let can_use = reader.read_u8()? != 0;
        Ok(Self { spell, can_use })
    }

    fn write_body<W: Write>(&self, writer: &mut W) -> SharedResult<()> {
        writer.write_u8(self.spell as u8)?;
        writer.write_u8(if self.can_use { 1 } else { 0 })?;
        Ok(())
    }
}

/// Client assigns magic to a key
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct MagicKey {
    pub spell: Spell,
    pub key: u8,
    pub old_key: u8,
}

impl Packet for MagicKey {
    const OPCODE: i16 = ClientPacketIds::MagicKey as i16;

    fn read_body<R: Read>(reader: &mut R) -> SharedResult<Self> {
        let spell = Spell::try_from(reader.read_u8()?).unwrap_or(Spell::None);
        let key = reader.read_u8()?;
        let old_key = reader.read_u8()?;
        Ok(Self {
            spell,
            key,
            old_key,
        })
    }

    fn write_body<W: Write>(&self, writer: &mut W) -> SharedResult<()> {
        writer.write_u8(self.spell as u8)?;
        writer.write_u8(self.key)?;
        writer.write_u8(self.old_key)?;
        Ok(())
    }
}
