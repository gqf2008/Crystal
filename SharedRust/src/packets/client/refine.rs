// 精炼相关数据包
use super::super::base::Packet;
use crate::data::stats::SharedResult;
use crate::enums::{ClientPacketIds, MirGridType};
use byteorder::{LittleEndian, WriteBytesExt};
use std::io::Read;

/// DepositRefineItem - 存入精炼物品 (24)
#[derive(Debug, Clone)]
pub struct DepositRefineItem {
    pub from: i32,                  // 来源槽位
    pub to: i32,                    // 目标槽位
}

impl Packet for DepositRefineItem {
    const OPCODE: i16 = ClientPacketIds::DepositRefineItem as i16;

    fn write_body<W: std::io::Write>(&self, writer: &mut W) -> SharedResult<()> {
        writer.write_i32::<LittleEndian>(self.from)?;
        writer.write_i32::<LittleEndian>(self.to)?;
        Ok(())
    }

    fn read_body<R: Read>(_reader: &mut R) -> SharedResult<Self> {
        unimplemented!("Client packets don't need read_body")
    }
}

/// RetrieveRefineItem - 取回精炼物品 (25)
#[derive(Debug, Clone)]
pub struct RetrieveRefineItem {
    pub from: i32,                  // 来源槽位
    pub to: i32,                    // 目标槽位
}

impl Packet for RetrieveRefineItem {
    const OPCODE: i16 = ClientPacketIds::RetrieveRefineItem as i16;

    fn write_body<W: std::io::Write>(&self, writer: &mut W) -> SharedResult<()> {
        writer.write_i32::<LittleEndian>(self.from)?;
        writer.write_i32::<LittleEndian>(self.to)?;
        Ok(())
    }

    fn read_body<R: Read>(_reader: &mut R) -> SharedResult<Self> {
        unimplemented!("Client packets don't need read_body")
    }
}

/// RefineCancel - 取消精炼 (26)
#[derive(Debug, Clone)]
pub struct RefineCancel {}

impl Packet for RefineCancel {
    const OPCODE: i16 = ClientPacketIds::RefineCancel as i16;

    fn write_body<W: std::io::Write>(&self, _writer: &mut W) -> SharedResult<()> {
        Ok(())
    }

    fn read_body<R: Read>(_reader: &mut R) -> SharedResult<Self> {
        unimplemented!("Client packets don't need read_body")
    }
}

/// RefineItem - 精炼物品 (27)
#[derive(Debug, Clone)]
pub struct RefineItem {
    pub unique_id: u64,             // 物品唯一ID
}

impl Packet for RefineItem {
    const OPCODE: i16 = ClientPacketIds::RefineItem as i16;

    fn write_body<W: std::io::Write>(&self, writer: &mut W) -> SharedResult<()> {
        writer.write_u64::<LittleEndian>(self.unique_id)?;
        Ok(())
    }

    fn read_body<R: Read>(_reader: &mut R) -> SharedResult<Self> {
        unimplemented!("Client packets don't need read_body")
    }
}

/// CheckRefine - 检查精炼 (28)
#[derive(Debug, Clone)]
pub struct CheckRefine {
    pub unique_id: u64,             // 物品唯一ID
}

impl Packet for CheckRefine {
    const OPCODE: i16 = ClientPacketIds::CheckRefine as i16;

    fn write_body<W: std::io::Write>(&self, writer: &mut W) -> SharedResult<()> {
        writer.write_u64::<LittleEndian>(self.unique_id)?;
        Ok(())
    }

    fn read_body<R: Read>(_reader: &mut R) -> SharedResult<Self> {
        unimplemented!("Client packets don't need read_body")
    }
}

/// ReplaceWedRing - 替换结婚戒指 (29)
#[derive(Debug, Clone)]
pub struct ReplaceWedRing {
    pub unique_id: u64,             // 物品唯一ID
}

impl Packet for ReplaceWedRing {
    const OPCODE: i16 = ClientPacketIds::ReplaceWedRing as i16;

    fn write_body<W: std::io::Write>(&self, writer: &mut W) -> SharedResult<()> {
        writer.write_u64::<LittleEndian>(self.unique_id)?;
        Ok(())
    }

    fn read_body<R: Read>(_reader: &mut R) -> SharedResult<Self> {
        unimplemented!("Client packets don't need read_body")
    }
}

/// DepositTradeItem - 存入交易物品 (30)
#[derive(Debug, Clone)]
pub struct DepositTradeItem {
    pub from_slot: i32,             // 来源槽位
}

impl Packet for DepositTradeItem {
    const OPCODE: i16 = ClientPacketIds::DepositTradeItem as i16;

    fn write_body<W: std::io::Write>(&self, writer: &mut W) -> SharedResult<()> {
        writer.write_i32::<LittleEndian>(self.from_slot)?;
        Ok(())
    }

    fn read_body<R: Read>(_reader: &mut R) -> SharedResult<Self> {
        unimplemented!("Client packets don't need read_body")
    }
}

/// RetrieveTradeItem - 取回交易物品 (31)
#[derive(Debug, Clone)]
pub struct RetrieveTradeItem {
    pub from_slot: i32,             // 来源槽位
}

impl Packet for RetrieveTradeItem {
    const OPCODE: i16 = ClientPacketIds::RetrieveTradeItem as i16;

    fn write_body<W: std::io::Write>(&self, writer: &mut W) -> SharedResult<()> {
        writer.write_i32::<LittleEndian>(self.from_slot)?;
        Ok(())
    }

    fn read_body<R: Read>(_reader: &mut R) -> SharedResult<Self> {
        unimplemented!("Client packets don't need read_body")
    }
}

/// TakeBackHeroItem - 取回英雄物品 (32)
#[derive(Debug, Clone)]
pub struct TakeBackHeroItem {
    pub from: i32,                  // 来源槽位
    pub to: i32,                    // 目标槽位
    pub grid_from: MirGridType,     // 来源网格类型
    pub grid_to: MirGridType,       // 目标网格类型
}

impl Packet for TakeBackHeroItem {
    const OPCODE: i16 = ClientPacketIds::TakeBackHeroItem as i16;

    fn write_body<W: std::io::Write>(&self, writer: &mut W) -> SharedResult<()> {
        writer.write_i32::<LittleEndian>(self.from)?;
        writer.write_i32::<LittleEndian>(self.to)?;
        writer.write_u8(self.grid_from as u8)?;
        writer.write_u8(self.grid_to as u8)?;
        Ok(())
    }

    fn read_body<R: Read>(_reader: &mut R) -> SharedResult<Self> {
        unimplemented!("Client packets don't need read_body")
    }
}

/// TransferHeroItem - 转移英雄物品 (33)
#[derive(Debug, Clone)]
pub struct TransferHeroItem {
    pub from: i32,                  // 来源槽位
    pub to: i32,                    // 目标槽位
    pub grid_from: MirGridType,     // 来源网格类型
    pub grid_to: MirGridType,       // 目标网格类型
}

impl Packet for TransferHeroItem {
    const OPCODE: i16 = ClientPacketIds::TransferHeroItem as i16;

    fn write_body<W: std::io::Write>(&self, writer: &mut W) -> SharedResult<()> {
        writer.write_i32::<LittleEndian>(self.from)?;
        writer.write_i32::<LittleEndian>(self.to)?;
        writer.write_u8(self.grid_from as u8)?;
        writer.write_u8(self.grid_to as u8)?;
        Ok(())
    }

    fn read_body<R: Read>(_reader: &mut R) -> SharedResult<Self> {
        unimplemented!("Client packets don't need read_body")
    }
}
