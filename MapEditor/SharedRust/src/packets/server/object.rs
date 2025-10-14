//! Object Status Packets
//!
//! This module contains object status-related packet definitions and parsers.

use std::io::{Read, Write};
use byteorder::{LittleEndian, ReadBytesExt, WriteBytesExt};
use crate::{
    enums::{SpellEffect, ServerPacketIds},
    map::Point,
};
use super::super::base::Packet;
use crate::data::stats::SharedResult;

// ============================================================================
// Packet Structures & PacketMessage Implementations
// ============================================================================

/// Object health update
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ObjectHealth {
    pub object_id: u32,
    pub percent: u8,
    pub expire: u16,
}

impl Packet for ObjectHealth {
    const OPCODE: i16 = ServerPacketIds::ObjectHealth as i16;

    fn read_body<R: Read>(reader: &mut R) -> SharedResult<Self> {
        let object_id = reader.read_u32::<LittleEndian>()?;
        let percent = reader.read_u8()?;
        let expire = reader.read_u16::<LittleEndian>()?;
        Ok(Self {
            object_id,
            percent,
            expire,
        })
    }

    fn write_body<W: Write>(&self, writer: &mut W) -> SharedResult<()> {
        writer.write_u32::<LittleEndian>(self.object_id)?;
        writer.write_u8(self.percent)?;
        writer.write_u16::<LittleEndian>(self.expire)?;
        Ok(())
    }
}

/// Object mana update
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ObjectMana {
    pub object_id: u32,
    pub percent: u8,
}

impl Packet for ObjectMana {
    const OPCODE: i16 = ServerPacketIds::ObjectMana as i16;

    fn read_body<R: Read>(reader: &mut R) -> SharedResult<Self> {
        let object_id = reader.read_u32::<LittleEndian>()?;
        let percent = reader.read_u8()?;
        Ok(Self { object_id, percent })
    }

    fn write_body<W: Write>(&self, writer: &mut W) -> SharedResult<()> {
        writer.write_u32::<LittleEndian>(self.object_id)?;
        writer.write_u8(self.percent)?;
        Ok(())
    }
}

/// Object hidden status changed
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ObjectHidden {
    pub object_id: u32,
    pub hidden: bool,
}

impl Packet for ObjectHidden {
    const OPCODE: i16 = ServerPacketIds::ObjectHidden as i16;

    fn read_body<R: Read>(reader: &mut R) -> SharedResult<Self> {
        let object_id = reader.read_u32::<LittleEndian>()?;
        let hidden = reader.read_u8()? != 0;
        Ok(Self { object_id, hidden })
    }

    fn write_body<W: Write>(&self, writer: &mut W) -> SharedResult<()> {
        writer.write_u32::<LittleEndian>(self.object_id)?;
        writer.write_u8(if self.hidden { 1 } else { 0 })?;
        Ok(())
    }
}

/// Map effect
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct MapEffect {
    pub location: Point,
    pub effect: SpellEffect,
    pub value: i32,
}

impl Packet for MapEffect {
    const OPCODE: i16 = ServerPacketIds::MapEffect as i16;

    fn read_body<R: Read>(reader: &mut R) -> SharedResult<Self> {
        let x = reader.read_i32::<LittleEndian>()?;
        let y = reader.read_i32::<LittleEndian>()?;
        let location = Point { x, y };
        let effect = SpellEffect::try_from(reader.read_u8()?)?;
        let value = reader.read_i32::<LittleEndian>()?;
        Ok(Self {
            location,
            effect,
            value,
        })
    }

    fn write_body<W: Write>(&self, writer: &mut W) -> SharedResult<()> {
        writer.write_i32::<LittleEndian>(self.location.x)?;
        writer.write_i32::<LittleEndian>(self.location.y)?;
        writer.write_u8(self.effect as u8)?;
        writer.write_i32::<LittleEndian>(self.value)?;
        Ok(())
    }
}
