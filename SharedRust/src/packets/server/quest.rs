//! Quest System Packets
//!
//! This module contains all quest-related packet definitions and parsers.

use super::super::base::Packet;
use crate::data::stats::SharedResult;
use crate::{
    data::client_data::{ClientQuestInfo, ClientQuestProgress},
    enums::ServerPacketIds,
};
use std::io::{Read, Write};

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
        self.quest.write_to(writer)
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
        self.quest.write_to(writer)
    }
}
