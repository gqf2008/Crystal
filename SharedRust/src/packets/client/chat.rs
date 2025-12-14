//! Chat Packets (Client → Server)

use std::io::{Read, Write};
use byteorder::{ReadBytesExt, WriteBytesExt};
use crate::binary::{read_dotnet_string, write_dotnet_string};
use crate::data::item::ChatItem;
use crate::enums::ClientPacketIds;
use super::super::base::Packet;
use crate::data::stats::SharedResult;

/// Client sends a chat message
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Chat {
    pub message: String,
    pub linked_items: Vec<ChatItem>,
}

impl Default for Chat {
    fn default() -> Self {
        Self {
            message: String::new(),
            linked_items: Vec::new(),
        }
    }
}

impl Packet for Chat {
    const OPCODE: i16 = ClientPacketIds::Chat as i16;

    fn read_body<R: Read>(reader: &mut R) -> SharedResult<Self> {
        use byteorder::LittleEndian;

        let message = read_dotnet_string(reader)?;
        let count = reader.read_i32::<LittleEndian>()?;

        const MAX_LINKED_ITEMS: i32 = 32;
        if count < 0 {
            return Err(crate::data::stats::SharedError::NegativeLength {
                field: "linked_items_count",
                length: count,
            });
        }
        if count > MAX_LINKED_ITEMS {
            return Err(crate::data::stats::SharedError::LengthTooLarge {
                field: "linked_items_count",
                length: count,
                max: MAX_LINKED_ITEMS,
            });
        }

        let mut linked_items = Vec::with_capacity(count as usize);
        for _ in 0..count {
            linked_items.push(ChatItem::read_from(reader)?);
        }

        Ok(Self {
            message,
            linked_items,
        })
    }

    fn write_body<W: Write>(&self, writer: &mut W) -> SharedResult<()> {
        use byteorder::LittleEndian;

        write_dotnet_string(writer, &self.message)?;
        let count = i32::try_from(self.linked_items.len()).map_err(|_| {
            crate::data::stats::SharedError::PacketTooLarge(self.linked_items.len())
        })?;
        writer.write_i32::<LittleEndian>(count)?;
        for item in &self.linked_items {
            item.write_to(writer)?;
        }
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
