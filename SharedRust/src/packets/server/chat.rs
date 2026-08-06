//! Chat System Packets (Server → Client)

use super::super::base::Packet;
use crate::binary::{read_dotnet_string, write_dotnet_string};
use crate::data::stats::SharedResult;
use crate::enums::{ChatType, ServerPacketIds};
use byteorder::ReadBytesExt;
use std::io::{Read, Write};

/// Chat message from server
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Chat {
    pub message: String,
    pub chat_type: ChatType,
}

impl Packet for Chat {
    const OPCODE: i16 = ServerPacketIds::Chat as i16;

    fn read_body<R: Read>(reader: &mut R) -> SharedResult<Self> {
        let message = read_dotnet_string(reader)?;
        let chat_type = ChatType::try_from(reader.read_u8()?)?;
        Ok(Self { message, chat_type })
    }

    fn write_body<W: Write>(&self, writer: &mut W) -> SharedResult<()> {
        write_dotnet_string(writer, &self.message)?;
        writer.write_all(&[self.chat_type as u8])?;
        Ok(())
    }
}

/// Object chat message (NPC or player speech bubble)
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ObjectChat {
    pub object_id: u32,
    pub text: String,
    pub chat_type: ChatType,
}

impl Packet for ObjectChat {
    const OPCODE: i16 = ServerPacketIds::ObjectChat as i16;

    fn read_body<R: Read>(reader: &mut R) -> SharedResult<Self> {
        use byteorder::{LittleEndian, ReadBytesExt};

        let object_id = reader.read_u32::<LittleEndian>()?;
        let text = read_dotnet_string(reader)?;
        let chat_type = ChatType::try_from(reader.read_u8()?)?;

        Ok(Self {
            object_id,
            text,
            chat_type,
        })
    }

    fn write_body<W: Write>(&self, writer: &mut W) -> SharedResult<()> {
        use byteorder::{LittleEndian, WriteBytesExt};

        writer.write_u32::<LittleEndian>(self.object_id)?;
        write_dotnet_string(writer, &self.text)?;
        writer.write_all(&[self.chat_type as u8])?;
        Ok(())
    }
}
