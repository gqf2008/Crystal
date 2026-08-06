// 物品操作相关的服务器数据包

use super::super::base::Packet;
use crate::data::item::UserItem;
use crate::data::stats::SharedResult;
use crate::enums::{MirGridType, ServerPacketIds};
use byteorder::{LittleEndian, ReadBytesExt, WriteBytesExt};
use std::io::{Read, Write};

/// 移动物品
#[derive(Debug, Clone)]
pub struct MoveItem {
    pub grid: MirGridType, // 物品网格类型（背包、装备、仓库等）
    pub from: i32,         // 源位置
    pub to: i32,           // 目标位置
    pub success: bool,     // 是否成功
}

impl Packet for MoveItem {
    const OPCODE: i16 = ServerPacketIds::MoveItem as i16;

    fn read_body<R: Read>(reader: &mut R) -> SharedResult<Self> {
        let grid = MirGridType::try_from(reader.read_u8()?)?;
        let from = reader.read_i32::<LittleEndian>()?;
        let to = reader.read_i32::<LittleEndian>()?;
        let success = reader.read_u8()? != 0;
        Ok(Self {
            grid,
            from,
            to,
            success,
        })
    }

    fn write_body<W: Write>(&self, writer: &mut W) -> SharedResult<()> {
        writer.write_u8(self.grid as u8)?;
        writer.write_i32::<LittleEndian>(self.from)?;
        writer.write_i32::<LittleEndian>(self.to)?;
        writer.write_u8(if self.success { 1 } else { 0 })?;
        Ok(())
    }
}

/// 装备物品
#[derive(Debug, Clone)]
pub struct EquipItem {
    pub grid: MirGridType, // 物品网格类型
    pub unique_id: u64,    // 物品唯一 ID
    pub to: i32,           // 目标位置
    pub success: bool,     // 是否成功
}

impl Packet for EquipItem {
    const OPCODE: i16 = ServerPacketIds::EquipItem as i16;

    fn read_body<R: Read>(reader: &mut R) -> SharedResult<Self> {
        let grid = MirGridType::try_from(reader.read_u8()?)?;
        let unique_id = reader.read_u64::<LittleEndian>()?;
        let to = reader.read_i32::<LittleEndian>()?;
        let success = reader.read_u8()? != 0;
        Ok(Self {
            grid,
            unique_id,
            to,
            success,
        })
    }

    fn write_body<W: Write>(&self, writer: &mut W) -> SharedResult<()> {
        writer.write_u8(self.grid as u8)?;
        writer.write_u64::<LittleEndian>(self.unique_id)?;
        writer.write_i32::<LittleEndian>(self.to)?;
        writer.write_u8(if self.success { 1 } else { 0 })?;
        Ok(())
    }
}

/// 合并物品
#[derive(Debug, Clone)]
pub struct MergeItem {
    pub grid_from: MirGridType, // 源网格类型
    pub grid_to: MirGridType,   // 目标网格类型
    pub id_from: u64,           // 源物品 ID
    pub id_to: u64,             // 目标物品 ID
    pub success: bool,          // 是否成功
}

impl Packet for MergeItem {
    const OPCODE: i16 = ServerPacketIds::MergeItem as i16;

    fn read_body<R: Read>(reader: &mut R) -> SharedResult<Self> {
        let grid_from = MirGridType::try_from(reader.read_u8()?)?;
        let grid_to = MirGridType::try_from(reader.read_u8()?)?;
        let id_from = reader.read_u64::<LittleEndian>()?;
        let id_to = reader.read_u64::<LittleEndian>()?;
        let success = reader.read_u8()? != 0;
        Ok(Self {
            grid_from,
            grid_to,
            id_from,
            id_to,
            success,
        })
    }

    fn write_body<W: Write>(&self, writer: &mut W) -> SharedResult<()> {
        writer.write_u8(self.grid_from as u8)?;
        writer.write_u8(self.grid_to as u8)?;
        writer.write_u64::<LittleEndian>(self.id_from)?;
        writer.write_u64::<LittleEndian>(self.id_to)?;
        writer.write_u8(if self.success { 1 } else { 0 })?;
        Ok(())
    }
}

/// 卸载物品
#[derive(Debug, Clone)]
pub struct RemoveItem {
    pub grid: MirGridType, // 网格类型
    pub unique_id: u64,    // 物品唯一 ID
    pub to: i32,           // 目标位置
    pub success: bool,     // 是否成功
}

impl Packet for RemoveItem {
    const OPCODE: i16 = ServerPacketIds::RemoveItem as i16;

    fn read_body<R: Read>(reader: &mut R) -> SharedResult<Self> {
        let grid = MirGridType::try_from(reader.read_u8()?)?;
        let unique_id = reader.read_u64::<LittleEndian>()?;
        let to = reader.read_i32::<LittleEndian>()?;
        let success = reader.read_u8()? != 0;
        Ok(Self {
            grid,
            unique_id,
            to,
            success,
        })
    }

    fn write_body<W: Write>(&self, writer: &mut W) -> SharedResult<()> {
        writer.write_u8(self.grid as u8)?;
        writer.write_u64::<LittleEndian>(self.unique_id)?;
        writer.write_i32::<LittleEndian>(self.to)?;
        writer.write_u8(if self.success { 1 } else { 0 })?;
        Ok(())
    }
}

/// 从槽位移除物品
#[derive(Debug, Clone)]
pub struct RemoveSlotItem {
    pub grid: MirGridType,    // 源网格类型
    pub grid_to: MirGridType, // 目标网格类型
    pub unique_id: u64,       // 物品唯一 ID
    pub to: i32,              // 目标位置
    pub success: bool,        // 是否成功
}

impl Packet for RemoveSlotItem {
    const OPCODE: i16 = ServerPacketIds::RemoveSlotItem as i16;

    fn read_body<R: Read>(reader: &mut R) -> SharedResult<Self> {
        let grid = MirGridType::try_from(reader.read_u8()?)?;
        let grid_to = MirGridType::try_from(reader.read_u8()?)?;
        let unique_id = reader.read_u64::<LittleEndian>()?;
        let to = reader.read_i32::<LittleEndian>()?;
        let success = reader.read_u8()? != 0;
        Ok(Self {
            grid,
            grid_to,
            unique_id,
            to,
            success,
        })
    }

    fn write_body<W: Write>(&self, writer: &mut W) -> SharedResult<()> {
        writer.write_u8(self.grid as u8)?;
        writer.write_u8(self.grid_to as u8)?;
        writer.write_u64::<LittleEndian>(self.unique_id)?;
        writer.write_i32::<LittleEndian>(self.to)?;
        writer.write_u8(if self.success { 1 } else { 0 })?;
        Ok(())
    }
}

/// 从仓库取回物品
#[derive(Debug, Clone)]
pub struct TakeBackItem {
    pub from: i32,     // 源位置
    pub to: i32,       // 目标位置
    pub success: bool, // 是否成功
}

impl Packet for TakeBackItem {
    const OPCODE: i16 = ServerPacketIds::TakeBackItem as i16;

    fn read_body<R: Read>(reader: &mut R) -> SharedResult<Self> {
        let from = reader.read_i32::<LittleEndian>()?;
        let to = reader.read_i32::<LittleEndian>()?;
        let success = reader.read_u8()? != 0;
        Ok(Self { from, to, success })
    }

    fn write_body<W: Write>(&self, writer: &mut W) -> SharedResult<()> {
        writer.write_i32::<LittleEndian>(self.from)?;
        writer.write_i32::<LittleEndian>(self.to)?;
        writer.write_u8(if self.success { 1 } else { 0 })?;
        Ok(())
    }
}

/// 存储物品到仓库
#[derive(Debug, Clone)]
pub struct StoreItem {
    pub from: i32,     // 源位置
    pub to: i32,       // 目标位置
    pub success: bool, // 是否成功
}

impl Packet for StoreItem {
    const OPCODE: i16 = ServerPacketIds::StoreItem as i16;

    fn read_body<R: Read>(reader: &mut R) -> SharedResult<Self> {
        let from = reader.read_i32::<LittleEndian>()?;
        let to = reader.read_i32::<LittleEndian>()?;
        let success = reader.read_u8()? != 0;
        Ok(Self { from, to, success })
    }

    fn write_body<W: Write>(&self, writer: &mut W) -> SharedResult<()> {
        writer.write_i32::<LittleEndian>(self.from)?;
        writer.write_i32::<LittleEndian>(self.to)?;
        writer.write_u8(if self.success { 1 } else { 0 })?;
        Ok(())
    }
}

/// 存入精炼物品
#[derive(Debug, Clone)]
pub struct DepositRefineItem {
    pub from: i32,     // 源位置
    pub to: i32,       // 目标位置
    pub success: bool, // 是否成功
}

impl Packet for DepositRefineItem {
    const OPCODE: i16 = ServerPacketIds::DepositRefineItem as i16;

    fn read_body<R: Read>(reader: &mut R) -> SharedResult<Self> {
        let from = reader.read_i32::<LittleEndian>()?;
        let to = reader.read_i32::<LittleEndian>()?;
        let success = reader.read_u8()? != 0;
        Ok(Self { from, to, success })
    }

    fn write_body<W: Write>(&self, writer: &mut W) -> SharedResult<()> {
        writer.write_i32::<LittleEndian>(self.from)?;
        writer.write_i32::<LittleEndian>(self.to)?;
        writer.write_u8(if self.success { 1 } else { 0 })?;
        Ok(())
    }
}

/// 取回精炼物品
#[derive(Debug, Clone)]
pub struct RetrieveRefineItem {
    pub from: i32,     // 源位置
    pub to: i32,       // 目标位置
    pub success: bool, // 是否成功
}

impl Packet for RetrieveRefineItem {
    const OPCODE: i16 = ServerPacketIds::RetrieveRefineItem as i16;

    fn read_body<R: Read>(reader: &mut R) -> SharedResult<Self> {
        let from = reader.read_i32::<LittleEndian>()?;
        let to = reader.read_i32::<LittleEndian>()?;
        let success = reader.read_u8()? != 0;
        Ok(Self { from, to, success })
    }

    fn write_body<W: Write>(&self, writer: &mut W) -> SharedResult<()> {
        writer.write_i32::<LittleEndian>(self.from)?;
        writer.write_i32::<LittleEndian>(self.to)?;
        writer.write_u8(if self.success { 1 } else { 0 })?;
        Ok(())
    }
}

/// 取消精炼
#[derive(Debug, Clone)]
pub struct RefineCancel {
    pub unlock: bool, // 是否解锁
}

impl Packet for RefineCancel {
    const OPCODE: i16 = ServerPacketIds::RefineCancel as i16;

    fn read_body<R: Read>(reader: &mut R) -> SharedResult<Self> {
        let unlock = reader.read_u8()? != 0;
        Ok(Self { unlock })
    }

    fn write_body<W: Write>(&self, writer: &mut W) -> SharedResult<()> {
        writer.write_u8(if self.unlock { 1 } else { 0 })?;
        Ok(())
    }
}

/// 精炼物品
#[derive(Debug, Clone)]
pub struct RefineItem {
    pub unique_id: u64, // 物品唯一 ID
}

impl Packet for RefineItem {
    const OPCODE: i16 = ServerPacketIds::RefineItem as i16;

    fn read_body<R: Read>(reader: &mut R) -> SharedResult<Self> {
        let unique_id = reader.read_u64::<LittleEndian>()?;
        Ok(Self { unique_id })
    }

    fn write_body<W: Write>(&self, writer: &mut W) -> SharedResult<()> {
        writer.write_u64::<LittleEndian>(self.unique_id)?;
        Ok(())
    }
}

/// 使用物品
#[derive(Debug, Clone)]
pub struct UseItem {
    pub unique_id: u64, // 物品唯一 ID
}

impl Packet for UseItem {
    const OPCODE: i16 = ServerPacketIds::UseItem as i16;

    fn read_body<R: Read>(reader: &mut R) -> SharedResult<Self> {
        let unique_id = reader.read_u64::<LittleEndian>()?;
        Ok(Self { unique_id })
    }

    fn write_body<W: Write>(&self, writer: &mut W) -> SharedResult<()> {
        writer.write_u64::<LittleEndian>(self.unique_id)?;
        Ok(())
    }
}

/// 丢弃物品
#[derive(Debug, Clone)]
pub struct DropItem {
    pub unique_id: u64, // 物品唯一 ID
    pub count: u32,     // 数量
    pub success: bool,  // 是否成功
}

impl Packet for DropItem {
    const OPCODE: i16 = ServerPacketIds::DropItem as i16;

    fn read_body<R: Read>(reader: &mut R) -> SharedResult<Self> {
        let unique_id = reader.read_u64::<LittleEndian>()?;
        let count = reader.read_u32::<LittleEndian>()?;
        let success = reader.read_u8()? != 0;
        Ok(Self {
            unique_id,
            count,
            success,
        })
    }

    fn write_body<W: Write>(&self, writer: &mut W) -> SharedResult<()> {
        writer.write_u64::<LittleEndian>(self.unique_id)?;
        writer.write_u32::<LittleEndian>(self.count)?;
        writer.write_u8(if self.success { 1 } else { 0 })?;
        Ok(())
    }
}

/// 合并物品（组合）
#[derive(Debug, Clone)]
pub struct CombineItem {
    pub grid: MirGridType, // 网格类型
    pub id_from: u64,      // 源物品 ID
    pub id_to: u64,        // 目标物品 ID
    pub success: bool,     // 是否成功
    pub destroy: bool,     // 是否销毁
}

impl Packet for CombineItem {
    const OPCODE: i16 = ServerPacketIds::CombineItem as i16;

    fn read_body<R: Read>(reader: &mut R) -> SharedResult<Self> {
        let grid = MirGridType::try_from(reader.read_u8()?)?;
        let id_from = reader.read_u64::<LittleEndian>()?;
        let id_to = reader.read_u64::<LittleEndian>()?;
        let success = reader.read_u8()? != 0;
        let destroy = reader.read_u8()? != 0;
        Ok(Self {
            grid,
            id_from,
            id_to,
            success,
            destroy,
        })
    }

    fn write_body<W: Write>(&self, writer: &mut W) -> SharedResult<()> {
        writer.write_u8(self.grid as u8)?;
        writer.write_u64::<LittleEndian>(self.id_from)?;
        writer.write_u64::<LittleEndian>(self.id_to)?;
        writer.write_u8(if self.success { 1 } else { 0 })?;
        writer.write_u8(if self.destroy { 1 } else { 0 })?;
        Ok(())
    }
}

/// 物品升级
#[derive(Debug, Clone)]
pub struct ItemUpgraded {
    pub item: UserItem, // 升级后的物品
}

impl Packet for ItemUpgraded {
    const OPCODE: i16 = ServerPacketIds::ItemUpgraded as i16;

    fn read_body<R: Read>(reader: &mut R) -> SharedResult<Self> {
        let item = UserItem::read_from(reader, i32::MAX, i32::MAX)?;
        Ok(Self { item })
    }

    fn write_body<W: Write>(&self, writer: &mut W) -> SharedResult<()> {
        self.item.write_to(writer)?;
        Ok(())
    }
}

/// 装备槽位物品
#[derive(Debug, Clone)]
pub struct EquipSlotItem {
    pub grid: MirGridType,    // 网格类型
    pub unique_id: u64,       // 物品唯一 ID
    pub to: i32,              // 目标位置
    pub grid_to: MirGridType, // 目标网格类型
    pub success: bool,        // 是否成功
}

impl Packet for EquipSlotItem {
    const OPCODE: i16 = ServerPacketIds::EquipSlotItem as i16;

    fn read_body<R: Read>(reader: &mut R) -> SharedResult<Self> {
        let grid = MirGridType::try_from(reader.read_u8()?)?;
        let unique_id = reader.read_u64::<LittleEndian>()?;
        let to = reader.read_i32::<LittleEndian>()?;
        let grid_to = MirGridType::try_from(reader.read_u8()?)?;
        let success = reader.read_u8()? != 0;
        Ok(Self {
            grid,
            unique_id,
            to,
            grid_to,
            success,
        })
    }

    fn write_body<W: Write>(&self, writer: &mut W) -> SharedResult<()> {
        writer.write_u8(self.grid as u8)?;
        writer.write_u64::<LittleEndian>(self.unique_id)?;
        writer.write_i32::<LittleEndian>(self.to)?;
        writer.write_u8(self.grid_to as u8)?;
        writer.write_u8(if self.success { 1 } else { 0 })?;
        Ok(())
    }
}
