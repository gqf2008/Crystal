//! Quest System Packets
//!
//! This module contains all quest-related packet definitions and parsers.

use std::io::{Read, Write};
use crate::{
    data::client_data::{ClientQuestInfo, ClientQuestProgress},
    enums::ServerPacketIds,
};
use super::super::base::Packet;
use crate::data::stats::SharedResult;

// ============================================================================
// Packet Structures & PacketMessage Implementations
// ============================================================================

/// Quest status changed
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ChangeQuest {
    pub quest: ClientQuestProgress,
}

impl Packet for ChangeQuest {
    const OPCODE: i16 = ServerPacketIds::ChangeQuest as i16;

    fn read_body<R: Read>(reader: &mut R) -> SharedResult<Self> {
        let quest = ClientQuestProgress::read_from(reader)?;
        Ok(Self { quest })
    }

    fn write_body<W: Write>(&self, writer: &mut W) -> SharedResult<()> {
        // Note: ClientQuestProgress doesn't have write_to yet, manual implementation
        use byteorder::WriteBytesExt;
        writer.write_i32::<byteorder::LittleEndian>(self.quest.id)?;
        writer.write_i32::<byteorder::LittleEndian>(self.quest.task_list.len() as i32)?;
        for task in &self.quest.task_list {
            crate::binary::write_dotnet_string(writer, task)?;
        }
        writer.write_u8(if self.quest.taken { 1 } else { 0 })?;
        writer.write_u8(if self.quest.completed { 1 } else { 0 })?;
        writer.write_u8(if self.quest.new { 1 } else { 0 })?;
        Ok(())
    }
}

/// New quest information
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NewQuestInfo {
    pub quest: ClientQuestInfo,
}

impl Packet for NewQuestInfo {
    const OPCODE: i16 = ServerPacketIds::NewQuestInfo as i16;

    fn read_body<R: Read>(reader: &mut R) -> SharedResult<Self> {
        let quest = ClientQuestInfo::read_from(reader)?;
        Ok(Self { quest })
    }

    fn write_body<W: Write>(&self, writer: &mut W) -> SharedResult<()> {
        // Note: ClientQuestInfo doesn't have write_to yet
        // This is a complex structure that would need full implementation
        // For now, return an error indicating not implemented
        Err(std::io::Error::new(std::io::ErrorKind::Unsupported, "ClientQuestInfo write not implemented").into())
    }
}
