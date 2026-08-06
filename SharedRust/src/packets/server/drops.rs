//! Drop Items and Currency Packets
//!
//! Packets related to dropped items, gold, and credit on the ground.

use super::super::base::Packet;
use crate::data::item::UserItem;
use crate::data::stats::SharedResult;
use crate::enums::ServerPacketIds;
use byteorder::{LittleEndian, ReadBytesExt, WriteBytesExt};
use std::io::{Read, Write};

/// ObjectItem packet - item on the ground
#[derive(Debug, Clone)]
pub struct ObjectItem {
    pub object_id: u32,
    pub item: UserItem,
    pub location_x: i32,
    pub location_y: i32,
}

impl Packet for ObjectItem {
    const OPCODE: i16 = ServerPacketIds::ObjectItem as i16;

    fn read_body<R: Read>(reader: &mut R) -> SharedResult<Self> {
        let object_id = reader.read_u32::<LittleEndian>()?;
        // 携带 ItemInfo（与 UserInformation 一致），客户端渲染图标/名称
        let item = UserItem::read_from_with_info(reader)?;
        let location_x = reader.read_i32::<LittleEndian>()?;
        let location_y = reader.read_i32::<LittleEndian>()?;
        Ok(Self {
            object_id,
            item,
            location_x,
            location_y,
        })
    }

    fn write_body<W: Write>(&self, writer: &mut W) -> SharedResult<()> {
        writer.write_u32::<LittleEndian>(self.object_id)?;
        self.item.write_to_with_info(writer)?;
        writer.write_i32::<LittleEndian>(self.location_x)?;
        writer.write_i32::<LittleEndian>(self.location_y)?;
        Ok(())
    }
}

/// ObjectGold packet - gold on the ground
#[derive(Debug, Clone)]
pub struct ObjectGold {
    pub object_id: u32,
    pub gold: u32,
    pub location_x: i32,
    pub location_y: i32,
}

impl Packet for ObjectGold {
    const OPCODE: i16 = ServerPacketIds::ObjectGold as i16;

    fn read_body<R: Read>(reader: &mut R) -> SharedResult<Self> {
        let object_id = reader.read_u32::<LittleEndian>()?;
        let gold = reader.read_u32::<LittleEndian>()?;
        let location_x = reader.read_i32::<LittleEndian>()?;
        let location_y = reader.read_i32::<LittleEndian>()?;
        Ok(Self {
            object_id,
            gold,
            location_x,
            location_y,
        })
    }

    fn write_body<W: Write>(&self, writer: &mut W) -> SharedResult<()> {
        writer.write_u32::<LittleEndian>(self.object_id)?;
        writer.write_u32::<LittleEndian>(self.gold)?;
        writer.write_i32::<LittleEndian>(self.location_x)?;
        writer.write_i32::<LittleEndian>(self.location_y)?;
        Ok(())
    }
}

/// GainedItem packet - item gained notification
#[derive(Debug, Clone)]
pub struct GainedItem {
    pub item: UserItem,
}

impl Packet for GainedItem {
    const OPCODE: i16 = ServerPacketIds::GainedItem as i16;

    fn read_body<R: Read>(reader: &mut R) -> SharedResult<Self> {
        let item = UserItem::read_from(reader, i32::MAX, i32::MAX)?;
        Ok(Self { item })
    }

    fn write_body<W: Write>(&self, writer: &mut W) -> SharedResult<()> {
        self.item.write_to(writer)?;
        Ok(())
    }
}

/// GainedGold packet - gold gained notification
#[derive(Debug, Clone)]
pub struct GainedGold {
    pub gold: u32,
}

impl Packet for GainedGold {
    const OPCODE: i16 = ServerPacketIds::GainedGold as i16;

    fn read_body<R: Read>(reader: &mut R) -> SharedResult<Self> {
        let gold = reader.read_u32::<LittleEndian>()?;
        Ok(Self { gold })
    }

    fn write_body<W: Write>(&self, writer: &mut W) -> SharedResult<()> {
        writer.write_u32::<LittleEndian>(self.gold)?;
        Ok(())
    }
}

/// LoseGold packet - gold lost notification
#[derive(Debug, Clone)]
pub struct LoseGold {
    pub gold: u32,
}

impl Packet for LoseGold {
    const OPCODE: i16 = ServerPacketIds::LoseGold as i16;

    fn read_body<R: Read>(reader: &mut R) -> SharedResult<Self> {
        let gold = reader.read_u32::<LittleEndian>()?;
        Ok(Self { gold })
    }

    fn write_body<W: Write>(&self, writer: &mut W) -> SharedResult<()> {
        writer.write_u32::<LittleEndian>(self.gold)?;
        Ok(())
    }
}

/// GainedCredit packet - credit gained notification
#[derive(Debug, Clone)]
pub struct GainedCredit {
    pub credit: u32,
}

impl Packet for GainedCredit {
    const OPCODE: i16 = ServerPacketIds::GainedCredit as i16;

    fn read_body<R: Read>(reader: &mut R) -> SharedResult<Self> {
        let credit = reader.read_u32::<LittleEndian>()?;
        Ok(Self { credit })
    }

    fn write_body<W: Write>(&self, writer: &mut W) -> SharedResult<()> {
        writer.write_u32::<LittleEndian>(self.credit)?;
        Ok(())
    }
}

/// LoseCredit packet - credit lost notification
#[derive(Debug, Clone)]
pub struct LoseCredit {
    pub credit: u32,
}

impl Packet for LoseCredit {
    const OPCODE: i16 = ServerPacketIds::LoseCredit as i16;

    fn read_body<R: Read>(reader: &mut R) -> SharedResult<Self> {
        let credit = reader.read_u32::<LittleEndian>()?;
        Ok(Self { credit })
    }

    fn write_body<W: Write>(&self, writer: &mut W) -> SharedResult<()> {
        writer.write_u32::<LittleEndian>(self.credit)?;
        Ok(())
    }
}
