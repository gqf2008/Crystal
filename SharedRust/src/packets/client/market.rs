//! Market System Packets (Client → Server)

use super::super::base::Packet;
use crate::binary::{read_dotnet_string, write_dotnet_string};
use crate::data::stats::SharedResult;
use crate::enums::{ClientPacketIds, ItemType, MarketPanelType};
use byteorder::{LittleEndian, ReadBytesExt, WriteBytesExt};
use std::io::{Read, Write};

/// Consign item to market
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ConsignItem {
    pub unique_id: u64,
    pub price: u32,
    pub panel_type: MarketPanelType,
}

impl Packet for ConsignItem {
    const OPCODE: i16 = ClientPacketIds::ConsignItem as i16;

    fn read_body<R: Read>(reader: &mut R) -> SharedResult<Self> {
        let unique_id = reader.read_u64::<LittleEndian>()?;
        let price = reader.read_u32::<LittleEndian>()?;
        let panel_type = MarketPanelType::try_from(reader.read_u8()?)?;
        Ok(Self {
            unique_id,
            price,
            panel_type,
        })
    }

    fn write_body<W: Write>(&self, writer: &mut W) -> SharedResult<()> {
        writer.write_u64::<LittleEndian>(self.unique_id)?;
        writer.write_u32::<LittleEndian>(self.price)?;
        writer.write_u8(self.panel_type as u8)?;
        Ok(())
    }
}

/// Search market for items
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MarketSearch {
    pub match_text: String,
    pub item_type: ItemType,
    pub user_mode: bool,
    pub min_shape: i16,
    pub max_shape: i16,
    pub market_type: MarketPanelType,
}

impl Packet for MarketSearch {
    const OPCODE: i16 = ClientPacketIds::MarketSearch as i16;

    fn read_body<R: Read>(reader: &mut R) -> SharedResult<Self> {
        let match_text = read_dotnet_string(reader)?;
        let item_type = ItemType::try_from(reader.read_u8()?)?;
        let user_mode = reader.read_u8()? != 0;
        let min_shape = reader.read_i16::<LittleEndian>()?;
        let max_shape = reader.read_i16::<LittleEndian>()?;
        let market_type = MarketPanelType::try_from(reader.read_u8()?)?;

        Ok(Self {
            match_text,
            item_type,
            user_mode,
            min_shape,
            max_shape,
            market_type,
        })
    }

    fn write_body<W: Write>(&self, writer: &mut W) -> SharedResult<()> {
        write_dotnet_string(writer, &self.match_text)?;
        writer.write_u8(self.item_type as u8)?;
        writer.write_u8(if self.user_mode { 1 } else { 0 })?;
        writer.write_i16::<LittleEndian>(self.min_shape)?;
        writer.write_i16::<LittleEndian>(self.max_shape)?;
        writer.write_u8(self.market_type as u8)?;
        Ok(())
    }
}

/// Refresh market listings
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct MarketRefresh;

impl Packet for MarketRefresh {
    const OPCODE: i16 = ClientPacketIds::MarketRefresh as i16;

    fn read_body<R: Read>(_reader: &mut R) -> SharedResult<Self> {
        Ok(Self)
    }

    fn write_body<W: Write>(&self, _writer: &mut W) -> SharedResult<()> {
        Ok(())
    }
}

/// Request market page
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct MarketPage {
    pub page: i32,
}

impl Packet for MarketPage {
    const OPCODE: i16 = ClientPacketIds::MarketPage as i16;

    fn read_body<R: Read>(reader: &mut R) -> SharedResult<Self> {
        let page = reader.read_i32::<LittleEndian>()?;
        Ok(Self { page })
    }

    fn write_body<W: Write>(&self, writer: &mut W) -> SharedResult<()> {
        writer.write_i32::<LittleEndian>(self.page)?;
        Ok(())
    }
}

/// Buy item from market
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct MarketBuy {
    pub auction_id: u64,
    pub bid_price: u32,
}

impl Packet for MarketBuy {
    const OPCODE: i16 = ClientPacketIds::MarketBuy as i16;

    fn read_body<R: Read>(reader: &mut R) -> SharedResult<Self> {
        let auction_id = reader.read_u64::<LittleEndian>()?;
        let bid_price = reader.read_u32::<LittleEndian>()?;
        Ok(Self {
            auction_id,
            bid_price,
        })
    }

    fn write_body<W: Write>(&self, writer: &mut W) -> SharedResult<()> {
        writer.write_u64::<LittleEndian>(self.auction_id)?;
        writer.write_u32::<LittleEndian>(self.bid_price)?;
        Ok(())
    }
}

/// Sell item immediately at market
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct MarketSellNow {
    pub auction_id: u64,
}

impl Packet for MarketSellNow {
    const OPCODE: i16 = ClientPacketIds::MarketSellNow as i16;

    fn read_body<R: Read>(reader: &mut R) -> SharedResult<Self> {
        let auction_id = reader.read_u64::<LittleEndian>()?;
        Ok(Self { auction_id })
    }

    fn write_body<W: Write>(&self, writer: &mut W) -> SharedResult<()> {
        writer.write_u64::<LittleEndian>(self.auction_id)?;
        Ok(())
    }
}

/// Get back item/gold from market
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct MarketGetBack {
    pub mode: u8, // MarketCollectionMode
    pub auction_id: u64,
}

impl Packet for MarketGetBack {
    const OPCODE: i16 = ClientPacketIds::MarketGetBack as i16;

    fn read_body<R: Read>(reader: &mut R) -> SharedResult<Self> {
        let mode = reader.read_u8()?;
        let auction_id = reader.read_u64::<LittleEndian>()?;
        Ok(Self { mode, auction_id })
    }

    fn write_body<W: Write>(&self, writer: &mut W) -> SharedResult<()> {
        writer.write_u8(self.mode)?;
        writer.write_u64::<LittleEndian>(self.auction_id)?;
        Ok(())
    }
}
