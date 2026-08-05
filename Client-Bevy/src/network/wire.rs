use mir2_shared::packets::base::Packet;

pub struct GuildStorageItemChangeWire {
    pub change_type: u8, // 0=存入 1=取出 2=移动 3=请求列表
    pub grid: u8,
    pub unique_id: u64,
    pub count: u32,
}

impl Packet for GuildStorageItemChangeWire {
    const OPCODE: i16 = mir2_shared::enums::ClientPacketIds::GuildStorageItemChange as i16;

    fn read_body<R: std::io::Read>(
        reader: &mut R,
    ) -> mir2_shared::data::stats::SharedResult<Self> {
        use byteorder::{LittleEndian, ReadBytesExt};
        Ok(Self {
            change_type: reader.read_u8()?,
            grid: reader.read_u8()?,
            unique_id: reader.read_u64::<LittleEndian>()?,
            count: reader.read_u32::<LittleEndian>()?,
        })
    }

    fn write_body<W: std::io::Write>(
        &self,
        writer: &mut W,
    ) -> mir2_shared::data::stats::SharedResult<()> {
        use byteorder::{LittleEndian, WriteBytesExt};
        writer.write_u8(self.change_type)?;
        writer.write_u8(self.grid)?;
        writer.write_u64::<LittleEndian>(self.unique_id)?;
        writer.write_u32::<LittleEndian>(self.count)?;
        Ok(())
    }
}

/// 举报（M45：gate 解析 [type u32][description dotnet]，与 SharedRust 不一致）
#[derive(Debug, Clone)]
pub struct ReportIssueWire {
    pub issue_type: u32,
    pub description: String,
}

impl Packet for ReportIssueWire {
    const OPCODE: i16 = mir2_shared::enums::ClientPacketIds::ReportIssue as i16;

    fn read_body<R: std::io::Read>(
        reader: &mut R,
    ) -> mir2_shared::data::stats::SharedResult<Self> {
        use byteorder::{LittleEndian, ReadBytesExt};
        Ok(Self {
            issue_type: reader.read_u32::<LittleEndian>()?,
            description: mir2_shared::binary::read_dotnet_string(reader)?,
        })
    }

    fn write_body<W: std::io::Write>(
        &self,
        writer: &mut W,
    ) -> mir2_shared::data::stats::SharedResult<()> {
        use byteorder::{LittleEndian, WriteBytesExt};
        writer.write_u32::<LittleEndian>(self.issue_type)?;
        mir2_shared::binary::write_dotnet_string(writer, &self.description)?;
        Ok(())
    }
}

/// 宠物列表请求（M47：gate 解析 [request_updates u8]）
#[derive(Debug, Clone, Copy)]
pub struct CreatureRequestWire {
    pub request: bool,
}

impl Packet for CreatureRequestWire {
    const OPCODE: i16 = mir2_shared::enums::ClientPacketIds::RequestIntelligentCreatureUpdates as i16;

    fn read_body<R: std::io::Read>(
        reader: &mut R,
    ) -> mir2_shared::data::stats::SharedResult<Self> {
        use byteorder::ReadBytesExt;
        Ok(Self {
            request: reader.read_u8()? != 0,
        })
    }

    fn write_body<W: std::io::Write>(
        &self,
        writer: &mut W,
    ) -> mir2_shared::data::stats::SharedResult<()> {
        use byteorder::WriteBytesExt;
        writer.write_u8(if self.request { 1 } else { 0 })?;
        Ok(())
    }
}

/// 英雄切换（M48：gate 解析 [hero_index u8]）
#[derive(Debug, Clone, Copy)]
pub struct ChangeHeroWire {
    pub hero_index: u8,
}

impl Packet for ChangeHeroWire {
    const OPCODE: i16 = mir2_shared::enums::ClientPacketIds::ChangeHero as i16;

    fn read_body<R: std::io::Read>(
        reader: &mut R,
    ) -> mir2_shared::data::stats::SharedResult<Self> {
        use byteorder::ReadBytesExt;
        Ok(Self {
            hero_index: reader.read_u8()?,
        })
    }

    fn write_body<W: std::io::Write>(
        &self,
        writer: &mut W,
    ) -> mir2_shared::data::stats::SharedResult<()> {
        use byteorder::WriteBytesExt;
        writer.write_u8(self.hero_index)?;
        Ok(())
    }
}

/// 英雄→主背包取回（#203：C# [from i32][to i32]，英雄格 → 主背包格）
#[derive(Debug, Clone, Copy)]
pub struct TakeBackHeroItemWire {
    pub from: i32,
    pub to: i32,
}

impl Packet for TakeBackHeroItemWire {
    const OPCODE: i16 = mir2_shared::enums::ClientPacketIds::TakeBackHeroItem as i16;

    fn read_body<R: std::io::Read>(
        reader: &mut R,
    ) -> mir2_shared::data::stats::SharedResult<Self> {
        use byteorder::ReadBytesExt;
        Ok(Self {
            from: reader.read_i32::<byteorder::LittleEndian>()?,
            to: reader.read_i32::<byteorder::LittleEndian>()?,
        })
    }

    fn write_body<W: std::io::Write>(
        &self,
        writer: &mut W,
    ) -> mir2_shared::data::stats::SharedResult<()> {
        use byteorder::WriteBytesExt;
        writer.write_i32::<byteorder::LittleEndian>(self.from)?;
        writer.write_i32::<byteorder::LittleEndian>(self.to)?;
        Ok(())
    }
}

/// 主背包→英雄转移（#203：C# [from i32][to i32]，主背包格 → 英雄格）
#[derive(Debug, Clone, Copy)]
pub struct TransferHeroItemWire {
    pub from: i32,
    pub to: i32,
}

impl Packet for TransferHeroItemWire {
    const OPCODE: i16 = mir2_shared::enums::ClientPacketIds::TransferHeroItem as i16;

    fn read_body<R: std::io::Read>(
        reader: &mut R,
    ) -> mir2_shared::data::stats::SharedResult<Self> {
        use byteorder::ReadBytesExt;
        Ok(Self {
            from: reader.read_i32::<byteorder::LittleEndian>()?,
            to: reader.read_i32::<byteorder::LittleEndian>()?,
        })
    }

    fn write_body<W: std::io::Write>(
        &self,
        writer: &mut W,
    ) -> mir2_shared::data::stats::SharedResult<()> {
        use byteorder::WriteBytesExt;
        writer.write_i32::<byteorder::LittleEndian>(self.from)?;
        writer.write_i32::<byteorder::LittleEndian>(self.to)?;
        Ok(())
    }
}
/// 婚姻客户端包（M49：SharedRust 为空包，gate 期望 dotnet，手动构造）
#[derive(Debug, Clone)]
pub struct MarriageRequestWire {
    pub target_name: String,
}

impl Packet for MarriageRequestWire {
    const OPCODE: i16 = mir2_shared::enums::ClientPacketIds::MarriageRequest as i16;

    fn read_body<R: std::io::Read>(
        reader: &mut R,
    ) -> mir2_shared::data::stats::SharedResult<Self> {
        Ok(Self {
            target_name: mir2_shared::binary::read_dotnet_string(reader)?,
        })
    }

    fn write_body<W: std::io::Write>(
        &self,
        writer: &mut W,
    ) -> mir2_shared::data::stats::SharedResult<()> {
        mir2_shared::binary::write_dotnet_string(writer, &self.target_name)?;
        Ok(())
    }
}

#[derive(Debug, Clone)]
pub struct DivorceRequestWire {
    pub partner_name: String,
}

impl Packet for DivorceRequestWire {
    const OPCODE: i16 = mir2_shared::enums::ClientPacketIds::DivorceRequest as i16;

    fn read_body<R: std::io::Read>(
        reader: &mut R,
    ) -> mir2_shared::data::stats::SharedResult<Self> {
        Ok(Self {
            partner_name: mir2_shared::binary::read_dotnet_string(reader)?,
        })
    }

    fn write_body<W: std::io::Write>(
        &self,
        writer: &mut W,
    ) -> mir2_shared::data::stats::SharedResult<()> {
        mir2_shared::binary::write_dotnet_string(writer, &self.partner_name)?;
        Ok(())
    }
}

/// 允许拜师开关（ServerRust gate 解析 [allow u8]，与 SharedRust 空包不一致，手动构造）
#[derive(Debug, Clone, Copy)]
pub struct AllowMentorWire {
    pub allow: bool,
}

impl Packet for AllowMentorWire {
    const OPCODE: i16 = mir2_shared::enums::ClientPacketIds::AllowMentor as i16;

    fn read_body<R: std::io::Read>(
        reader: &mut R,
    ) -> mir2_shared::data::stats::SharedResult<Self> {
        use byteorder::ReadBytesExt;
        Ok(Self {
            allow: reader.read_u8()? != 0,
        })
    }

    fn write_body<W: std::io::Write>(
        &self,
        writer: &mut W,
    ) -> mir2_shared::data::stats::SharedResult<()> {
        use byteorder::WriteBytesExt;
        writer.write_u8(if self.allow { 1 } else { 0 })?;
        Ok(())
    }
}

/// 市场客户端包（M34）
/// ServerRust gate 实际解析 wire 与 SharedRust 客户端包结构不一致，手动构造：
///   ConsignItem: [unique_id u32][price u32][duration u32]（gate 要求 ≥12 字节）
///   MarketSearch: [item_index u32]   MarketPage: [page u32]
///   MarketBuy: [listing_id u32]      MarketGetBack: [listing_id u32]
///   MarketSellNow: [unique_id u32][price u32]
#[derive(Debug, Clone, Copy)]
pub struct MarketConsignWire {
    pub unique_id: u32,
    pub price: u32,
    pub duration: u32,
}

impl Packet for MarketConsignWire {
    const OPCODE: i16 = mir2_shared::enums::ClientPacketIds::ConsignItem as i16;

    fn read_body<R: std::io::Read>(
        reader: &mut R,
    ) -> mir2_shared::data::stats::SharedResult<Self> {
        use byteorder::{LittleEndian, ReadBytesExt};
        Ok(Self {
            unique_id: reader.read_u32::<LittleEndian>()?,
            price: reader.read_u32::<LittleEndian>()?,
            duration: reader.read_u32::<LittleEndian>()?,
        })
    }

    fn write_body<W: std::io::Write>(
        &self,
        writer: &mut W,
    ) -> mir2_shared::data::stats::SharedResult<()> {
        use byteorder::{LittleEndian, WriteBytesExt};
        writer.write_u32::<LittleEndian>(self.unique_id)?;
        writer.write_u32::<LittleEndian>(self.price)?;
        writer.write_u32::<LittleEndian>(self.duration)?;
        Ok(())
    }
}

#[derive(Debug, Clone, Copy)]
pub struct MarketSearchWire {
    pub item_index: u32,
}

impl Packet for MarketSearchWire {
    const OPCODE: i16 = mir2_shared::enums::ClientPacketIds::MarketSearch as i16;

    fn read_body<R: std::io::Read>(
        reader: &mut R,
    ) -> mir2_shared::data::stats::SharedResult<Self> {
        use byteorder::{LittleEndian, ReadBytesExt};
        Ok(Self {
            item_index: reader.read_u32::<LittleEndian>()?,
        })
    }

    fn write_body<W: std::io::Write>(
        &self,
        writer: &mut W,
    ) -> mir2_shared::data::stats::SharedResult<()> {
        use byteorder::{LittleEndian, WriteBytesExt};
        writer.write_u32::<LittleEndian>(self.item_index)?;
        Ok(())
    }
}

#[derive(Debug, Clone, Copy)]
pub struct MarketPageWire {
    pub page: u32,
}

impl Packet for MarketPageWire {
    const OPCODE: i16 = mir2_shared::enums::ClientPacketIds::MarketPage as i16;

    fn read_body<R: std::io::Read>(
        reader: &mut R,
    ) -> mir2_shared::data::stats::SharedResult<Self> {
        use byteorder::{LittleEndian, ReadBytesExt};
        Ok(Self {
            page: reader.read_u32::<LittleEndian>()?,
        })
    }

    fn write_body<W: std::io::Write>(
        &self,
        writer: &mut W,
    ) -> mir2_shared::data::stats::SharedResult<()> {
        use byteorder::{LittleEndian, WriteBytesExt};
        writer.write_u32::<LittleEndian>(self.page)?;
        Ok(())
    }
}

#[derive(Debug, Clone, Copy)]
pub struct MarketBuyWire {
    pub listing_id: u32,
}

impl Packet for MarketBuyWire {
    const OPCODE: i16 = mir2_shared::enums::ClientPacketIds::MarketBuy as i16;

    fn read_body<R: std::io::Read>(
        reader: &mut R,
    ) -> mir2_shared::data::stats::SharedResult<Self> {
        use byteorder::{LittleEndian, ReadBytesExt};
        Ok(Self {
            listing_id: reader.read_u32::<LittleEndian>()?,
        })
    }

    fn write_body<W: std::io::Write>(
        &self,
        writer: &mut W,
    ) -> mir2_shared::data::stats::SharedResult<()> {
        use byteorder::{LittleEndian, WriteBytesExt};
        writer.write_u32::<LittleEndian>(self.listing_id)?;
        Ok(())
    }
}

#[derive(Debug, Clone, Copy)]
pub struct MarketGetBackWire {
    pub listing_id: u32,
}

impl Packet for MarketGetBackWire {
    const OPCODE: i16 = mir2_shared::enums::ClientPacketIds::MarketGetBack as i16;

    fn read_body<R: std::io::Read>(
        reader: &mut R,
    ) -> mir2_shared::data::stats::SharedResult<Self> {
        use byteorder::{LittleEndian, ReadBytesExt};
        Ok(Self {
            listing_id: reader.read_u32::<LittleEndian>()?,
        })
    }

    fn write_body<W: std::io::Write>(
        &self,
        writer: &mut W,
    ) -> mir2_shared::data::stats::SharedResult<()> {
        use byteorder::{LittleEndian, WriteBytesExt};
        writer.write_u32::<LittleEndian>(self.listing_id)?;
        Ok(())
    }
}

#[derive(Debug, Clone, Copy)]
pub struct MarketSellNowWire {
    pub unique_id: u32,
    pub price: u32,
}

impl Packet for MarketSellNowWire {
    const OPCODE: i16 = mir2_shared::enums::ClientPacketIds::MarketSellNow as i16;

    fn read_body<R: std::io::Read>(
        reader: &mut R,
    ) -> mir2_shared::data::stats::SharedResult<Self> {
        use byteorder::{LittleEndian, ReadBytesExt};
        Ok(Self {
            unique_id: reader.read_u32::<LittleEndian>()?,
            price: reader.read_u32::<LittleEndian>()?,
        })
    }

    fn write_body<W: std::io::Write>(
        &self,
        writer: &mut W,
    ) -> mir2_shared::data::stats::SharedResult<()> {
        use byteorder::{LittleEndian, WriteBytesExt};
        writer.write_u32::<LittleEndian>(self.unique_id)?;
        writer.write_u32::<LittleEndian>(self.price)?;
        Ok(())
    }
}
