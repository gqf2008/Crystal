//! Guild System Packets (Client → Server)

use std::io::{Read, Write};
use byteorder::{LittleEndian, ReadBytesExt, WriteBytesExt};
use crate::binary::{read_dotnet_string, write_dotnet_string};
use crate::enums::ClientPacketIds;
use super::super::base::Packet;
use crate::data::stats::SharedResult;

/// Edit guild member (kick, promote, demote, etc.)
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EditGuildMember {
    pub change_type: u8,
    pub rank_index: u8,
    pub name: String,
    pub rank_name: String,
}

impl Packet for EditGuildMember {
    const OPCODE: i16 = ClientPacketIds::EditGuildMember as i16;

    fn read_body<R: Read>(reader: &mut R) -> SharedResult<Self> {
        let change_type = reader.read_u8()?;
        let rank_index = reader.read_u8()?;
        let name = read_dotnet_string(reader)?;
        let rank_name = read_dotnet_string(reader)?;
        
        Ok(Self { change_type, rank_index, name, rank_name })
    }

    fn write_body<W: Write>(&self, writer: &mut W) -> SharedResult<()> {
        writer.write_u8(self.change_type)?;
        writer.write_u8(self.rank_index)?;
        write_dotnet_string(writer, &self.name)?;
        write_dotnet_string(writer, &self.rank_name)?;
        Ok(())
    }
}

/// Edit guild notice (bulletin board)
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EditGuildNotice {
    pub notice_lines: Vec<String>,
}

impl Packet for EditGuildNotice {
    const OPCODE: i16 = ClientPacketIds::EditGuildNotice as i16;

    fn read_body<R: Read>(reader: &mut R) -> SharedResult<Self> {
        let line_count = reader.read_i32::<LittleEndian>()? as usize;
        let mut notice_lines = Vec::with_capacity(line_count);
        
        for _ in 0..line_count {
            notice_lines.push(read_dotnet_string(reader)?);
        }
        
        Ok(Self { notice_lines })
    }

    fn write_body<W: Write>(&self, writer: &mut W) -> SharedResult<()> {
        writer.write_i32::<LittleEndian>(self.notice_lines.len() as i32)?;
        
        for line in &self.notice_lines {
            write_dotnet_string(writer, line)?;
        }
        
        Ok(())
    }
}

/// Reply to guild invite
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct GuildInvite {
    pub accept_invite: bool,
}

impl Packet for GuildInvite {
    const OPCODE: i16 = ClientPacketIds::GuildInvite as i16;

    fn read_body<R: Read>(reader: &mut R) -> SharedResult<Self> {
        let accept_invite = reader.read_u8()? != 0;
        Ok(Self { accept_invite })
    }

    fn write_body<W: Write>(&self, writer: &mut W) -> SharedResult<()> {
        writer.write_u8(if self.accept_invite { 1 } else { 0 })?;
        Ok(())
    }
}

/// Request guild information
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct RequestGuildInfo {
    pub info_type: u8,
}

impl Packet for RequestGuildInfo {
    const OPCODE: i16 = ClientPacketIds::RequestGuildInfo as i16;

    fn read_body<R: Read>(reader: &mut R) -> SharedResult<Self> {
        let info_type = reader.read_u8()?;
        Ok(Self { info_type })
    }

    fn write_body<W: Write>(&self, writer: &mut W) -> SharedResult<()> {
        writer.write_u8(self.info_type)?;
        Ok(())
    }
}

/// Return guild name for display
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GuildNameReturn {
    pub name: String,
}

impl Packet for GuildNameReturn {
    const OPCODE: i16 = ClientPacketIds::GuildNameReturn as i16;

    fn read_body<R: Read>(reader: &mut R) -> SharedResult<Self> {
        let name = read_dotnet_string(reader)?;
        Ok(Self { name })
    }

    fn write_body<W: Write>(&self, writer: &mut W) -> SharedResult<()> {
        write_dotnet_string(writer, &self.name)?;
        Ok(())
    }
}

/// Declare war on another guild
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GuildWarReturn {
    pub guild_name: String,
}

impl Packet for GuildWarReturn {
    const OPCODE: i16 = ClientPacketIds::GuildWarReturn as i16;

    fn read_body<R: Read>(reader: &mut R) -> SharedResult<Self> {
        let guild_name = read_dotnet_string(reader)?;
        Ok(Self { guild_name })
    }

    fn write_body<W: Write>(&self, writer: &mut W) -> SharedResult<()> {
        write_dotnet_string(writer, &self.guild_name)?;
        Ok(())
    }
}

/// Change guild storage gold (deposit/withdraw)
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct GuildStorageGoldChange {
    pub change_type: u8, // 0 = deposit, 1 = withdraw
    pub amount: u32,
}

impl Packet for GuildStorageGoldChange {
    const OPCODE: i16 = ClientPacketIds::GuildStorageGoldChange as i16;

    fn read_body<R: Read>(reader: &mut R) -> SharedResult<Self> {
        let change_type = reader.read_u8()?;
        let amount = reader.read_u32::<LittleEndian>()?;
        Ok(Self { change_type, amount })
    }

    fn write_body<W: Write>(&self, writer: &mut W) -> SharedResult<()> {
        writer.write_u8(self.change_type)?;
        writer.write_u32::<LittleEndian>(self.amount)?;
        Ok(())
    }
}

/// Move item in guild storage
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct GuildStorageItemChange {
    pub change_type: u8, // 0 = store, 1 = retrieve, 2 = move
    pub from_slot: i32,
    pub to_slot: i32,
}

impl Packet for GuildStorageItemChange {
    const OPCODE: i16 = ClientPacketIds::GuildStorageItemChange as i16;

    fn read_body<R: Read>(reader: &mut R) -> SharedResult<Self> {
        let change_type = reader.read_u8()?;
        let from_slot = reader.read_i32::<LittleEndian>()?;
        let to_slot = reader.read_i32::<LittleEndian>()?;
        Ok(Self { change_type, from_slot, to_slot })
    }

    fn write_body<W: Write>(&self, writer: &mut W) -> SharedResult<()> {
        writer.write_u8(self.change_type)?;
        writer.write_i32::<LittleEndian>(self.from_slot)?;
        writer.write_i32::<LittleEndian>(self.to_slot)?;
        Ok(())
    }
}

/// Update guild buff status
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct GuildBuffUpdate {
    pub action: u8, // 0 = request list, 1 = enable buff, 2 = activate buff
    pub buff_id: i32,
}

impl Packet for GuildBuffUpdate {
    const OPCODE: i16 = ClientPacketIds::GuildBuffUpdate as i16;

    fn read_body<R: Read>(reader: &mut R) -> SharedResult<Self> {
        let action = reader.read_u8()?;
        let buff_id = reader.read_i32::<LittleEndian>()?;
        Ok(Self { action, buff_id })
    }

    fn write_body<W: Write>(&self, writer: &mut W) -> SharedResult<()> {
        writer.write_u8(self.action)?;
        writer.write_i32::<LittleEndian>(self.buff_id)?;
        Ok(())
    }
}

/// Request guild territory page
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct GuildTerritoryPage {
    pub page: i32,
}

impl Packet for GuildTerritoryPage {
    const OPCODE: i16 = ClientPacketIds::GuildTerritoryPage as i16;

    fn read_body<R: Read>(reader: &mut R) -> SharedResult<Self> {
        let page = reader.read_i32::<LittleEndian>()?;
        Ok(Self { page })
    }

    fn write_body<W: Write>(&self, writer: &mut W) -> SharedResult<()> {
        writer.write_i32::<LittleEndian>(self.page)?;
        Ok(())
    }
}

/// Purchase guild territory
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PurchaseGuildTerritory {
    pub owner: String,
}

impl Packet for PurchaseGuildTerritory {
    const OPCODE: i16 = ClientPacketIds::PurchaseGuildTerritory as i16;

    fn read_body<R: Read>(reader: &mut R) -> SharedResult<Self> {
        let owner = read_dotnet_string(reader)?;
        Ok(Self { owner })
    }

    fn write_body<W: Write>(&self, writer: &mut W) -> SharedResult<()> {
        write_dotnet_string(writer, &self.owner)?;
        Ok(())
    }
}
