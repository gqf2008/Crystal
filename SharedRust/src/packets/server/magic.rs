//! Magic System Packets
//!
//! This module contains all magic/spell-related packet definitions and parsers.

use std::io::{Read, Write};
use byteorder::ReadBytesExt;
use crate::{
    data::client_data::ClientMagic,
    enums::{Spell, ServerPacketIds},
};
use super::super::base::Packet;
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

/// Magic leveled up (C# S.MagicLeveled: ObjectID u32 + Spell byte + Level byte + Experience u16)
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct MagicLeveled {
    pub object_id: u32,
    pub spell: Spell,
    pub level: u8,
    pub experience: u16,
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

impl Packet for NewMagic {
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

impl Packet for MagicLeveled {
    const OPCODE: i16 = ServerPacketIds::MagicLeveled as i16;

    fn read_body<R: Read>(reader: &mut R) -> SharedResult<Self> {
        use byteorder::{LittleEndian, ReadBytesExt};
        let object_id = reader.read_u32::<LittleEndian>()?;
        let spell = Spell::try_from(reader.read_u8()?)?;
        let level = reader.read_u8()?;
        let experience = reader.read_u16::<LittleEndian>()?;
        Ok(Self { object_id, spell, level, experience })
    }

    fn write_body<W: Write>(&self, writer: &mut W) -> SharedResult<()> {
        use byteorder::{LittleEndian, WriteBytesExt};
        writer.write_u32::<LittleEndian>(self.object_id)?;
        writer.write_all(&[self.spell as u8, self.level])?;
        writer.write_u16::<LittleEndian>(self.experience)?;
        Ok(())
    }
}

impl Packet for RemoveMagic {
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

impl Packet for SpellToggle {
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

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Cursor;

    #[test]
    fn magic_leveled_roundtrip() {
        // C# S.MagicLeveled: ObjectID u32 + Spell byte + Level byte + Experience u16（8 字节）
        let pkt = MagicLeveled {
            object_id: 1000,
            spell: Spell::FireBall,
            level: 2,
            experience: 500,
        };
        let mut buf = Vec::new();
        pkt.write_body(&mut buf).unwrap();
        assert_eq!(buf.len(), 8, "C# S.MagicLeveled 应为 8 字节（u32+byte+byte+u16）");
        let mut cur = Cursor::new(&buf);
        let read = MagicLeveled::read_body(&mut cur).unwrap();
        assert_eq!(read, pkt);
    }
}
