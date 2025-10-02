//! NPC System Packets
//!
//! This module contains all NPC-related packet definitions and parsers.

use std::io::{Read, Write};
use byteorder::{LittleEndian, ReadBytesExt, WriteBytesExt};
use crate::{
    enums::ServerPacketIds,
    binary::{read_dotnet_string, write_dotnet_string},
};
use super::super::base::Packet;
use crate::data::stats::SharedResult;

// ============================================================================
// Packet Structures & PacketMessage Implementations
// ============================================================================

/// NPC opens sell dialog
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct NPCSell;

impl Packet for NPCSell {
    const OPCODE: i16 = ServerPacketIds::NPCSell as i16;
    
    fn read_body<R: Read>(_reader: &mut R) -> SharedResult<Self> {
        Ok(Self)
    }
    
    fn write_body<W: Write>(&self, _writer: &mut W) -> SharedResult<()> {
        Ok(())
    }
}

/// NPC repair dialog with repair rate
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct NPCRepair {
    pub rate: f32,
}

impl Packet for NPCRepair {
    const OPCODE: i16 = ServerPacketIds::NPCRepair as i16;
    
    fn read_body<R: Read>(reader: &mut R) -> SharedResult<Self> {
        let rate = reader.read_f32::<LittleEndian>()?;
        Ok(Self { rate })
    }
    
    fn write_body<W: Write>(&self, writer: &mut W) -> SharedResult<()> {
        writer.write_f32::<LittleEndian>(self.rate)?;
        Ok(())
    }
}

/// NPC special repair dialog with repair rate
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct NPCSRepair {
    pub rate: f32,
}

impl Packet for NPCSRepair {
    const OPCODE: i16 = ServerPacketIds::NPCSRepair as i16;
    
    fn read_body<R: Read>(reader: &mut R) -> SharedResult<Self> {
        let rate = reader.read_f32::<LittleEndian>()?;
        Ok(Self { rate })
    }
    
    fn write_body<W: Write>(&self, writer: &mut W) -> SharedResult<()> {
        writer.write_f32::<LittleEndian>(self.rate)?;
        Ok(())
    }
}

/// NPC refine dialog
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct NPCRefine {
    pub rate: f32,
    pub refining: bool,
}

impl Packet for NPCRefine {
    const OPCODE: i16 = ServerPacketIds::NPCRefine as i16;
    
    fn read_body<R: Read>(reader: &mut R) -> SharedResult<Self> {
        let rate = reader.read_f32::<LittleEndian>()?;
        let refining = reader.read_u8()? != 0;
        Ok(Self { rate, refining })
    }
    
    fn write_body<W: Write>(&self, writer: &mut W) -> SharedResult<()> {
        writer.write_f32::<LittleEndian>(self.rate)?;
        writer.write_u8(if self.refining { 1 } else { 0 })?;
        Ok(())
    }
}

/// NPC check refine status
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct NPCCheckRefine;

impl Packet for NPCCheckRefine {
    const OPCODE: i16 = ServerPacketIds::NPCCheckRefine as i16;
    
    fn read_body<R: Read>(_reader: &mut R) -> SharedResult<Self> {
        Ok(Self)
    }
    
    fn write_body<W: Write>(&self, _writer: &mut W) -> SharedResult<()> {
        Ok(())
    }
}

/// NPC collect refine result
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct NPCCollectRefine {
    pub success: bool,
}

impl Packet for NPCCollectRefine {
    const OPCODE: i16 = ServerPacketIds::NPCCollectRefine as i16;
    
    fn read_body<R: Read>(reader: &mut R) -> SharedResult<Self> {
        let success = reader.read_u8()? != 0;
        Ok(Self { success })
    }
    
    fn write_body<W: Write>(&self, writer: &mut W) -> SharedResult<()> {
        writer.write_u8(if self.success { 1 } else { 0 })?;
        Ok(())
    }
}

/// NPC replace wedding ring dialog
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct NPCReplaceWedRing {
    pub rate: f32,
}

impl Packet for NPCReplaceWedRing {
    const OPCODE: i16 = ServerPacketIds::NPCReplaceWedRing as i16;
    
    fn read_body<R: Read>(reader: &mut R) -> SharedResult<Self> {
        let rate = reader.read_f32::<LittleEndian>()?;
        Ok(Self { rate })
    }
    
    fn write_body<W: Write>(&self, writer: &mut W) -> SharedResult<()> {
        writer.write_f32::<LittleEndian>(self.rate)?;
        Ok(())
    }
}

/// NPC storage dialog
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct NPCStorage;

impl Packet for NPCStorage {
    const OPCODE: i16 = ServerPacketIds::NPCStorage as i16;
    
    fn read_body<R: Read>(_reader: &mut R) -> SharedResult<Self> {
        Ok(Self)
    }
    
    fn write_body<W: Write>(&self, _writer: &mut W) -> SharedResult<()> {
        Ok(())
    }
}

/// NPC requests input from player
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NPCRequestInput {
    pub message: String,
    pub max_length: u8,
}

impl Packet for NPCRequestInput {
    const OPCODE: i16 = ServerPacketIds::NPCRequestInput as i16;
    
    fn read_body<R: Read>(reader: &mut R) -> SharedResult<Self> {
        let message = read_dotnet_string(reader)?;
        let max_length = reader.read_u8()?;
        Ok(Self { message, max_length })
    }
    
    fn write_body<W: Write>(&self, writer: &mut W) -> SharedResult<()> {
        write_dotnet_string(writer, &self.message)?;
        writer.write_u8(self.max_length)?;
        Ok(())
    }
}
