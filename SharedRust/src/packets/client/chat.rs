//! Chat Packets (Client → Server)

use std::io::{Read, Write};
use byteorder::{ReadBytesExt, WriteBytesExt};
use crate::binary::{read_dotnet_string, write_dotnet_string};
use crate::enums::ClientPacketIds;
use super::super::base::Packet;
use crate::data::stats::SharedResult;

/// Client sends a chat message
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Chat {
    pub message: String,
}

impl Default for Chat {
    fn default() -> Self {
        Self {
            message: String::new(),
        }
    }
}

impl Packet for Chat {
    const OPCODE: i16 = ClientPacketIds::Chat as i16;

    fn read_body<R: Read>(reader: &mut R) -> SharedResult<Self> {
        Ok(Self {
            message: read_dotnet_string(reader)?,
        })
    }

    fn write_body<W: Write>(&self, writer: &mut W) -> SharedResult<()> {
        write_dotnet_string(writer, &self.message)?;
        Ok(())
    }
}

/// Client requests to inspect another object
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Inspect {
    pub object_id: u32,
}

impl Packet for Inspect {
    const OPCODE: i16 = ClientPacketIds::Inspect as i16;

    fn read_body<R: Read>(reader: &mut R) -> SharedResult<Self> {
        let object_id = reader.read_u32::<byteorder::LittleEndian>()?;
        Ok(Self { object_id })
    }

    fn write_body<W: Write>(&self, writer: &mut W) -> SharedResult<()> {
        writer.write_u32::<byteorder::LittleEndian>(self.object_id)?;
        Ok(())
    }
}

/// Client requests to observe another player
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Observe {
    pub name: String,
}

impl Default for Observe {
    fn default() -> Self {
        Self {
            name: String::new(),
        }
    }
}

impl Packet for Observe {
    const OPCODE: i16 = ClientPacketIds::Observe as i16;

    fn read_body<R: Read>(reader: &mut R) -> SharedResult<Self> {
        Ok(Self {
            name: read_dotnet_string(reader)?,
        })
    }

    fn write_body<W: Write>(&self, writer: &mut W) -> SharedResult<()> {
        write_dotnet_string(writer, &self.name)?;
        Ok(())
    }
}
