//! Monster / NPC / Item Information Request Packets (Client → Server)
//!
//! PR #1126 — KR NPC/Quest Linking. Clients request detailed info for tooltips.

use super::super::base::Packet;
use crate::data::stats::SharedResult;
use crate::enums::ClientPacketIds;
use byteorder::{LittleEndian, ReadBytesExt, WriteBytesExt};
use std::io::{Read, Write};

/// Client requests detailed monster info (used for tooltip when hovering <$MONSTER:IDX>).
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct RequestMonsterInfo {
    pub monster_index: i32,
}

impl Packet for RequestMonsterInfo {
    const OPCODE: i16 = ClientPacketIds::RequestMonsterInfo as i16;

    fn read_body<R: Read>(reader: &mut R) -> SharedResult<Self> {
        Ok(Self {
            monster_index: reader.read_i32::<LittleEndian>()?,
        })
    }

    fn write_body<W: Write>(&self, writer: &mut W) -> SharedResult<()> {
        writer.write_i32::<LittleEndian>(self.monster_index)?;
        Ok(())
    }
}

/// Client requests detailed NPC info (used for tooltip when hovering <$NPC:IDX>).
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct RequestNPCInfo {
    pub npc_index: i32,
}

impl Packet for RequestNPCInfo {
    const OPCODE: i16 = ClientPacketIds::RequestNPCInfo as i16;

    fn read_body<R: Read>(reader: &mut R) -> SharedResult<Self> {
        Ok(Self {
            npc_index: reader.read_i32::<LittleEndian>()?,
        })
    }

    fn write_body<W: Write>(&self, writer: &mut W) -> SharedResult<()> {
        writer.write_i32::<LittleEndian>(self.npc_index)?;
        Ok(())
    }
}

/// Client requests detailed item info (used for tooltip when hovering <$ITEM:IDX>).
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct RequestItemInfo {
    pub item_index: i32,
}

impl Packet for RequestItemInfo {
    const OPCODE: i16 = ClientPacketIds::RequestItemInfo as i16;

    fn read_body<R: Read>(reader: &mut R) -> SharedResult<Self> {
        Ok(Self {
            item_index: reader.read_i32::<LittleEndian>()?,
        })
    }

    fn write_body<W: Write>(&self, writer: &mut W) -> SharedResult<()> {
        writer.write_i32::<LittleEndian>(self.item_index)?;
        Ok(())
    }
}
