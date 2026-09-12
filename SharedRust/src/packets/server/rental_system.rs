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
    pub items: Vec<RentalItemInfo>, // 租赁物品列表
}

/// 单条已租出物品信息，字段与 C# `ItemRentalInformation`（Shared/Data/ItemData.cs:1090）
/// 一一对应：ItemId / ItemName / RentingPlayerName / ItemReturnDate。
/// 客户端没有本地物品库，故物品名由服务端随包下发（C# 亦是字符串）。
#[derive(Debug, Clone)]
pub struct RentalItemInfo {
    /// C# `ItemId`
    pub item_id: u64,
    /// C# `ItemName`
    pub item_name: String,
    /// C# `RentingPlayerName`
    pub renting_player_name: String,
    /// C# `ItemReturnDate.ToBinary()`（Rust 端口用 Unix 秒墙钟）
    pub return_date: i64,
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
///
/// C# `S.ItemRentalRequest`（Shared/ServerPackets.cs:6382）：`Name` + `Renting`。
/// 两端各收一份，用 `Renting` 区分本端角色（C# `GameScene.ItemRentalRequest`）：
/// - 点击 RENT 的一端收 `renting = false`，`name` = 对方（租客）名
///   → 本端是物主：自有「物品窗」+ 对方「费用窗」（`GuestItemRentDialog`）
/// - 被请求的一端收 `renting = true`，`name` = 对方（物主）名
///   → 本端是租客：自有「费用窗」+ 对方「物品窗」（`GuestItemRentingDialog`）
#[derive(Debug, Clone)]
pub struct ItemRentalRequest {
    /// C# `Name`：对方名字（显示在对方镜窗的名称标签）
    pub name: String,
    /// C# `Renting`：true = 本端是租客，false = 本端是物主
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
///
/// C# `S.UpdateRentalItem`（Shared/ServerPackets.cs:6492）：`HasData` + `LoanItem`，
/// 由物主存入/取回物品后发给租客（填对方物品窗的物品格）。
/// Rust 端口额外带 `rental_fee`/`rental_period`（C# 无；供客户端少一次往返刷新）。
#[derive(Debug, Clone)]
pub struct UpdateRentalItem {
    /// C# `LoanItem`；`None` = C# `HasData == false`（取回后清空对方物品格）
    pub item: Option<UserItem>,
    pub rental_fee: u32,    // 租金
    pub rental_period: i32, // 租赁期限
}

impl Packet for UpdateRentalItem {
    const OPCODE: i16 = ServerPacketIds::UpdateRentalItem as i16;

    fn write_body<W: std::io::Write>(&self, writer: &mut W) -> SharedResult<()> {
        use byteorder::WriteBytesExt;

        // C# `HasData` 语义：无物品时只写 false，不写物品体
        match &self.item {
            Some(item) => {
                writer.write_u8(1)?;
                item.write_to(writer)?;
            }
            None => writer.write_u8(0)?,
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
///
/// C# `S.ItemRentalLock`（Shared/ServerPackets.cs:6533）：`Success`/`GoldLocked`/`ItemLocked`，
/// 本端锁定回执：`gold_locked` → 自有费用窗锁形，`item_locked` → 自有物品窗锁形。
#[derive(Debug, Clone)]
pub struct ItemRentalLock {
    pub success: bool,     // 是否锁定成功
    pub gold_locked: bool, // 费用已锁定（C# `GoldLocked`）
    pub item_locked: bool, // 物品已锁定（C# `ItemLocked`）
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
///
/// C# `S.ItemRentalPartnerLock`（Shared/ServerPackets.cs:6558）：`GoldLocked` + `ItemLocked`，
/// 对方锁定通知：`gold_locked` → 对方「费用窗」锁形（`GuestItemRentDialog.Lock()`），
/// `item_locked` → 对方「物品窗」锁形（`GuestItemRentingDialog.Lock()`）。
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::packets::base::Packet;

    /// #2214/#2720：GetRentedItems 物主视角线格式 roundtrip
    /// （C# `ItemRentalInformation` 形状：ItemId/ItemName/RentingPlayerName/ItemReturnDate）
    #[test]
    fn get_rented_items_roundtrip() -> SharedResult<()> {
        let pkt = GetRentedItems {
            items: vec![RentalItemInfo {
                item_id: 88_888,
                item_name: "屠龙".to_string(),
                renting_player_name: "Renter".to_string(),
                return_date: 1_700_000_000,
            }],
        };
        let mut buf = Vec::new();
        pkt.write_body(&mut buf)?;
        let mut cur = std::io::Cursor::new(&buf);
        let back = GetRentedItems::read_body(&mut cur)?;
        assert_eq!(back.items.len(), 1);
        assert_eq!(back.items[0].item_id, 88_888);
        assert_eq!(back.items[0].item_name, "屠龙");
        assert_eq!(back.items[0].renting_player_name, "Renter");
        assert_eq!(back.items[0].return_date, 1_700_000_000);
        Ok(())
    }

    /// #2720：ItemRentalRequest 角色标记线格式 roundtrip
    /// （C# `S.ItemRentalRequest`：Name + Renting，两端各收一份区分物主/租客）
    #[test]
    fn item_rental_request_role_roundtrip() -> SharedResult<()> {
        for (name, renting) in [("物主甲", false), ("租客乙", true)] {
            let pkt = ItemRentalRequest {
                name: name.to_string(),
                renting,
            };
            let mut buf = Vec::new();
            pkt.write_body(&mut buf)?;
            let mut cur = std::io::Cursor::new(&buf);
            let back = ItemRentalRequest::read_body(&mut cur)?;
            assert_eq!(back.name, name);
            assert_eq!(back.renting, renting);
        }
        Ok(())
    }

    /// #2720：UpdateRentalItem 的 `HasData` 两种形态 roundtrip
    /// （C# 无物品时只写 false，不写物品体；有物品时写整只 UserItem）
    #[test]
    fn update_rental_item_hasdata_roundtrip() -> SharedResult<()> {
        let item = UserItem {
            unique_id: 4242,
            item_index: 853,
            count: 1,
            ..Default::default()
        };
        let with_item = UpdateRentalItem {
            item: Some(item.clone()),
            rental_fee: 100,
            rental_period: 24,
        };
        let mut buf = Vec::new();
        with_item.write_body(&mut buf)?;
        let mut cur = std::io::Cursor::new(&buf);
        let back = UpdateRentalItem::read_body(&mut cur)?;
        assert_eq!(back.item.as_ref().map(|i| i.unique_id), Some(4242));
        assert_eq!(back.item.as_ref().map(|i| i.item_index), Some(853));
        assert_eq!(back.rental_fee, 100);
        assert_eq!(back.rental_period, 24);

        let cleared = UpdateRentalItem {
            item: None,
            rental_fee: 0,
            rental_period: 0,
        };
        let mut buf = Vec::new();
        cleared.write_body(&mut buf)?;
        assert_eq!(buf[0], 0, "HasData=false 时首字节应为 0");
        assert_eq!(buf.len(), 9, "HasData=false 时只有 [u8][u32][i32]");
        let mut cur = std::io::Cursor::new(&buf);
        let back = UpdateRentalItem::read_body(&mut cur)?;
        assert!(back.item.is_none());
        Ok(())
    }

    /// #2720：锁定回执/对方锁定通知的线格式（C# `GoldLocked`/`ItemLocked` 判别位）
    #[test]
    fn rental_lock_discriminators_roundtrip() -> SharedResult<()> {
        let own = ItemRentalLock {
            success: true,
            gold_locked: false,
            item_locked: true,
        };
        let mut buf = Vec::new();
        own.write_body(&mut buf)?;
        assert_eq!(buf, vec![1, 0, 1], "本端锁定：[success][gold][item]");
        let back = ItemRentalLock::read_body(&mut std::io::Cursor::new(&buf))?;
        assert!(back.success && !back.gold_locked && back.item_locked);

        let partner = ItemRentalPartnerLock {
            gold_locked: true,
            item_locked: false,
        };
        let mut buf = Vec::new();
        partner.write_body(&mut buf)?;
        assert_eq!(buf, vec![1, 0], "对方锁定：[gold][item]");
        let back = ItemRentalPartnerLock::read_body(&mut std::io::Cursor::new(&buf))?;
        assert!(back.gold_locked && !back.item_locked);
        Ok(())
    }
}
