//! Guild System Packets
//!
//! This module contains all guild-related packet definitions and parsers.

use crate::{data::client_data::GuildRank, data::item::UserItem};

#[cfg(feature = "client-parse")]
use std::io::Cursor;
#[cfg(feature = "client-parse")]
use byteorder::{LittleEndian, ReadBytesExt};
#[cfg(feature = "client-parse")]
use crate::binary::read_dotnet_string;

// ============================================================================
// Packet Structures
// ============================================================================

/// Guild storage list
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GuildStorageList {
    pub items: Vec<Option<UserItem>>,
}

/// Guild notice changed
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GuildNoticeChange {
    pub notice: Vec<String>,
    pub update: bool,
}

/// Guild member status changed
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GuildMemberChange {
    pub name: String,
    pub status: u8,
    pub ranks: Vec<GuildRank>,
}

// ============================================================================
// Parser Functions
// ============================================================================

#[cfg(feature = "client-parse")]
pub(crate) fn parse_guild_storage_list(payload: &[u8]) -> Result<GuildStorageList, String> {
    let mut cursor = Cursor::new(payload);
    let count = cursor
        .read_i32::<LittleEndian>()
        .map_err(|e| format!("Failed to read item count: {}", e))?;
    let mut items = Vec::new();
    for _ in 0..count {
        let has_item = cursor
            .read_u8()
            .map_err(|e| format!("Failed to read has_item flag: {}", e))?
            != 0;
        if has_item {
            let item = UserItem::read_from(&mut cursor, i32::MAX, i32::MAX)?;
            items.push(Some(item));
        } else {
            items.push(None);
        }
    }
    Ok(GuildStorageList { items })
}

#[cfg(feature = "client-parse")]
pub(crate) fn parse_guild_notice_change(payload: &[u8]) -> Result<GuildNoticeChange, String> {
    let mut cursor = Cursor::new(payload);
    let count = cursor
        .read_i32::<LittleEndian>()
        .map_err(|e| format!("Failed to read notice line count: {}", e))?;
    let mut notice = Vec::new();
    for _ in 0..count {
        notice.push(read_dotnet_string(&mut cursor)?);
    }
    let update = cursor
        .read_u8()
        .map_err(|e| format!("Failed to read update: {}", e))?
        != 0;
    Ok(GuildNoticeChange { notice, update })
}

#[cfg(feature = "client-parse")]
pub(crate) fn parse_guild_member_change(payload: &[u8]) -> Result<GuildMemberChange, String> {
    let mut cursor = Cursor::new(payload);
    let name = read_dotnet_string(&mut cursor)?;
    let status = cursor
        .read_u8()
        .map_err(|e| format!("Failed to read status: {}", e))?;

    let mut ranks = Vec::new();
    // Only read ranks if status indicates member addition/update
    if status == 0 || status == 1 {
        let rank_count = cursor
            .read_i32::<LittleEndian>()
            .map_err(|e| format!("Failed to read rank count: {}", e))?;
        for _ in 0..rank_count {
            ranks.push(GuildRank::read_from(&mut cursor, false)?);
        }
    }

    Ok(GuildMemberChange {
        name,
        status,
        ranks,
    })
}
