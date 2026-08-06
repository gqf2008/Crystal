//! Connection Packets (Client → Server)

use super::super::base::Packet;
use crate::data::stats::SharedResult;
use crate::enums::ClientPacketIds;
use byteorder::{LittleEndian, ReadBytesExt, WriteBytesExt};
use std::io::{Read, Write};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ClientVersion {
    pub version_hash: Vec<u8>,
}

impl Packet for ClientVersion {
    const OPCODE: i16 = ClientPacketIds::ClientVersion as i16;

    fn read_body<R: Read>(reader: &mut R) -> SharedResult<Self> {
        let length = reader.read_i32::<LittleEndian>()?;
        let length = usize::try_from(length).map_err(|_| {
            crate::data::stats::SharedError::NegativeLength {
                field: "version_hash",
                length,
            }
        })?;
        let mut version_hash = vec![0; length];
        reader.read_exact(&mut version_hash)?;
        Ok(Self { version_hash })
    }

    fn write_body<W: Write>(&self, writer: &mut W) -> SharedResult<()> {
        let length = i32::try_from(self.version_hash.len()).map_err(|_| {
            crate::data::stats::SharedError::PacketTooLarge(self.version_hash.len())
        })?;
        writer.write_i32::<LittleEndian>(length)?;
        writer.write_all(&self.version_hash)?;
        Ok(())
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct Disconnect;

impl Packet for Disconnect {
    const OPCODE: i16 = ClientPacketIds::Disconnect as i16;

    fn read_body<R: Read>(_: &mut R) -> SharedResult<Self> {
        Ok(Self)
    }

    fn write_body<W: Write>(&self, _: &mut W) -> SharedResult<()> {
        Ok(())
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct KeepAlive {
    pub time: i64,
}

impl Packet for KeepAlive {
    const OPCODE: i16 = ClientPacketIds::KeepAlive as i16;

    fn read_body<R: Read>(reader: &mut R) -> SharedResult<Self> {
        let time = reader.read_i64::<LittleEndian>()?;
        Ok(Self { time })
    }

    fn write_body<W: Write>(&self, writer: &mut W) -> SharedResult<()> {
        writer.write_i64::<LittleEndian>(self.time)?;
        Ok(())
    }
}
