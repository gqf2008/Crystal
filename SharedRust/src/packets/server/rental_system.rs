// 租赁系统相关数据包
use super::super::base::Packet;
use crate::data::item::UserItem;
use crate::data::stats::SharedResult;
use crate::enums::ServerPacketIds;
use byteorder::{LittleEndian, ReadBytesExt};
use std::io::Read;

/// GetRentedItems - 获取租赁物品 (252)
#[derive(Debug, Clone)]
pub struct GetRentedItems {
    pub items: Vec<RentalItemInfo>, // 租赁物品列表
}

#[derive(Debug, Clone)]
pub struct RentalItemInfo {
    pub item: UserItem,     // 物品
    pub rental_fee: u32,    // 租金
    pub rental_period: i32, // 租赁期限(小时)
    pub expiry_date: i64,   // 到期日期
}

impl Packet for GetRentedItems {
    const OPCODE: i16 = ServerPacketIds::GetRentedItems as i16;

    fn write_body<W: std::io::Write>(&self, writer: &mut W) -> SharedResult<()> {
        use byteorder::WriteBytesExt;

        writer.write_i32::<LittleEndian>(self.items.len() as i32)?;

        for info in &self.items {
            info.item.write_to(writer)?;
            writer.write_u32::<LittleEndian>(info.rental_fee)?;
            writer.write_i32::<LittleEndian>(info.rental_period)?;
            writer.write_i64::<LittleEndian>(info.expiry_date)?;
        }

        Ok(())
    }

    fn read_body<R: Read>(reader: &mut R) -> SharedResult<Self> {
        let count = reader.read_i32::<LittleEndian>()?;
        let mut items = Vec::with_capacity(count as usize);

        for _ in 0..count {
            let item = UserItem::read_from(reader, i32::MAX, i32::MAX)?;
            let rental_fee = reader.read_u32::<LittleEndian>()?;
            let rental_period = reader.read_i32::<LittleEndian>()?;
            let expiry_date = reader.read_i64::<LittleEndian>()?;

            items.push(RentalItemInfo {
                item,
                rental_fee,
                rental_period,
                expiry_date,
            });
        }

        Ok(Self { items })
    }
}

/// ItemRentalRequest - 物品租赁请求 (253)
#[derive(Debug, Clone)]
pub struct ItemRentalRequest {
    // C# 原实现为空
}

impl Packet for ItemRentalRequest {
    const OPCODE: i16 = ServerPacketIds::ItemRentalRequest as i16;

    fn write_body<W: std::io::Write>(&self, _writer: &mut W) -> SharedResult<()> {
        // C# original implementation is empty
        Ok(())
    }

    fn read_body<R: Read>(_reader: &mut R) -> SharedResult<Self> {
        Ok(Self {})
    }
}

/// ItemRentalFee - 物品租赁费用 (254)
#[derive(Debug, Clone)]
pub struct ItemRentalFee {
    pub fee: u32, // 租赁费用
}

impl Packet for ItemRentalFee {
    const OPCODE: i16 = ServerPacketIds::ItemRentalFee as i16;

    fn write_body<W: std::io::Write>(&self, writer: &mut W) -> SharedResult<()> {
        use byteorder::WriteBytesExt;

        writer.write_u32::<LittleEndian>(self.fee)?;

        Ok(())
    }

    fn read_body<R: Read>(reader: &mut R) -> SharedResult<Self> {
        let fee = reader.read_u32::<LittleEndian>()?;
        Ok(Self { fee })
    }
}

/// ItemRentalPeriod - 物品租赁期限 (255)
#[derive(Debug, Clone)]
pub struct ItemRentalPeriod {
    pub period: i32, // 租赁期限(小时)
}

impl Packet for ItemRentalPeriod {
    const OPCODE: i16 = ServerPacketIds::ItemRentalPeriod as i16;

    fn write_body<W: std::io::Write>(&self, writer: &mut W) -> SharedResult<()> {
        use byteorder::WriteBytesExt;

        // Note: C# uses Days(u32), but Rust uses period(i32)
        writer.write_i32::<LittleEndian>(self.period)?;

        Ok(())
    }

    fn read_body<R: Read>(reader: &mut R) -> SharedResult<Self> {
        let period = reader.read_i32::<LittleEndian>()?;
        Ok(Self { period })
    }
}

/// DepositRentalItem - 存入租赁物品 (256)
#[derive(Debug, Clone)]
pub struct DepositRentalItem {
    pub unique_id: u64, // 物品唯一ID
    pub success: bool,  // 是否成功
}

impl Packet for DepositRentalItem {
    const OPCODE: i16 = ServerPacketIds::DepositRentalItem as i16;

    fn write_body<W: std::io::Write>(&self, writer: &mut W) -> SharedResult<()> {
        use byteorder::WriteBytesExt;

        // Note: C# uses From/To(i32), but Rust uses unique_id(u64)
        writer.write_u64::<LittleEndian>(self.unique_id)?;
        writer.write_u8(if self.success { 1 } else { 0 })?;

        Ok(())
    }

    fn read_body<R: Read>(reader: &mut R) -> SharedResult<Self> {
        let unique_id = reader.read_u64::<LittleEndian>()?;
        let success = reader.read_u8()? != 0;
        Ok(Self { unique_id, success })
    }
}

/// RetrieveRentalItem - 取回租赁物品 (257)
#[derive(Debug, Clone)]
pub struct RetrieveRentalItem {
    pub unique_id: u64, // 物品唯一ID
    pub success: bool,  // 是否成功
}

impl Packet for RetrieveRentalItem {
    const OPCODE: i16 = ServerPacketIds::RetrieveRentalItem as i16;

    fn write_body<W: std::io::Write>(&self, writer: &mut W) -> SharedResult<()> {
        use byteorder::WriteBytesExt;

        // Note: C# uses From/To(i32), but Rust uses unique_id(u64)
        writer.write_u64::<LittleEndian>(self.unique_id)?;
        writer.write_u8(if self.success { 1 } else { 0 })?;

        Ok(())
    }

    fn read_body<R: Read>(reader: &mut R) -> SharedResult<Self> {
        let unique_id = reader.read_u64::<LittleEndian>()?;
        let success = reader.read_u8()? != 0;
        Ok(Self { unique_id, success })
    }
}

/// UpdateRentalItem - 更新租赁物品 (258)
#[derive(Debug, Clone)]
pub struct UpdateRentalItem {
    pub item: UserItem,     // 更新的物品
    pub rental_fee: u32,    // 租金
    pub rental_period: i32, // 租赁期限
}

impl Packet for UpdateRentalItem {
    const OPCODE: i16 = ServerPacketIds::UpdateRentalItem as i16;

    fn write_body<W: std::io::Write>(&self, writer: &mut W) -> SharedResult<()> {
        use byteorder::WriteBytesExt;

        // Note: Rust always has item, C# can be null
        // Writing as if always present
        writer.write_u8(1)?; // HasData = true
        self.item.write_to(writer)?;
        writer.write_u32::<LittleEndian>(self.rental_fee)?;
        writer.write_i32::<LittleEndian>(self.rental_period)?;

        Ok(())
    }

    fn read_body<R: Read>(reader: &mut R) -> SharedResult<Self> {
        let item = UserItem::read_from(reader, i32::MAX, i32::MAX)?;
        let rental_fee = reader.read_u32::<LittleEndian>()?;
        let rental_period = reader.read_i32::<LittleEndian>()?;
        Ok(Self {
            item,
            rental_fee,
            rental_period,
        })
    }
}

/// CancelItemRental - 取消物品租赁 (259)
#[derive(Debug, Clone)]
pub struct CancelItemRental {
    pub unique_id: u64, // 物品唯一ID
    pub success: bool,  // 是否成功
}

impl Packet for CancelItemRental {
    const OPCODE: i16 = ServerPacketIds::CancelItemRental as i16;

    fn write_body<W: std::io::Write>(&self, writer: &mut W) -> SharedResult<()> {
        use byteorder::WriteBytesExt;

        // Note: C# is empty, but Rust has unique_id + success
        writer.write_u64::<LittleEndian>(self.unique_id)?;
        writer.write_u8(if self.success { 1 } else { 0 })?;

        Ok(())
    }

    fn read_body<R: Read>(reader: &mut R) -> SharedResult<Self> {
        let unique_id = reader.read_u64::<LittleEndian>()?;
        let success = reader.read_u8()? != 0;
        Ok(Self { unique_id, success })
    }
}

/// ItemRentalLock - 物品租赁锁定 (260)
#[derive(Debug, Clone)]
pub struct ItemRentalLock {
    pub unique_id: u64, // 物品唯一ID
    pub locked: bool,   // 是否锁定
}

impl Packet for ItemRentalLock {
    const OPCODE: i16 = ServerPacketIds::ItemRentalLock as i16;

    fn write_body<W: std::io::Write>(&self, writer: &mut W) -> SharedResult<()> {
        use byteorder::WriteBytesExt;

        // Note: C# has Success/GoldLocked/ItemLocked, Rust has unique_id/locked
        writer.write_u64::<LittleEndian>(self.unique_id)?;
        writer.write_u8(if self.locked { 1 } else { 0 })?;

        Ok(())
    }

    fn read_body<R: Read>(reader: &mut R) -> SharedResult<Self> {
        let unique_id = reader.read_u64::<LittleEndian>()?;
        let locked = reader.read_u8()? != 0;
        Ok(Self { unique_id, locked })
    }
}

/// ItemRentalPartnerLock - 物品租赁伙伴锁定 (261)
#[derive(Debug, Clone)]
pub struct ItemRentalPartnerLock {
    pub unique_id: u64, // 物品唯一ID
    pub locked: bool,   // 是否锁定
}

impl Packet for ItemRentalPartnerLock {
    const OPCODE: i16 = ServerPacketIds::ItemRentalPartnerLock as i16;

    fn write_body<W: std::io::Write>(&self, writer: &mut W) -> SharedResult<()> {
        use byteorder::WriteBytesExt;

        // Note: C# has GoldLocked/ItemLocked, Rust has unique_id/locked
        writer.write_u64::<LittleEndian>(self.unique_id)?;
        writer.write_u8(if self.locked { 1 } else { 0 })?;

        Ok(())
    }

    fn read_body<R: Read>(reader: &mut R) -> SharedResult<Self> {
        let unique_id = reader.read_u64::<LittleEndian>()?;
        let locked = reader.read_u8()? != 0;
        Ok(Self { unique_id, locked })
    }
}

/// CanConfirmItemRental - 可确认物品租赁 (262)
#[derive(Debug, Clone)]
pub struct CanConfirmItemRental {
    pub can_confirm: bool, // 是否可以确认
}

impl Packet for CanConfirmItemRental {
    const OPCODE: i16 = ServerPacketIds::CanConfirmItemRental as i16;

    fn write_body<W: std::io::Write>(&self, writer: &mut W) -> SharedResult<()> {
        use byteorder::WriteBytesExt;

        // Note: C# is empty, but Rust has can_confirm
        writer.write_u8(if self.can_confirm { 1 } else { 0 })?;

        Ok(())
    }

    fn read_body<R: Read>(reader: &mut R) -> SharedResult<Self> {
        let can_confirm = reader.read_u8()? != 0;
        Ok(Self { can_confirm })
    }
}

/// ConfirmItemRental - 确认物品租赁 (263)
#[derive(Debug, Clone)]
pub struct ConfirmItemRental {
    pub success: bool, // 是否成功
}

impl Packet for ConfirmItemRental {
    const OPCODE: i16 = ServerPacketIds::ConfirmItemRental as i16;

    fn write_body<W: std::io::Write>(&self, writer: &mut W) -> SharedResult<()> {
        use byteorder::WriteBytesExt;

        // Note: C# is empty, but Rust has success
        writer.write_u8(if self.success { 1 } else { 0 })?;

        Ok(())
    }

    fn read_body<R: Read>(reader: &mut R) -> SharedResult<Self> {
        let success = reader.read_u8()? != 0;
        Ok(Self { success })
    }
}
