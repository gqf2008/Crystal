//! Storage (Warehouse) Password Result Packets (Server → Client)
//!
//! PR #1169 — KR Mir2 Warehouse password feature. The server's response to
//! UnlockStorage / SetStoragePassword / RemoveStoragePassword requests.

use std::io::{Read, Write};
use byteorder::{LittleEndian, ReadBytesExt, WriteBytesExt};
use crate::data::stats::SharedResult;
use crate::enums::ServerPacketIds;
use super::super::base::Packet;

/// Result of an UnlockStorage request.
///
/// `result` codes (from master `Shared/ServerPackets.cs`):
/// * `0` Success
/// * `1` Bad Password
/// * `2` Wrong Password
/// * `3` Not Available
/// * `4` No Password Set
#[derive(Debug, Clone)]
pub struct StorageUnlockResult {
    pub result: u8,
    pub has_password: bool,
}

impl Packet for StorageUnlockResult {
    const OPCODE: i16 = ServerPacketIds::StorageUnlockResult as i16;

    fn read_body<R: Read>(reader: &mut R) -> SharedResult<Self> {
        let result = reader.read_u8()?;
        let has_password = reader.read_u8()? != 0;
        Ok(Self { result, has_password })
    }

    fn write_body<W: Write>(&self, writer: &mut W) -> SharedResult<()> {
        writer.write_u8(self.result)?;
        writer.write_u8(self.has_password as u8)?;
        Ok(())
    }
}

/// Result of a SetStoragePassword / RemoveStoragePassword request.
///
/// `result` codes:
/// * `0` Not Available
/// * `1` Bad Current Password
/// * `2` Wrong Current Password
/// * `3` Bad New Password
/// * `4` Success
/// * `5` No Password Set
#[derive(Debug, Clone)]
pub struct StoragePasswordResult {
    pub result: u8,
    pub removing: bool,
    pub has_password: bool,
    pub last_set_time: i64, // DateTime.ToBinary()
}

impl Packet for StoragePasswordResult {
    const OPCODE: i16 = ServerPacketIds::StoragePasswordResult as i16;

    fn read_body<R: Read>(reader: &mut R) -> SharedResult<Self> {
        let result = reader.read_u8()?;
        let removing = reader.read_u8()? != 0;
        let has_password = reader.read_u8()? != 0;
        let last_set_time = reader.read_i64::<LittleEndian>()?;
        Ok(Self { result, removing, has_password, last_set_time })
    }

    fn write_body<W: Write>(&self, writer: &mut W) -> SharedResult<()> {
        writer.write_u8(self.result)?;
        writer.write_u8(self.removing as u8)?;
        writer.write_u8(self.has_password as u8)?;
        writer.write_i64::<LittleEndian>(self.last_set_time)?;
        Ok(())
    }
}
