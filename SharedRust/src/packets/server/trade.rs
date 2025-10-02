// 交易系统数据包解析
//! Trade System Packets
//!
//! This module contains all trade-related packet definitions and parsers.

use byteorder::{LittleEndian, ReadBytesExt};
use crate::data::item::UserItem;
use std::io::Cursor;

// 辅助函数
fn read_bool(cursor: &mut Cursor<&[u8]>) -> Result<bool, std::io::Error> {
    Ok(cursor.read_u8()? != 0)
}

fn read_dotnet_string(cursor: &mut Cursor<&[u8]>) -> Result<String, std::io::Error> {
    let len = cursor.read_u16::<LittleEndian>()? as usize;
    let mut buf = vec![0u8; len];
    std::io::Read::read_exact(cursor, &mut buf)?;
    String::from_utf8(buf).map_err(|e| std::io::Error::new(std::io::ErrorKind::InvalidData, e))
}

// ==================== 数据结构 ====================

#[derive(Debug, Clone)]
pub struct TradeRequest {
    pub name: String,
}

#[derive(Debug, Clone)]
pub struct TradeAccept {
    pub name: String,
}

#[derive(Debug, Clone)]
pub struct TradeGold {
    pub amount: u32,
}

#[derive(Debug, Clone)]
pub struct TradeItem {
    pub trade_items: Vec<Option<UserItem>>,
}

#[derive(Debug, Clone)]
pub struct TradeConfirm;

#[derive(Debug, Clone)]
pub struct TradeCancel {
    pub unlock: bool,
}

// ==================== 解析函数 ====================

#[cfg(feature = "client-parse")]
pub(crate) fn parse_trade_request(payload: &[u8]) -> Result<TradeRequest, String> {
    let mut cursor = Cursor::new(payload);
    let name = read_dotnet_string(&mut cursor)
        .map_err(|err| format!("failed to read trade request name: {err}"))?;
    Ok(TradeRequest { name })
}

#[cfg(feature = "client-parse")]
pub(crate) fn parse_trade_accept(payload: &[u8]) -> Result<TradeAccept, String> {
    let mut cursor = Cursor::new(payload);
    let name = read_dotnet_string(&mut cursor)
        .map_err(|err| format!("failed to read trade accept name: {err}"))?;
    Ok(TradeAccept { name })
}

#[cfg(feature = "client-parse")]
pub(crate) fn parse_trade_gold(payload: &[u8]) -> Result<TradeGold, String> {
    let mut cursor = Cursor::new(payload);
    let amount = cursor
        .read_u32::<LittleEndian>()
        .map_err(|err| format!("failed to read trade gold amount: {err}"))?;
    Ok(TradeGold { amount })
}

#[cfg(feature = "client-parse")]
pub(crate) fn parse_trade_item(payload: &[u8]) -> Result<TradeItem, String> {
    let mut cursor = Cursor::new(payload);
    let count = cursor
        .read_i32::<LittleEndian>()
        .map_err(|err| format!("failed to read trade item count: {err}"))?;

    let mut trade_items = Vec::with_capacity(count as usize);
    for _ in 0..count {
        let has_item = read_bool(&mut cursor)
            .map_err(|err| format!("failed to read trade item flag: {err}"))?;
        if has_item {
            let item = UserItem::read_default(&mut cursor)
                .map_err(|err| format!("failed to read trade item: {err}"))?;
            trade_items.push(Some(item));
        } else {
            trade_items.push(None);
        }
    }

    Ok(TradeItem { trade_items })
}

#[cfg(feature = "client-parse")]
pub(crate) fn parse_trade_confirm(_payload: &[u8]) -> Result<TradeConfirm, String> {
    Ok(TradeConfirm)
}

#[cfg(feature = "client-parse")]
pub(crate) fn parse_trade_cancel(payload: &[u8]) -> Result<TradeCancel, String> {
    let mut cursor = Cursor::new(payload);
    let unlock = read_bool(&mut cursor)
        .map_err(|err| format!("failed to read trade cancel unlock: {err}"))?;
    Ok(TradeCancel { unlock })
}
