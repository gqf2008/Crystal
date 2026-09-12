// 租赁系统相关数据包
use super::super::base::Packet;
use crate::binary::{read_dotnet_string, write_dotnet_string};
use crate::data::item::UserItem;
use crate::data::stats::SharedResult;
use crate::enums::ServerPacketIds;
use byteorder::{LittleEndian, ReadBytesExt};
use std::io::Read;

/// GetRentedItems - 获取租赁物品 (252)
#[derive(Debug, Clone)]
pub struct GetRentedItems {
    pub items: Vec<RentalItemInfo>,  // 租赁物品列表
}

#[derive(Debug, Clone)]
pub struct RentalItemInfo {
    // C# `ItemRentalInformation`（Shared/Data/ItemData.cs:1090）同形
    pub item_id: u64,               // ItemId
    pub item_name: String,          // ItemName（客户端无本地物品库，服务端随包下发）
    pub renting_player_name: String,// RentingPlayerName
    pub return_date: i64,           // ItemReturnDate（Unix 秒）
}

impl Packet for GetRentedItems {
    const OPCODE: i16 = ServerPacketIds::GetRentedItems as i16;

    fn write_body<W: std::io::Write>(&self, writer: &mut W) -> SharedResult<()> {
        use byteorder::WriteBytesExt;
        
        writer.write_i32::<LittleEndian>(self.items.len() as i32)?;
        
        for info in &self.items {
            writer.write_u64::<LittleEndian>(info.item_id)?;
            write_dotnet_string(writer, &info.item_name)?;
            write_dotnet_string(writer, &info.renting_player_name)?;
            writer.write_i64::<LittleEndian>(info.return_date)?;
        }
        
        Ok(())
    }

    fn read_body<R: Read>(reader: &mut R) -> SharedResult<Self> {
        let count = reader.read_i32::<LittleEndian>()?;
        let mut items = Vec::with_capacity(count as usize);
        
        for _ in 0..count {
            let item_id = reader.read_u64::<LittleEndian>()?;
            let item_name = read_dotnet_string(reader)?;
            let renting_player_name = read_dotnet_string(reader)?;
            let return_date = reader.read_i64::<LittleEndian>()?;

            items.push(RentalItemInfo {
                item_id,
                item_name,
                renting_player_name,
                return_date,
            });
        }
        
        Ok(Self { items })
    }
}

/// ItemRentalRequest - 物品租赁请求 (253)
/// C# `S.ItemRentalRequest`（Shared/ServerPackets.cs:6382）同形：`Name` + `Renting`
/// （两端各收一份：`renting=false` = 本端物主，`renting=true` = 本端租客）
#[derive(Debug, Clone)]
pub struct ItemRentalRequest {
    // C# `Name`：对方名字
    pub name: String,
    // C# `Renting`：true = 本端是租客
    pub renting: bool,
}

impl Packet for ItemRentalRequest {
    const OPCODE: i16 = ServerPacketIds::ItemRentalRequest as i16;

    fn write_body<W: std::io::Write>(&self, writer: &mut W) -> SharedResult<()> {
        use byteorder::WriteBytesExt;

        write_dotnet_string(writer, &self.name)?;
        writer.write_u8(if self.renting { 1 } else { 0 })?;

        Ok(())
    }

    fn read_body<R: Read>(reader: &mut R) -> SharedResult<Self> {
        let name = read_dotnet_string(reader)?;
        let renting = reader.read_u8()? != 0;
        Ok(Self { name, renting })
    }
}

/// ItemRentalFee - 物品租赁费用 (254)
#[derive(Debug, Clone)]
pub struct ItemRentalFee {
    pub fee: u32,                   // 租赁费用
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
    pub period: i32,                // 租赁期限(小时)
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
    pub unique_id: u64,             // 物品唯一ID
    pub success: bool,              // 是否成功
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
    pub unique_id: u64,             // 物品唯一ID
    pub success: bool,              // 是否成功
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
/// C# `S.UpdateRentalItem`（Shared/ServerPackets.cs:6492）同形：`HasData` + `LoanItem`
/// （Rust 额外带 rental_fee/rental_period）
#[derive(Debug, Clone)]
pub struct UpdateRentalItem {
    // C# LoanItem；None = HasData=false（清空物品格）
    pub item: Option<UserItem>,
    pub rental_fee: u32,    // 租金
    pub rental_period: i32, // 租赁期限
}

impl Packet for UpdateRentalItem {
    const OPCODE: i16 = ServerPacketIds::UpdateRentalItem as i16;

    fn write_body<W: std::io::Write>(&self, writer: &mut W) -> SharedResult<()> {
        use byteorder::WriteBytesExt;

        match &self.item {
            Some(item) => {
                writer.write_u8(1)?; // HasData = true
                item.write_to(writer)?;
            }
            None => writer.write_u8(0)?, // HasData = false
        }
        writer.write_u32::<LittleEndian>(self.rental_fee)?;
        writer.write_i32::<LittleEndian>(self.rental_period)?;

        Ok(())
    }

    fn read_body<R: Read>(reader: &mut R) -> SharedResult<Self> {
        let has_data = reader.read_u8()? != 0;
        let item = if has_data {
            Some(UserItem::read_from(reader, i32::MAX, i32::MAX)?)
        } else {
            None
        };
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
    pub unique_id: u64,             // 物品唯一ID
    pub success: bool,              // 是否成功
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
/// C# `S.ItemRentalLock`（Shared/ServerPackets.cs:6533）同形：Success/GoldLocked/ItemLocked
#[derive(Debug, Clone)]
pub struct ItemRentalLock {
    pub success: bool,     // 是否锁定成功
    pub gold_locked: bool, // C# GoldLocked：费用已锁定
    pub item_locked: bool, // C# ItemLocked：物品已锁定
}

impl Packet for ItemRentalLock {
    const OPCODE: i16 = ServerPacketIds::ItemRentalLock as i16;

    fn write_body<W: std::io::Write>(&self, writer: &mut W) -> SharedResult<()> {
        use byteorder::WriteBytesExt;

        writer.write_u8(if self.success { 1 } else { 0 })?;
        writer.write_u8(if self.gold_locked { 1 } else { 0 })?;
        writer.write_u8(if self.item_locked { 1 } else { 0 })?;

        Ok(())
    }

    fn read_body<R: Read>(reader: &mut R) -> SharedResult<Self> {
        let success = reader.read_u8()? != 0;
        let gold_locked = reader.read_u8()? != 0;
        let item_locked = reader.read_u8()? != 0;
        Ok(Self {
            success,
            gold_locked,
            item_locked,
        })
    }
}

/// ItemRentalPartnerLock - 物品租赁伙伴锁定 (261)
/// C# `S.ItemRentalPartnerLock`（Shared/ServerPackets.cs:6558）同形：GoldLocked/ItemLocked
#[derive(Debug, Clone)]
pub struct ItemRentalPartnerLock {
    pub gold_locked: bool, // 对方已锁定费用
    pub item_locked: bool, // 对方已锁定物品
}

impl Packet for ItemRentalPartnerLock {
    const OPCODE: i16 = ServerPacketIds::ItemRentalPartnerLock as i16;

    fn write_body<W: std::io::Write>(&self, writer: &mut W) -> SharedResult<()> {
        use byteorder::WriteBytesExt;

        writer.write_u8(if self.gold_locked { 1 } else { 0 })?;
        writer.write_u8(if self.item_locked { 1 } else { 0 })?;

        Ok(())
    }

    fn read_body<R: Read>(reader: &mut R) -> SharedResult<Self> {
        let gold_locked = reader.read_u8()? != 0;
        let item_locked = reader.read_u8()? != 0;
        Ok(Self {
            gold_locked,
            item_locked,
        })
    }
}

/// CanConfirmItemRental - 可确认物品租赁 (262)
#[derive(Debug, Clone)]
pub struct CanConfirmItemRental {
    pub can_confirm: bool,          // 是否可以确认
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
    pub success: bool,              // 是否成功
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
