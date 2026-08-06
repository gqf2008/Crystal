// 市场/寄售系统相关数据包
use super::super::base::Packet;
use crate::data::item::UserItem;
use crate::data::stats::SharedResult;
use crate::enums::ServerPacketIds;
use byteorder::{LittleEndian, ReadBytesExt};
use std::io::Read;

/// NPCConsign - NPC寄售 (152)
#[derive(Debug, Clone)]
pub struct NPCConsign {
    // C# 原实现为空
}

impl Packet for NPCConsign {
    const OPCODE: i16 = ServerPacketIds::NPCConsign as i16;

    fn write_body<W: std::io::Write>(&self, _writer: &mut W) -> SharedResult<()> {
        // Empty packet - no data to write
        Ok(())
    }

    fn read_body<R: Read>(_reader: &mut R) -> SharedResult<Self> {
        Ok(Self {})
    }
}

/// NPCMarket - NPC市场 (153)
#[derive(Debug, Clone)]
pub struct NPCMarket {
    pub pages: Vec<String>,         // 页面列表
}

impl Packet for NPCMarket {
    const OPCODE: i16 = ServerPacketIds::NPCMarket as i16;

    fn write_body<W: std::io::Write>(&self, writer: &mut W) -> SharedResult<()> {
        use byteorder::WriteBytesExt;
        use crate::binary::write_dotnet_string;
        
        writer.write_i32::<LittleEndian>(self.pages.len() as i32)?;
        
        for page in &self.pages {
            write_dotnet_string(writer, page)?;
        }
        
        Ok(())
    }

    fn read_body<R: Read>(reader: &mut R) -> SharedResult<Self> {
        use crate::binary::read_dotnet_string;

        let count = reader.read_i32::<LittleEndian>()?;
        let mut pages = Vec::with_capacity(count as usize);

        for _ in 0..count {
            pages.push(read_dotnet_string(reader)?);
        }
        
        Ok(Self { pages })
    }
}

/// NPCMarketPage - NPC市场页面 (154)
#[derive(Debug, Clone)]
pub struct NPCMarketPage {
    pub listings: Vec<MarketListing>,  // 商品列表
}

#[derive(Debug, Clone)]
pub struct MarketListing {
    pub auction_id: u64,            // 拍卖ID
    pub item: UserItem,             // 物品
    pub seller_name: String,        // 卖家名称
    pub price: u32,                 // 价格
    pub consignment_date: i64,      // 寄售日期
}

impl Packet for NPCMarketPage {
    const OPCODE: i16 = ServerPacketIds::NPCMarketPage as i16;

    fn write_body<W: std::io::Write>(&self, writer: &mut W) -> SharedResult<()> {
        use byteorder::WriteBytesExt;
        use crate::binary::write_dotnet_string;
        
        writer.write_i32::<LittleEndian>(self.listings.len() as i32)?;
        
        for listing in &self.listings {
            writer.write_u64::<LittleEndian>(listing.auction_id)?;
            listing.item.write_to(writer)?;
            write_dotnet_string(writer, &listing.seller_name)?;
            writer.write_u32::<LittleEndian>(listing.price)?;
            writer.write_i64::<LittleEndian>(listing.consignment_date)?;
        }
        
        Ok(())
    }

    fn read_body<R: Read>(reader: &mut R) -> SharedResult<Self> {
        use crate::binary::read_dotnet_string;
        let count = reader.read_i32::<LittleEndian>()?;
        let mut listings = Vec::with_capacity(count as usize);
        
        for _ in 0..count {
            let auction_id = reader.read_u64::<LittleEndian>()?;
            let item = UserItem::read_from(reader, i32::MAX, i32::MAX)?;
            
            let seller_name = read_dotnet_string(reader)?;
            
            let price = reader.read_u32::<LittleEndian>()?;
            let consignment_date = reader.read_i64::<LittleEndian>()?;
            
            listings.push(MarketListing {
                auction_id,
                item,
                seller_name,
                price,
                consignment_date,
            });
        }
        
        Ok(Self { listings })
    }
}

/// ConsignItem - 寄售物品 (155)
#[derive(Debug, Clone)]
pub struct ConsignItem {
    pub unique_id: u64,             // 物品唯一ID
    pub success: bool,              // 是否成功
}

impl Packet for ConsignItem {
    const OPCODE: i16 = ServerPacketIds::ConsignItem as i16;

    fn write_body<W: std::io::Write>(&self, writer: &mut W) -> SharedResult<()> {
        use byteorder::WriteBytesExt;
        
        writer.write_u64::<LittleEndian>(self.unique_id)?;
        writer.write_u8(if self.success { 1 } else { 0 })?;
        
        Ok(())
    }

    fn read_body<R: Read>(reader: &mut R) -> SharedResult<Self> {
        let unique_id = reader.read_u64::<LittleEndian>()?;
        let success = reader.read_u8()? != 0;
        Ok(Self {
            unique_id,
            success,
        })
    }
}

/// MarketFail - 市场操作失败 (156)
#[derive(Debug, Clone)]
pub struct MarketFail {
    pub reason: u8,                 // 失败原因
}

impl Packet for MarketFail {
    const OPCODE: i16 = ServerPacketIds::MarketFail as i16;

    fn write_body<W: std::io::Write>(&self, writer: &mut W) -> SharedResult<()> {
        use byteorder::WriteBytesExt;
        
        writer.write_u8(self.reason)?;
        
        Ok(())
    }

    fn read_body<R: Read>(reader: &mut R) -> SharedResult<Self> {
        let reason = reader.read_u8()?;
        Ok(Self { reason })
    }
}

/// MarketSuccess - 市场操作成功 (157)
#[derive(Debug, Clone)]
pub struct MarketSuccess {
    pub message: String,            // 成功消息
}

impl Packet for MarketSuccess {
    const OPCODE: i16 = ServerPacketIds::MarketSuccess as i16;

    fn write_body<W: std::io::Write>(&self, writer: &mut W) -> SharedResult<()> {
        use crate::binary::write_dotnet_string;
        
        write_dotnet_string(writer, &self.message)?;
        
        Ok(())
    }

    fn read_body<R: Read>(reader: &mut R) -> SharedResult<Self> {
        use crate::binary::read_dotnet_string;
        let message = read_dotnet_string(reader)?;
        Ok(Self { message })
    }
}
