//! Magic System Packets
//!
//! This module contains all magic/spell-related packet definitions and parsers.

use std::io::{Read, Write};
use byteorder::ReadBytesExt;
use crate::{
    data::client_data::ClientMagic,
    enums::{Spell, ServerPacketIds},
};
use super::super::base::PacketMessage;
use crate::data::stats::SharedResult;

// ============================================================================
// Packet Structures
// ============================================================================

/// Player learned a new magic/spell
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NewMagic {
    pub magic: ClientMagic,
    pub hero: bool,
}

/// Magic leveled up
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct MagicLeveled {
    pub spell: Spell,
    pub level: u8,
    pub hero: bool,
}

/// Magic removed from player
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct RemoveMagic {
    pub spell: Spell,
    pub hero: bool,
}

/// Spell toggle status changed
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SpellToggle {
    pub spell: Spell,
    pub can_use: bool,
    pub hero: bool,
}

// ============================================================================
// PacketMessage Implementations
// ============================================================================

impl PacketMessage for NewMagic {
    const OPCODE: i16 = ServerPacketIds::NewMagic as i16;

    fn read_body<R: Read>(reader: &mut R) -> SharedResult<Self> {
        let magic = ClientMagic::read_from(reader)?;
        let hero = reader.read_u8()? != 0;
        Ok(Self { magic, hero })
    }

    fn write_body<W: Write>(&self, writer: &mut W) -> SharedResult<()> {
        self.magic.write_to(writer)?;
        writer.write_all(&[if self.hero { 1 } else { 0 }])?;
        Ok(())
    }
}

impl PacketMessage for MagicLeveled {
    const OPCODE: i16 = ServerPacketIds::MagicLeveled as i16;

    fn read_body<R: Read>(reader: &mut R) -> SharedResult<Self> {
        let spell = Spell::try_from(reader.read_u8()?)?;
        let level = reader.read_u8()?;
        let hero = reader.read_u8()? != 0;
        Ok(Self { spell, level, hero })
    }

    fn write_body<W: Write>(&self, writer: &mut W) -> SharedResult<()> {
        writer.write_all(&[self.spell as u8, self.level, if self.hero { 1 } else { 0 }])?;
        Ok(())
    }
}

impl PacketMessage for RemoveMagic {
    const OPCODE: i16 = ServerPacketIds::RemoveMagic as i16;

    fn read_body<R: Read>(reader: &mut R) -> SharedResult<Self> {
        let spell = Spell::try_from(reader.read_u8()?)?;
        let hero = reader.read_u8()? != 0;
        Ok(Self { spell, hero })
    }

    fn write_body<W: Write>(&self, writer: &mut W) -> SharedResult<()> {
        writer.write_all(&[self.spell as u8, if self.hero { 1 } else { 0 }])?;
        Ok(())
    }
}

impl PacketMessage for SpellToggle {
    const OPCODE: i16 = ServerPacketIds::SpellToggle as i16;

    fn read_body<R: Read>(reader: &mut R) -> SharedResult<Self> {
        let spell = Spell::try_from(reader.read_u8()?)?;
        let can_use = reader.read_u8()? != 0;
        let hero = reader.read_u8()? != 0;
        Ok(Self {
            spell,
            can_use,
            hero,
        })
    }

    fn write_body<W: Write>(&self, writer: &mut W) -> SharedResult<()> {
        writer.write_all(&[
            self.spell as u8,
            if self.can_use { 1 } else { 0 },
            if self.hero { 1 } else { 0 },
        ])?;
        Ok(())
    }
}
