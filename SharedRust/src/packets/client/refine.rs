// 精炼相关数据包
use super::super::base::Packet;
use crate::data::stats::SharedResult;
use crate::enums::ClientPacketIds;
use byteorder::{LittleEndian, WriteBytesExt};
use std::io::Read;

/// DepositRefineItem - 存入精炼物品 (24)
#[derive(Debug, Clone)]
pub struct DepositRefineItem {
    pub from: i32, // 来源槽位
    pub to: i32,   // 目标槽位
}

impl Packet for DepositRefineItem {
    const OPCODE: i16 = ClientPacketIds::DepositRefineItem as i16;

    fn write_body<W: std::io::Write>(&self, writer: &mut W) -> SharedResult<()> {
        writer.write_i32::<LittleEndian>(self.from)?;
        writer.write_i32::<LittleEndian>(self.to)?;
        Ok(())
    }

    fn read_body<R: Read>(reader: &mut R) -> SharedResult<Self> {
        use byteorder::ReadBytesExt;
        Ok(Self {
            from: reader.read_i32::<LittleEndian>()?,
            to: reader.read_i32::<LittleEndian>()?,
        })
    }
}

/// RetrieveRefineItem - 取回精炼物品 (25)
#[derive(Debug, Clone)]
pub struct RetrieveRefineItem {
    pub from: i32, // 来源槽位
    pub to: i32,   // 目标槽位
}

impl Packet for RetrieveRefineItem {
    const OPCODE: i16 = ClientPacketIds::RetrieveRefineItem as i16;

    fn write_body<W: std::io::Write>(&self, writer: &mut W) -> SharedResult<()> {
        writer.write_i32::<LittleEndian>(self.from)?;
        writer.write_i32::<LittleEndian>(self.to)?;
        Ok(())
    }

    fn read_body<R: Read>(reader: &mut R) -> SharedResult<Self> {
        use byteorder::ReadBytesExt;
        Ok(Self {
            from: reader.read_i32::<LittleEndian>()?,
            to: reader.read_i32::<LittleEndian>()?,
        })
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
        Ok(Self {})
    }
}

/// RefineItem - 精炼物品 (27)
#[derive(Debug, Clone)]
pub struct RefineItem {
    pub unique_id: u64, // 物品唯一ID
}

impl Packet for RefineItem {
    const OPCODE: i16 = ClientPacketIds::RefineItem as i16;

    fn write_body<W: std::io::Write>(&self, writer: &mut W) -> SharedResult<()> {
        writer.write_u64::<LittleEndian>(self.unique_id)?;
        Ok(())
    }

    fn read_body<R: Read>(reader: &mut R) -> SharedResult<Self> {
        use byteorder::ReadBytesExt;
        Ok(Self {
            unique_id: reader.read_u64::<LittleEndian>()?,
        })
    }
}

/// CheckRefine - 检查精炼 (28)
#[derive(Debug, Clone)]
pub struct CheckRefine {
    pub unique_id: u64, // 物品唯一ID
}

impl Packet for CheckRefine {
    const OPCODE: i16 = ClientPacketIds::CheckRefine as i16;

    fn write_body<W: std::io::Write>(&self, writer: &mut W) -> SharedResult<()> {
        writer.write_u64::<LittleEndian>(self.unique_id)?;
        Ok(())
    }

    fn read_body<R: Read>(reader: &mut R) -> SharedResult<Self> {
        use byteorder::ReadBytesExt;
        Ok(Self {
            unique_id: reader.read_u64::<LittleEndian>()?,
        })
    }
}

/// ReplaceWedRing - 替换结婚戒指 (29)
#[derive(Debug, Clone)]
pub struct ReplaceWedRing {
    pub unique_id: u64, // 物品唯一ID
}

impl Packet for ReplaceWedRing {
    const OPCODE: i16 = ClientPacketIds::ReplaceWedRing as i16;

    fn write_body<W: std::io::Write>(&self, writer: &mut W) -> SharedResult<()> {
        writer.write_u64::<LittleEndian>(self.unique_id)?;
        Ok(())
    }

    fn read_body<R: Read>(reader: &mut R) -> SharedResult<Self> {
        use byteorder::ReadBytesExt;
        Ok(Self {
            unique_id: reader.read_u64::<LittleEndian>()?,
        })
    }
}

/// DepositTradeItem - 存入交易物品 (30)
#[derive(Debug, Clone)]
pub struct DepositTradeItem {
    pub from: i32, // 来源槽位
    pub to: i32,   // 目标槽位
}

impl Packet for DepositTradeItem {
    const OPCODE: i16 = ClientPacketIds::DepositTradeItem as i16;

    fn write_body<W: std::io::Write>(&self, writer: &mut W) -> SharedResult<()> {
        writer.write_i32::<LittleEndian>(self.from)?;
        writer.write_i32::<LittleEndian>(self.to)?;
        Ok(())
    }

    fn read_body<R: Read>(reader: &mut R) -> SharedResult<Self> {
        use byteorder::ReadBytesExt;
        Ok(Self {
            from: reader.read_i32::<LittleEndian>()?,
            to: reader.read_i32::<LittleEndian>()?,
        })
    }
}

/// RetrieveTradeItem - 取回交易物品 (31)
#[derive(Debug, Clone)]
pub struct RetrieveTradeItem {
    pub from: i32, // 来源槽位
    pub to: i32,   // 目标槽位
}

impl Packet for RetrieveTradeItem {
    const OPCODE: i16 = ClientPacketIds::RetrieveTradeItem as i16;

    fn write_body<W: std::io::Write>(&self, writer: &mut W) -> SharedResult<()> {
        writer.write_i32::<LittleEndian>(self.from)?;
        writer.write_i32::<LittleEndian>(self.to)?;
        Ok(())
    }

    fn read_body<R: Read>(reader: &mut R) -> SharedResult<Self> {
        use byteorder::ReadBytesExt;
        Ok(Self {
            from: reader.read_i32::<LittleEndian>()?,
            to: reader.read_i32::<LittleEndian>()?,
        })
    }
}

/// TakeBackHeroItem - 取回英雄物品 (32)
#[derive(Debug, Clone)]
pub struct TakeBackHeroItem {
    pub from: i32, // 来源槽位
    pub to: i32,   // 目标槽位
}

impl Packet for TakeBackHeroItem {
    const OPCODE: i16 = ClientPacketIds::TakeBackHeroItem as i16;

    fn write_body<W: std::io::Write>(&self, writer: &mut W) -> SharedResult<()> {
        writer.write_i32::<LittleEndian>(self.from)?;
        writer.write_i32::<LittleEndian>(self.to)?;
        Ok(())
    }

    fn read_body<R: Read>(reader: &mut R) -> SharedResult<Self> {
        use byteorder::ReadBytesExt;
        Ok(Self {
            from: reader.read_i32::<LittleEndian>()?,
            to: reader.read_i32::<LittleEndian>()?,
        })
    }
}

/// TransferHeroItem - 转移英雄物品 (33)
#[derive(Debug, Clone)]
pub struct TransferHeroItem {
    pub from: i32, // 来源槽位
    pub to: i32,   // 目标槽位
}

impl Packet for TransferHeroItem {
    const OPCODE: i16 = ClientPacketIds::TransferHeroItem as i16;

    fn write_body<W: std::io::Write>(&self, writer: &mut W) -> SharedResult<()> {
        writer.write_i32::<LittleEndian>(self.from)?;
        writer.write_i32::<LittleEndian>(self.to)?;
        Ok(())
    }

    fn read_body<R: Read>(reader: &mut R) -> SharedResult<Self> {
        use byteorder::ReadBytesExt;
        Ok(Self {
            from: reader.read_i32::<LittleEndian>()?,
            to: reader.read_i32::<LittleEndian>()?,
        })
    }
}
