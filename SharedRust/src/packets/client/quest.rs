//! Quest System Packets (Client → Server)

use std::io::{Read, Write};
use byteorder::{LittleEndian, ReadBytesExt, WriteBytesExt};
use crate::enums::ClientPacketIds;
use super::super::base::Packet;
use crate::data::stats::SharedResult;

/// Accept quest from NPC
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct AcceptQuest {
    pub npc_index: u32,
    pub quest_index: i32,
}

impl Packet for AcceptQuest {
    const OPCODE: i16 = ClientPacketIds::AcceptQuest as i16;

    fn read_body<R: Read>(reader: &mut R) -> SharedResult<Self> {
        let npc_index = reader.read_u32::<LittleEndian>()?;
        let quest_index = reader.read_i32::<LittleEndian>()?;
        Ok(Self { npc_index, quest_index })
    }

    fn write_body<W: Write>(&self, writer: &mut W) -> SharedResult<()> {
        writer.write_u32::<LittleEndian>(self.npc_index)?;
        writer.write_i32::<LittleEndian>(self.quest_index)?;
        Ok(())
    }
}

/// Finish quest (turn in)
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct FinishQuest {
    pub quest_index: i32,
    pub selected_item_index: i32,
}

impl Packet for FinishQuest {
    const OPCODE: i16 = ClientPacketIds::FinishQuest as i16;

    fn read_body<R: Read>(reader: &mut R) -> SharedResult<Self> {
        let quest_index = reader.read_i32::<LittleEndian>()?;
        let selected_item_index = reader.read_i32::<LittleEndian>()?;
        Ok(Self { quest_index, selected_item_index })
    }

    fn write_body<W: Write>(&self, writer: &mut W) -> SharedResult<()> {
        writer.write_i32::<LittleEndian>(self.quest_index)?;
        writer.write_i32::<LittleEndian>(self.selected_item_index)?;
        Ok(())
    }
}

/// Abandon quest
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct AbandonQuest {
    pub quest_index: i32,
}

impl Packet for AbandonQuest {
    const OPCODE: i16 = ClientPacketIds::AbandonQuest as i16;

    fn read_body<R: Read>(reader: &mut R) -> SharedResult<Self> {
        let quest_index = reader.read_i32::<LittleEndian>()?;
        Ok(Self { quest_index })
    }

    fn write_body<W: Write>(&self, writer: &mut W) -> SharedResult<()> {
        writer.write_i32::<LittleEndian>(self.quest_index)?;
        Ok(())
    }
}

/// Share quest with party
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ShareQuest {
    pub quest_index: i32,
}

impl Packet for ShareQuest {
    const OPCODE: i16 = ClientPacketIds::ShareQuest as i16;

    fn read_body<R: Read>(reader: &mut R) -> SharedResult<Self> {
        let quest_index = reader.read_i32::<LittleEndian>()?;
        Ok(Self { quest_index })
    }

    fn write_body<W: Write>(&self, writer: &mut W) -> SharedResult<()> {
        writer.write_i32::<LittleEndian>(self.quest_index)?;
        Ok(())
    }
}
