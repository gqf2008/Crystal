//! Item System Packets
//!
//! This module contains all item-related packet definitions and parsers.

use std::io::{Read, Write};
use byteorder::{LittleEndian, ReadBytesExt, WriteBytesExt};
use crate::{
    enums::{MirGridType, ServerPacketIds},
    data::item::{ItemInfo, UserItem},
};
use super::super::base::Packet;
use crate::data::stats::SharedResult;

// ============================================================================
// Packet Structures & PacketMessage Implementations
// ============================================================================

/// Item sold to NPC
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SellItem {
    pub unique_id: u64,
    pub count: u16,
    pub success: bool,
}

impl Packet for SellItem {
    const OPCODE: i16 = ServerPacketIds::SellItem as i16;

    fn read_body<R: Read>(reader: &mut R) -> SharedResult<Self> {
        let unique_id = reader.read_u64::<LittleEndian>()?;
        let count = reader.read_u16::<LittleEndian>()?;
        let success = reader.read_u8()? != 0;
        Ok(Self { unique_id, count, success })
    }

    fn write_body<W: Write>(&self, writer: &mut W) -> SharedResult<()> {
        writer.write_u64::<LittleEndian>(self.unique_id)?;
        writer.write_u16::<LittleEndian>(self.count)?;
        writer.write_u8(if self.success { 1 } else { 0 })?;
        Ok(())
    }
}

/// Item sent for repair
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct RepairItem {
    pub unique_id: u64,
}

impl Packet for RepairItem {
    const OPCODE: i16 = ServerPacketIds::RepairItem as i16;

    fn read_body<R: Read>(reader: &mut R) -> SharedResult<Self> {
        let unique_id = reader.read_u64::<LittleEndian>()?;
        Ok(Self { unique_id })
    }

    fn write_body<W: Write>(&self, writer: &mut W) -> SharedResult<()> {
        writer.write_u64::<LittleEndian>(self.unique_id)?;
        Ok(())
    }
}

/// Item repair completed
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ItemRepaired {
    pub unique_id: u64,
    pub max_dura: u16,
    pub current_dura: u16,
}

impl Packet for ItemRepaired {
    const OPCODE: i16 = ServerPacketIds::ItemRepaired as i16;

    fn read_body<R: Read>(reader: &mut R) -> SharedResult<Self> {
        let unique_id = reader.read_u64::<LittleEndian>()?;
        let max_dura = reader.read_u16::<LittleEndian>()?;
        let current_dura = reader.read_u16::<LittleEndian>()?;
        Ok(Self { unique_id, max_dura, current_dura })
    }

    fn write_body<W: Write>(&self, writer: &mut W) -> SharedResult<()> {
        writer.write_u64::<LittleEndian>(self.unique_id)?;
        writer.write_u16::<LittleEndian>(self.max_dura)?;
        writer.write_u16::<LittleEndian>(self.current_dura)?;
        Ok(())
    }
}

/// Split item stack
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SplitItem {
    pub grid: MirGridType,
    pub unique_id: u64,
    pub count: u16,
}

impl Packet for SplitItem {
    const OPCODE: i16 = ServerPacketIds::SplitItem as i16;

    fn read_body<R: Read>(reader: &mut R) -> SharedResult<Self> {
        let grid = MirGridType::try_from(reader.read_u8()?)?;
        let unique_id = reader.read_u64::<LittleEndian>()?;
        let count = reader.read_u16::<LittleEndian>()?;
        Ok(Self { grid, unique_id, count })
    }

    fn write_body<W: Write>(&self, writer: &mut W) -> SharedResult<()> {
        writer.write_u8(self.grid as u8)?;
        writer.write_u64::<LittleEndian>(self.unique_id)?;
        writer.write_u16::<LittleEndian>(self.count)?;
        Ok(())
    }
}

/// Split item stack (variant 1)
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SplitItem1 {
    pub grid: MirGridType,
    pub unique_id: u64,
    pub count: u16,
}

impl Packet for SplitItem1 {
    const OPCODE: i16 = ServerPacketIds::SplitItem1 as i16;

    fn read_body<R: Read>(reader: &mut R) -> SharedResult<Self> {
        let grid = MirGridType::try_from(reader.read_u8()?)?;
        let unique_id = reader.read_u64::<LittleEndian>()?;
        let count = reader.read_u16::<LittleEndian>()?;
        Ok(Self { grid, unique_id, count })
    }

    fn write_body<W: Write>(&self, writer: &mut W) -> SharedResult<()> {
        writer.write_u8(self.grid as u8)?;
        writer.write_u64::<LittleEndian>(self.unique_id)?;
        writer.write_u16::<LittleEndian>(self.count)?;
        Ok(())
    }
}

/// Refresh item data
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RefreshItem {
    pub item: UserItem,
}

impl Packet for RefreshItem {
    const OPCODE: i16 = ServerPacketIds::RefreshItem as i16;

    fn read_body<R: Read>(reader: &mut R) -> SharedResult<Self> {
        let item = UserItem::read_default(reader)?;
        Ok(Self { item })
    }

    fn write_body<W: Write>(&self, writer: &mut W) -> SharedResult<()> {
        self.item.write_to(writer)?;
        Ok(())
    }
}

/// Item slot size changed
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ItemSlotSizeChanged {
    pub unique_id: u64,
    pub slot_size: i32,
}

impl Packet for ItemSlotSizeChanged {
    const OPCODE: i16 = ServerPacketIds::ItemSlotSizeChanged as i16;

    fn read_body<R: Read>(reader: &mut R) -> SharedResult<Self> {
        let unique_id = reader.read_u64::<LittleEndian>()?;
        let slot_size = reader.read_i32::<LittleEndian>()?;
        Ok(Self { unique_id, slot_size })
    }

    fn write_body<W: Write>(&self, writer: &mut W) -> SharedResult<()> {
        writer.write_u64::<LittleEndian>(self.unique_id)?;
        writer.write_i32::<LittleEndian>(self.slot_size)?;
        Ok(())
    }
}

/// Item seal status changed
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ItemSealChanged {
    pub grid_type: MirGridType,
    pub unique_id: u64,
    pub expiry_date: i64,
}

impl Packet for ItemSealChanged {
    const OPCODE: i16 = ServerPacketIds::ItemSealChanged as i16;

    fn read_body<R: Read>(reader: &mut R) -> SharedResult<Self> {
        let grid_type = MirGridType::try_from(reader.read_u8()?)?;
        let unique_id = reader.read_u64::<LittleEndian>()?;
        let expiry_date = reader.read_i64::<LittleEndian>()?;
        Ok(Self { grid_type, unique_id, expiry_date })
    }

    fn write_body<W: Write>(&self, writer: &mut W) -> SharedResult<()> {
        writer.write_u8(self.grid_type as u8)?;
        writer.write_u64::<LittleEndian>(self.unique_id)?;
        writer.write_i64::<LittleEndian>(self.expiry_date)?;
        Ok(())
    }
}

/// Item crafting result
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct CraftItem {
    pub unique_id: u64,
    pub count: u16,
    pub success: bool,
}

impl Packet for CraftItem {
    const OPCODE: i16 = ServerPacketIds::CraftItem as i16;

    fn read_body<R: Read>(reader: &mut R) -> SharedResult<Self> {
        let unique_id = reader.read_u64::<LittleEndian>()?;
        let count = reader.read_u16::<LittleEndian>()?;
        let success = reader.read_u8()? != 0;
        Ok(Self { unique_id, count, success })
    }

    fn write_body<W: Write>(&self, writer: &mut W) -> SharedResult<()> {
        writer.write_u64::<LittleEndian>(self.unique_id)?;
        writer.write_u16::<LittleEndian>(self.count)?;
        writer.write_u8(if self.success { 1 } else { 0 })?;
        Ok(())
    }
}

/// New item information received
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NewItemInfo {
    pub info: ItemInfo,
}

impl Packet for NewItemInfo {
    const OPCODE: i16 = ServerPacketIds::NewItemInfo as i16;

    fn read_body<R: Read>(reader: &mut R) -> SharedResult<Self> {
        let info = ItemInfo::read_default(reader)?;
        Ok(Self { info })
    }

    fn write_body<W: Write>(&self, writer: &mut W) -> SharedResult<()> {
        self.info.write_to(writer)?;
        Ok(())
    }
}
