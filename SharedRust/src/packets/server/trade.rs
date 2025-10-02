//! Trade System Packets
//!
//! This module contains all trade-related packet definitions and parsers.

use std::io::{Read, Write};
use byteorder::{LittleEndian, ReadBytesExt, WriteBytesExt};
use crate::{
    data::item::UserItem,
    enums::ServerPacketIds,
    binary::{read_dotnet_string, write_dotnet_string},
};
use super::super::base::PacketMessage;
use crate::data::stats::SharedResult;

// ==================== 数据结构 & PacketMessage 实现 ====================

#[derive(Debug, Clone)]
pub struct TradeRequest {
    pub name: String,
}

impl PacketMessage for TradeRequest {
    const OPCODE: i16 = ServerPacketIds::TradeRequest as i16;

    fn read_body<R: Read>(reader: &mut R) -> SharedResult<Self> {
        let name = read_dotnet_string(reader)?;
        Ok(Self { name })
    }

    fn write_body<W: Write>(&self, writer: &mut W) -> SharedResult<()> {
        write_dotnet_string(writer, &self.name)?;
        Ok(())
    }
}

#[derive(Debug, Clone)]
pub struct TradeAccept {
    pub name: String,
}

impl PacketMessage for TradeAccept {
    const OPCODE: i16 = ServerPacketIds::TradeAccept as i16;

    fn read_body<R: Read>(reader: &mut R) -> SharedResult<Self> {
        let name = read_dotnet_string(reader)?;
        Ok(Self { name })
    }

    fn write_body<W: Write>(&self, writer: &mut W) -> SharedResult<()> {
        write_dotnet_string(writer, &self.name)?;
        Ok(())
    }
}

#[derive(Debug, Clone)]
pub struct TradeGold {
    pub amount: u32,
}

impl PacketMessage for TradeGold {
    const OPCODE: i16 = ServerPacketIds::TradeGold as i16;

    fn read_body<R: Read>(reader: &mut R) -> SharedResult<Self> {
        let amount = reader.read_u32::<LittleEndian>()?;
        Ok(Self { amount })
    }

    fn write_body<W: Write>(&self, writer: &mut W) -> SharedResult<()> {
        writer.write_u32::<LittleEndian>(self.amount)?;
        Ok(())
    }
}

#[derive(Debug, Clone)]
pub struct TradeItem {
    pub trade_items: Vec<Option<UserItem>>,
}

impl PacketMessage for TradeItem {
    const OPCODE: i16 = ServerPacketIds::TradeItem as i16;

    fn read_body<R: Read>(reader: &mut R) -> SharedResult<Self> {
        let count = reader.read_i32::<LittleEndian>()? as usize;
        let mut trade_items = Vec::with_capacity(count);
        
        for _ in 0..count {
            let has_item = reader.read_u8()? != 0;
            if has_item {
                let item = UserItem::read_default(reader)?;
                trade_items.push(Some(item));
            } else {
                trade_items.push(None);
            }
        }

        Ok(Self { trade_items })
    }

    fn write_body<W: Write>(&self, writer: &mut W) -> SharedResult<()> {
        writer.write_i32::<LittleEndian>(self.trade_items.len() as i32)?;
        
        for item_opt in &self.trade_items {
            if let Some(item) = item_opt {
                writer.write_u8(1)?;
                item.write_to(writer)?;
            } else {
                writer.write_u8(0)?;
            }
        }

        Ok(())
    }
}

#[derive(Debug, Clone)]
pub struct TradeConfirm;

impl PacketMessage for TradeConfirm {
    const OPCODE: i16 = ServerPacketIds::TradeConfirm as i16;

    fn read_body<R: Read>(_reader: &mut R) -> SharedResult<Self> {
        Ok(Self)
    }

    fn write_body<W: Write>(&self, _writer: &mut W) -> SharedResult<()> {
        Ok(())
    }
}

#[derive(Debug, Clone)]
pub struct TradeCancel {
    pub unlock: bool,
}

impl PacketMessage for TradeCancel {
    const OPCODE: i16 = ServerPacketIds::TradeCancel as i16;

    fn read_body<R: Read>(reader: &mut R) -> SharedResult<Self> {
        let unlock = reader.read_u8()? != 0;
        Ok(Self { unlock })
    }

    fn write_body<W: Write>(&self, writer: &mut W) -> SharedResult<()> {
        writer.write_u8(if self.unlock { 1 } else { 0 })?;
        Ok(())
    }
}
