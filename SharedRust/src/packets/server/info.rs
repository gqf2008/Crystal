//! Monster / NPC Detailed Information Packets (Server → Client)
//!
//! PR #1126 — KR NPC/Quest Linking. The server's response to
//! `client::info::RequestMonsterInfo` / `RequestNPCInfo` / `RequestItemInfo`.
//!
//! `NewMonsterInfo` and `NewNPCInfo` carry a `ClientMonsterInfo` / `ClientNPCInfo`
//! payload respectively, used to render rich hover tooltips.

use std::io::{Read, Write};

use super::super::base::Packet;
use crate::data::client_data::{ClientMonsterInfo, ClientNPCInfo};
use crate::data::stats::SharedResult;
use crate::enums::ServerPacketIds;

/// Server response to `RequestMonsterInfo`: detailed monster info for tooltip.
#[derive(Debug, Clone)]
pub struct NewMonsterInfo {
    pub info: ClientMonsterInfo,
}

impl Packet for NewMonsterInfo {
    const OPCODE: i16 = ServerPacketIds::NewMonsterInfo as i16;

    fn read_body<R: Read>(reader: &mut R) -> SharedResult<Self> {
        let info = ClientMonsterInfo::read_from(reader)?;
        Ok(Self { info })
    }

    fn write_body<W: Write>(&self, writer: &mut W) -> SharedResult<()> {
        self.info.write_to(writer)?;
        Ok(())
    }
}

/// Server response to `RequestNPCInfo`: detailed NPC info for tooltip.
#[derive(Debug, Clone)]
pub struct NewNPCInfo {
    pub info: ClientNPCInfo,
}

impl Packet for NewNPCInfo {
    const OPCODE: i16 = ServerPacketIds::NewNPCInfo as i16;

    fn read_body<R: Read>(reader: &mut R) -> SharedResult<Self> {
        let info = ClientNPCInfo::read_from(reader)?;
        Ok(Self { info })
    }

    fn write_body<W: Write>(&self, writer: &mut W) -> SharedResult<()> {
        self.info.write_to(writer)?;
        Ok(())
    }
}
