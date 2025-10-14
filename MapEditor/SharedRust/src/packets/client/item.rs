//! Item Management Packets (Client → Server)

use std::io::{Read, Write};
use byteorder::{LittleEndian, ReadBytesExt, WriteBytesExt};
use crate::enums::{ClientPacketIds, MirGridType};
use super::super::base::Packet;
use crate::data::stats::SharedResult;

/// Client requests to move an item
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct MoveItem {
    pub grid: MirGridType,
    pub from: i32,
    pub to: i32,
}

impl Packet for MoveItem {
    const OPCODE: i16 = ClientPacketIds::MoveItem as i16;

    fn read_body<R: Read>(reader: &mut R) -> SharedResult<Self> {
        let grid = MirGridType::try_from(reader.read_u8()?).unwrap_or(MirGridType::Inventory);
        let from = reader.read_i32::<LittleEndian>()?;
        let to = reader.read_i32::<LittleEndian>()?;
        Ok(Self { grid, from, to })
    }

    fn write_body<W: Write>(&self, writer: &mut W) -> SharedResult<()> {
        writer.write_u8(self.grid as u8)?;
        writer.write_i32::<LittleEndian>(self.from)?;
        writer.write_i32::<LittleEndian>(self.to)?;
        Ok(())
    }
}

/// Client requests to store an item
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct StoreItem {
    pub from: i32,
    pub to: i32,
}

impl Packet for StoreItem {
    const OPCODE: i16 = ClientPacketIds::StoreItem as i16;

    fn read_body<R: Read>(reader: &mut R) -> SharedResult<Self> {
        let from = reader.read_i32::<LittleEndian>()?;
        let to = reader.read_i32::<LittleEndian>()?;
        Ok(Self { from, to })
    }

    fn write_body<W: Write>(&self, writer: &mut W) -> SharedResult<()> {
        writer.write_i32::<LittleEndian>(self.from)?;
        writer.write_i32::<LittleEndian>(self.to)?;
        Ok(())
    }
}

/// Client requests to take back an item from storage
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct TakeBackItem {
    pub from: i32,
    pub to: i32,
}

impl Packet for TakeBackItem {
    const OPCODE: i16 = ClientPacketIds::TakeBackItem as i16;

    fn read_body<R: Read>(reader: &mut R) -> SharedResult<Self> {
        let from = reader.read_i32::<LittleEndian>()?;
        let to = reader.read_i32::<LittleEndian>()?;
        Ok(Self { from, to })
    }

    fn write_body<W: Write>(&self, writer: &mut W) -> SharedResult<()> {
        writer.write_i32::<LittleEndian>(self.from)?;
        writer.write_i32::<LittleEndian>(self.to)?;
        Ok(())
    }
}

/// Client requests to merge two items
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct MergeItem {
    pub grid_from: MirGridType,
    pub grid_to: MirGridType,
    pub id_from: u64,
    pub id_to: u64,
}

impl Packet for MergeItem {
    const OPCODE: i16 = ClientPacketIds::MergeItem as i16;

    fn read_body<R: Read>(reader: &mut R) -> SharedResult<Self> {
        let grid_from = MirGridType::try_from(reader.read_u8()?).unwrap_or(MirGridType::Inventory);
        let grid_to = MirGridType::try_from(reader.read_u8()?).unwrap_or(MirGridType::Inventory);
        let id_from = reader.read_u64::<LittleEndian>()?;
        let id_to = reader.read_u64::<LittleEndian>()?;
        Ok(Self { grid_from, grid_to, id_from, id_to })
    }

    fn write_body<W: Write>(&self, writer: &mut W) -> SharedResult<()> {
        writer.write_u8(self.grid_from as u8)?;
        writer.write_u8(self.grid_to as u8)?;
        writer.write_u64::<LittleEndian>(self.id_from)?;
        writer.write_u64::<LittleEndian>(self.id_to)?;
        Ok(())
    }
}

/// Client requests to equip an item
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct EquipItem {
    pub grid: MirGridType,
    pub unique_id: u64,
    pub to: i32,
}

impl Packet for EquipItem {
    const OPCODE: i16 = ClientPacketIds::EquipItem as i16;

    fn read_body<R: Read>(reader: &mut R) -> SharedResult<Self> {
        let grid = MirGridType::try_from(reader.read_u8()?).unwrap_or(MirGridType::Inventory);
        let unique_id = reader.read_u64::<LittleEndian>()?;
        let to = reader.read_i32::<LittleEndian>()?;
        Ok(Self { grid, unique_id, to })
    }

    fn write_body<W: Write>(&self, writer: &mut W) -> SharedResult<()> {
        writer.write_u8(self.grid as u8)?;
        writer.write_u64::<LittleEndian>(self.unique_id)?;
        writer.write_i32::<LittleEndian>(self.to)?;
        Ok(())
    }
}

/// Client requests to remove an equipped item
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct RemoveItem {
    pub grid: MirGridType,
    pub unique_id: u64,
    pub to: i32,
}

impl Packet for RemoveItem {
    const OPCODE: i16 = ClientPacketIds::RemoveItem as i16;

    fn read_body<R: Read>(reader: &mut R) -> SharedResult<Self> {
        let grid = MirGridType::try_from(reader.read_u8()?).unwrap_or(MirGridType::Equipment);
        let unique_id = reader.read_u64::<LittleEndian>()?;
        let to = reader.read_i32::<LittleEndian>()?;
        Ok(Self { grid, unique_id, to })
    }

    fn write_body<W: Write>(&self, writer: &mut W) -> SharedResult<()> {
        writer.write_u8(self.grid as u8)?;
        writer.write_u64::<LittleEndian>(self.unique_id)?;
        writer.write_i32::<LittleEndian>(self.to)?;
        Ok(())
    }
}

/// Client requests to remove an item from a specific slot
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct RemoveSlotItem {
    pub grid: MirGridType,
    pub unique_id: u64,
    pub to: i32,
    pub from_slot: i32,
}

impl Packet for RemoveSlotItem {
    const OPCODE: i16 = ClientPacketIds::RemoveSlotItem as i16;

    fn read_body<R: Read>(reader: &mut R) -> SharedResult<Self> {
        let grid = MirGridType::try_from(reader.read_u8()?).unwrap_or(MirGridType::Equipment);
        let unique_id = reader.read_u64::<LittleEndian>()?;
        let to = reader.read_i32::<LittleEndian>()?;
        let from_slot = reader.read_i32::<LittleEndian>()?;
        Ok(Self { grid, unique_id, to, from_slot })
    }

    fn write_body<W: Write>(&self, writer: &mut W) -> SharedResult<()> {
        writer.write_u8(self.grid as u8)?;
        writer.write_u64::<LittleEndian>(self.unique_id)?;
        writer.write_i32::<LittleEndian>(self.to)?;
        writer.write_i32::<LittleEndian>(self.from_slot)?;
        Ok(())
    }
}

/// Client requests to split an item stack
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SplitItem {
    pub grid: MirGridType,
    pub unique_id: u64,
    pub count: u32,
}

impl Packet for SplitItem {
    const OPCODE: i16 = ClientPacketIds::SplitItem as i16;

    fn read_body<R: Read>(reader: &mut R) -> SharedResult<Self> {
        let grid = MirGridType::try_from(reader.read_u8()?).unwrap_or(MirGridType::Inventory);
        let unique_id = reader.read_u64::<LittleEndian>()?;
        let count = reader.read_u32::<LittleEndian>()?;
        Ok(Self { grid, unique_id, count })
    }

    fn write_body<W: Write>(&self, writer: &mut W) -> SharedResult<()> {
        writer.write_u8(self.grid as u8)?;
        writer.write_u64::<LittleEndian>(self.unique_id)?;
        writer.write_u32::<LittleEndian>(self.count)?;
        Ok(())
    }
}

/// Client requests to use an item
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct UseItem {
    pub unique_id: u64,
}

impl Packet for UseItem {
    const OPCODE: i16 = ClientPacketIds::UseItem as i16;

    fn read_body<R: Read>(reader: &mut R) -> SharedResult<Self> {
        let unique_id = reader.read_u64::<LittleEndian>()?;
        Ok(Self { unique_id })
    }

    fn write_body<W: Write>(&self, writer: &mut W) -> SharedResult<()> {
        writer.write_u64::<LittleEndian>(self.unique_id)?;
        Ok(())
    }
}

/// Client requests to drop an item
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct DropItem {
    pub unique_id: u64,
    pub count: u32,
    pub hero_inventory: bool,
}

impl Packet for DropItem {
    const OPCODE: i16 = ClientPacketIds::DropItem as i16;

    fn read_body<R: Read>(reader: &mut R) -> SharedResult<Self> {
        let unique_id = reader.read_u64::<LittleEndian>()?;
        let count = reader.read_u32::<LittleEndian>()?;
        let hero_inventory = reader.read_u8()? != 0;
        Ok(Self { unique_id, count, hero_inventory })
    }

    fn write_body<W: Write>(&self, writer: &mut W) -> SharedResult<()> {
        writer.write_u64::<LittleEndian>(self.unique_id)?;
        writer.write_u32::<LittleEndian>(self.count)?;
        writer.write_u8(if self.hero_inventory { 1 } else { 0 })?;
        Ok(())
    }
}

/// Client requests to drop gold
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct DropGold {
    pub amount: u32,
}

impl Packet for DropGold {
    const OPCODE: i16 = ClientPacketIds::DropGold as i16;

    fn read_body<R: Read>(reader: &mut R) -> SharedResult<Self> {
        let amount = reader.read_u32::<LittleEndian>()?;
        Ok(Self { amount })
    }

    fn write_body<W: Write>(&self, writer: &mut W) -> SharedResult<()> {
        writer.write_u32::<LittleEndian>(self.amount)?;
        Ok(())
    }
}

/// Client requests to pick up an item
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct PickUp;

impl Packet for PickUp {
    const OPCODE: i16 = ClientPacketIds::PickUp as i16;

    fn read_body<R: Read>(_: &mut R) -> SharedResult<Self> {
        Ok(Self)
    }

    fn write_body<W: Write>(&self, _: &mut W) -> SharedResult<()> {
        Ok(())
    }
}

/// Client requests rented items list
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct GetRentedItems;

impl Packet for GetRentedItems {
    const OPCODE: i16 = ClientPacketIds::GetRentedItems as i16;

    fn read_body<R: Read>(_: &mut R) -> SharedResult<Self> {
        Ok(Self)
    }

    fn write_body<W: Write>(&self, _: &mut W) -> SharedResult<()> {
        Ok(())
    }
}

/// Client requests item rental
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct ItemRentalRequest;

impl Packet for ItemRentalRequest {
    const OPCODE: i16 = ClientPacketIds::ItemRentalRequest as i16;

    fn read_body<R: Read>(_: &mut R) -> SharedResult<Self> {
        Ok(Self)
    }

    fn write_body<W: Write>(&self, _: &mut W) -> SharedResult<()> {
        Ok(())
    }
}

/// Client sets rental fee amount
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ItemRentalFee {
    pub amount: u32,
}

impl Packet for ItemRentalFee {
    const OPCODE: i16 = ClientPacketIds::ItemRentalFee as i16;

    fn read_body<R: Read>(reader: &mut R) -> SharedResult<Self> {
        let amount = reader.read_u32::<LittleEndian>()?;
        Ok(Self { amount })
    }

    fn write_body<W: Write>(&self, writer: &mut W) -> SharedResult<()> {
        writer.write_u32::<LittleEndian>(self.amount)?;
        Ok(())
    }
}

/// Client sets rental period in days
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ItemRentalPeriod {
    pub days: u32,
}

impl Packet for ItemRentalPeriod {
    const OPCODE: i16 = ClientPacketIds::ItemRentalPeriod as i16;

    fn read_body<R: Read>(reader: &mut R) -> SharedResult<Self> {
        let days = reader.read_u32::<LittleEndian>()?;
        Ok(Self { days })
    }

    fn write_body<W: Write>(&self, writer: &mut W) -> SharedResult<()> {
        writer.write_u32::<LittleEndian>(self.days)?;
        Ok(())
    }
}

/// Client deposits item for rental
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct DepositRentalItem {
    pub from: i32,
    pub to: i32,
}

impl Packet for DepositRentalItem {
    const OPCODE: i16 = ClientPacketIds::DepositRentalItem as i16;

    fn read_body<R: Read>(reader: &mut R) -> SharedResult<Self> {
        let from = reader.read_i32::<LittleEndian>()?;
        let to = reader.read_i32::<LittleEndian>()?;
        Ok(Self { from, to })
    }

    fn write_body<W: Write>(&self, writer: &mut W) -> SharedResult<()> {
        writer.write_i32::<LittleEndian>(self.from)?;
        writer.write_i32::<LittleEndian>(self.to)?;
        Ok(())
    }
}

/// Client retrieves rental item
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct RetrieveRentalItem {
    pub from: i32,
    pub to: i32,
}

impl Packet for RetrieveRentalItem {
    const OPCODE: i16 = ClientPacketIds::RetrieveRentalItem as i16;

    fn read_body<R: Read>(reader: &mut R) -> SharedResult<Self> {
        let from = reader.read_i32::<LittleEndian>()?;
        let to = reader.read_i32::<LittleEndian>()?;
        Ok(Self { from, to })
    }

    fn write_body<W: Write>(&self, writer: &mut W) -> SharedResult<()> {
        writer.write_i32::<LittleEndian>(self.from)?;
        writer.write_i32::<LittleEndian>(self.to)?;
        Ok(())
    }
}

/// Client cancels item rental
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct CancelItemRental;

impl Packet for CancelItemRental {
    const OPCODE: i16 = ClientPacketIds::CancelItemRental as i16;

    fn read_body<R: Read>(_: &mut R) -> SharedResult<Self> {
        Ok(Self)
    }

    fn write_body<W: Write>(&self, _: &mut W) -> SharedResult<()> {
        Ok(())
    }
}

/// Client locks rental fee
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct ItemRentalLockFee;

impl Packet for ItemRentalLockFee {
    const OPCODE: i16 = ClientPacketIds::ItemRentalLockFee as i16;

    fn read_body<R: Read>(_: &mut R) -> SharedResult<Self> {
        Ok(Self)
    }

    fn write_body<W: Write>(&self, _: &mut W) -> SharedResult<()> {
        Ok(())
    }
}

/// Client locks rental item
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct ItemRentalLockItem;

impl Packet for ItemRentalLockItem {
    const OPCODE: i16 = ClientPacketIds::ItemRentalLockItem as i16;

    fn read_body<R: Read>(_: &mut R) -> SharedResult<Self> {
        Ok(Self)
    }

    fn write_body<W: Write>(&self, _: &mut W) -> SharedResult<()> {
        Ok(())
    }
}

/// Client confirms item rental
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct ConfirmItemRental;

impl Packet for ConfirmItemRental {
    const OPCODE: i16 = ClientPacketIds::ConfirmItemRental as i16;

    fn read_body<R: Read>(_: &mut R) -> SharedResult<Self> {
        Ok(Self)
    }

    fn write_body<W: Write>(&self, _: &mut W) -> SharedResult<()> {
        Ok(())
    }
}
