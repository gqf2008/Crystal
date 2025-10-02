//! Guild System Packets
//!
//! This module contains all guild-related packet definitions and parsers.

use std::io::{Read, Write};
use byteorder::{LittleEndian, ReadBytesExt, WriteBytesExt};
use crate::{
    data::client_data::{GuildRank, GuildStorageItem},
    enums::ServerPacketIds,
    binary::{read_dotnet_string, write_dotnet_string},
};
use super::super::base::Packet;
use crate::data::stats::SharedResult;

// ============================================================================
// Packet Structures & PacketMessage Implementations
// ============================================================================

/// Guild storage list
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GuildStorageList {
    pub items: Vec<Option<GuildStorageItem>>,
}

impl Packet for GuildStorageList {
    const OPCODE: i16 = ServerPacketIds::GuildStorageList as i16;

    fn read_body<R: Read>(reader: &mut R) -> SharedResult<Self> {
        let count = reader.read_i32::<LittleEndian>()? as usize;
        let mut items = Vec::with_capacity(count);
        
        for _ in 0..count {
            let has_item = reader.read_u8()? != 0;
            if has_item {
                let item = GuildStorageItem::read_from(reader)?;
                items.push(Some(item));
            } else {
                items.push(None);
            }
        }
        
        Ok(Self { items })
    }

    fn write_body<W: Write>(&self, writer: &mut W) -> SharedResult<()> {
        writer.write_i32::<LittleEndian>(self.items.len() as i32)?;
        
        for item_opt in &self.items {
            if let Some(item) = item_opt {
                writer.write_u8(1)?;
                item.write_to(writer)?;
            } else {
                writer.write_u8(0)?;
            }
        }
        
        Ok(())
    }
}

/// Guild notice changed
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GuildNoticeChange {
    pub notice: Vec<String>,
}

impl Packet for GuildNoticeChange {
    const OPCODE: i16 = ServerPacketIds::GuildNoticeChange as i16;

    fn read_body<R: Read>(reader: &mut R) -> SharedResult<Self> {
        let count = reader.read_i32::<LittleEndian>()? as usize;
        let mut notice = Vec::with_capacity(count);
        
        for _ in 0..count {
            notice.push(read_dotnet_string(reader)?);
        }
        
        Ok(Self { notice })
    }

    fn write_body<W: Write>(&self, writer: &mut W) -> SharedResult<()> {
        writer.write_i32::<LittleEndian>(self.notice.len() as i32)?;
        
        for line in &self.notice {
            write_dotnet_string(writer, line)?;
        }
        
        Ok(())
    }
}

/// Guild member status changed
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GuildMemberChange {
    pub name: String,
    pub rank_index: u8,
    pub status: u8,
    pub ranks: Vec<GuildRank>,
}

impl Packet for GuildMemberChange {
    const OPCODE: i16 = ServerPacketIds::GuildMemberChange as i16;

    fn read_body<R: Read>(reader: &mut R) -> SharedResult<Self> {
        let name = read_dotnet_string(reader)?;
        let rank_index = reader.read_u8()?;
        let status = reader.read_u8()?;
        
        let mut ranks = Vec::new();
        // Only read ranks if status > 5 (based on C# code)
        if status > 5 {
            let rank_count = reader.read_i32::<LittleEndian>()? as usize;
            for _ in 0..rank_count {
                ranks.push(GuildRank::read_from(reader, false)?);
            }
        }
        
        Ok(Self {
            name,
            rank_index,
            status,
            ranks,
        })
    }

    fn write_body<W: Write>(&self, writer: &mut W) -> SharedResult<()> {
        write_dotnet_string(writer, &self.name)?;
        writer.write_u8(self.rank_index)?;
        writer.write_u8(self.status)?;
        
        if self.status > 5 {
            writer.write_i32::<LittleEndian>(self.ranks.len() as i32)?;
            // Note: GuildRank doesn't have write_to method yet, need to implement
            // For now, this is a placeholder that will need proper implementation
            for rank in &self.ranks {
                write_dotnet_string(writer, &rank.name)?;
                writer.write_u8(rank.options.bits())?;
                writer.write_i32::<LittleEndian>(rank.index)?;
                writer.write_i32::<LittleEndian>(rank.members.len() as i32)?;
                for member in &rank.members {
                    write_dotnet_string(writer, &member.name)?;
                    write_dotnet_string(writer, &member.rank_name)?;
                    writer.write_u8(member.rank_index)?;
                    writer.write_u8(if member.online { 1 } else { 0 })?;
                    writer.write_i64::<LittleEndian>(member.last_login)?;
                }
            }
        }
        
        Ok(())
    }
}
