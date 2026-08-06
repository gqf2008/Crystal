//! Connection Management Packets
//!
//! Packets related to connection lifecycle and keep-alive.

use super::super::base::Packet;
use crate::data::stats::SharedResult;
use crate::enums::ServerPacketIds;
use byteorder::{LittleEndian, ReadBytesExt, WriteBytesExt};
use std::io::{Read, Write};

/// KeepAlive packet - sent periodically to maintain connection
#[derive(Debug, Clone)]
pub struct KeepAlive {
    pub time: i64,
}

impl Packet for KeepAlive {
    const OPCODE: i16 = ServerPacketIds::KeepAlive as i16;

    fn read_body<R: Read>(reader: &mut R) -> SharedResult<Self> {
        let time = reader.read_i64::<LittleEndian>()?;
        Ok(Self { time })
    }

    fn write_body<W: Write>(&self, writer: &mut W) -> SharedResult<()> {
        writer.write_i64::<LittleEndian>(self.time)?;
        Ok(())
    }
}

/// Connected packet - sent when client successfully connects
#[derive(Debug, Clone)]
pub struct Connected;

impl Packet for Connected {
    const OPCODE: i16 = ServerPacketIds::Connected as i16;

    fn read_body<R: Read>(_reader: &mut R) -> SharedResult<Self> {
        Ok(Self)
    }

    fn write_body<W: Write>(&self, _writer: &mut W) -> SharedResult<()> {
        Ok(())
    }
}

/// ClientVersion packet - response to client version check
#[derive(Debug, Clone)]
pub struct ClientVersion {
    pub result: u8,
    // 0: Wrong Version
    // 1: Correct Version
}

impl Packet for ClientVersion {
    const OPCODE: i16 = ServerPacketIds::ClientVersion as i16;

    fn read_body<R: Read>(reader: &mut R) -> SharedResult<Self> {
        let result = reader.read_u8()?;
        Ok(Self { result })
    }

    fn write_body<W: Write>(&self, writer: &mut W) -> SharedResult<()> {
        writer.write_u8(self.result)?;
        Ok(())
    }
}

/// Disconnect packet - notifies client of disconnection
#[derive(Debug, Clone)]
pub struct Disconnect {
    pub reason: u8,
    // 0: Server Closing
    // 1: Another User
    // 2: Packet Error
    // 3: Server Crashed
}

impl Packet for Disconnect {
    const OPCODE: i16 = ServerPacketIds::Disconnect as i16;

    fn read_body<R: Read>(reader: &mut R) -> SharedResult<Self> {
        let reason = reader.read_u8()?;
        Ok(Self { reason })
    }

    fn write_body<W: Write>(&self, writer: &mut W) -> SharedResult<()> {
        writer.write_u8(self.reason)?;
        Ok(())
    }
}
