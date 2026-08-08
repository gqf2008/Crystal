//! Chat Packets (Client → Server)

use super::super::base::Packet;
use crate::binary::{read_dotnet_string, write_dotnet_string};
use crate::data::item::ChatItem;
use crate::data::stats::SharedResult;
use crate::enums::ClientPacketIds;
use byteorder::{ReadBytesExt, WriteBytesExt};
use std::io::{Read, Write};

/// Client sends a chat message
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct Chat {
    pub message: String,
    pub linked_items: Vec<ChatItem>,
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
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Inspect {
    pub object_id: u32,
    /// 排行榜查看（C# Inspect.Ranking）
    pub ranking: bool,
    /// 排行榜查看时离线玩家回查（Rust 无持久化角色 id，用名字查 DB）
    pub name: String,
}

impl Packet for Inspect {
    const OPCODE: i16 = ClientPacketIds::Inspect as i16;

    fn read_body<R: Read>(reader: &mut R) -> SharedResult<Self> {
        let object_id = reader.read_u32::<byteorder::LittleEndian>()?;
        let ranking = reader.read_u8()? != 0;
        let name = read_dotnet_string(reader)?;
        Ok(Self {
            object_id,
            ranking,
            name,
        })
    }

    fn write_body<W: Write>(&self, writer: &mut W) -> SharedResult<()> {
        writer.write_u32::<byteorder::LittleEndian>(self.object_id)?;
        writer.write_u8(self.ranking as u8)?;
        write_dotnet_string(writer, &self.name)?;
        Ok(())
    }
}

/// Client requests to observe another player
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct Observe {
    pub name: String,
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
