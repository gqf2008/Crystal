//! Trade System Packets (Client → Server)

use super::super::base::Packet;
use crate::data::stats::SharedResult;
use crate::enums::ClientPacketIds;
use byteorder::{LittleEndian, ReadBytesExt, WriteBytesExt};
use std::io::{Read, Write};

/// Request trade with another player
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct TradeRequest;

impl Packet for TradeRequest {
    const OPCODE: i16 = ClientPacketIds::TradeRequest as i16;

    fn read_body<R: Read>(_reader: &mut R) -> SharedResult<Self> {
        Ok(Self)
    }

    fn write_body<W: Write>(&self, _writer: &mut W) -> SharedResult<()> {
        Ok(())
    }
}

/// Reply to trade request
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct TradeReply {
    pub accept_invite: bool,
}

impl Packet for TradeReply {
    const OPCODE: i16 = ClientPacketIds::TradeReply as i16;

    fn read_body<R: Read>(reader: &mut R) -> SharedResult<Self> {
        let accept_invite = reader.read_u8()? != 0;
        Ok(Self { accept_invite })
    }

    fn write_body<W: Write>(&self, writer: &mut W) -> SharedResult<()> {
        writer.write_u8(if self.accept_invite { 1 } else { 0 })?;
        Ok(())
    }
}

/// Add gold to trade
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct TradeGold {
    pub amount: u32,
}

impl Packet for TradeGold {
    const OPCODE: i16 = ClientPacketIds::TradeGold as i16;

    fn read_body<R: Read>(reader: &mut R) -> SharedResult<Self> {
        let amount = reader.read_u32::<LittleEndian>()?;
        Ok(Self { amount })
    }

    fn write_body<W: Write>(&self, writer: &mut W) -> SharedResult<()> {
        writer.write_u32::<LittleEndian>(self.amount)?;
        Ok(())
    }
}

/// Confirm trade (lock in)
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct TradeConfirm {
    pub locked: bool,
}

impl Packet for TradeConfirm {
    const OPCODE: i16 = ClientPacketIds::TradeConfirm as i16;

    fn read_body<R: Read>(reader: &mut R) -> SharedResult<Self> {
        let locked = reader.read_u8()? != 0;
        Ok(Self { locked })
    }

    fn write_body<W: Write>(&self, writer: &mut W) -> SharedResult<()> {
        writer.write_u8(if self.locked { 1 } else { 0 })?;
        Ok(())
    }
}

/// Deposit item into trade (from inventory slot to trade slot)
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct DepositTradeItem {
    pub from: i32,
    pub to: i32,
}

impl Packet for DepositTradeItem {
    const OPCODE: i16 = ClientPacketIds::DepositTradeItem as i16;

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

/// Retrieve item from trade (from trade slot to inventory)
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct RetrieveTradeItem {
    pub from: i32,
    pub to: i32,
}

impl Packet for RetrieveTradeItem {
    const OPCODE: i16 = ClientPacketIds::RetrieveTradeItem as i16;

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

/// Cancel trade
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct TradeCancel;

impl Packet for TradeCancel {
    const OPCODE: i16 = ClientPacketIds::TradeCancel as i16;

    fn read_body<R: Read>(_reader: &mut R) -> SharedResult<Self> {
        Ok(Self)
    }

    fn write_body<W: Write>(&self, _writer: &mut W) -> SharedResult<()> {
        Ok(())
    }
}
