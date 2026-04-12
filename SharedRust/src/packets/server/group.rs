//! Group System Packets
//!
//! This module contains group/party-related packet definitions and parsers.

use std::io::{Read, Write};
use byteorder::{LittleEndian, ReadBytesExt, WriteBytesExt};
use crate::{
    map::Point,
    enums::ServerPacketIds,
    binary::{read_dotnet_string, write_dotnet_string},
};
use super::super::base::Packet;
use crate::data::stats::SharedResult;

// ============================================================================
// Packet Structures & PacketMessage Implementations
// ============================================================================

/// Switch group mode
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SwitchGroup {
    pub allow_group: bool,
}

impl Packet for SwitchGroup {
    const OPCODE: i16 = ServerPacketIds::SwitchGroup as i16;

    fn read_body<R: Read>(reader: &mut R) -> SharedResult<Self> {
        let allow_group = reader.read_u8()? != 0;
        Ok(Self { allow_group })
    }

    fn write_body<W: Write>(&self, writer: &mut W) -> SharedResult<()> {
        writer.write_u8(if self.allow_group { 1 } else { 0 })?;
        Ok(())
    }
}

/// Group members map info
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GroupMembersMap {
    pub player_name: String,
    pub player_map: String,
}

impl Packet for GroupMembersMap {
    const OPCODE: i16 = ServerPacketIds::GroupMembersMap as i16;

    fn read_body<R: Read>(reader: &mut R) -> SharedResult<Self> {
        let player_name = read_dotnet_string(reader)?;
        let player_map = read_dotnet_string(reader)?;
        Ok(Self { player_name, player_map })
    }

    fn write_body<W: Write>(&self, writer: &mut W) -> SharedResult<()> {
        write_dotnet_string(writer, &self.player_name)?;
        write_dotnet_string(writer, &self.player_map)?;
        Ok(())
    }
}

/// Send member location
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SendMemberLocation {
    pub member_name: String,
    pub location: Point,
}

impl Packet for SendMemberLocation {
    const OPCODE: i16 = ServerPacketIds::SendMemberLocation as i16;

    fn read_body<R: Read>(reader: &mut R) -> SharedResult<Self> {
        let member_name = read_dotnet_string(reader)?;
        let x = reader.read_i32::<LittleEndian>()?;
        let y = reader.read_i32::<LittleEndian>()?;
        let location = Point { x, y };
        
        Ok(Self {
            member_name,
            location,
        })
    }

    fn write_body<W: Write>(&self, writer: &mut W) -> SharedResult<()> {
        write_dotnet_string(writer, &self.member_name)?;
        writer.write_i32::<LittleEndian>(self.location.x)?;
        writer.write_i32::<LittleEndian>(self.location.y)?;
        Ok(())
    }
}

/// Delete group - player has left the group
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct DeleteGroup;

impl Packet for DeleteGroup {
    const OPCODE: i16 = ServerPacketIds::DeleteGroup as i16;

    fn read_body<R: Read>(_reader: &mut R) -> SharedResult<Self> {
        Ok(Self)
    }

    fn write_body<W: Write>(&self, _writer: &mut W) -> SharedResult<()> {
        Ok(())
    }
}

/// Delete member from group
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DeleteMember {
    pub name: String,
}

impl Packet for DeleteMember {
    const OPCODE: i16 = ServerPacketIds::DeleteMember as i16;

    fn read_body<R: Read>(reader: &mut R) -> SharedResult<Self> {
        let name = read_dotnet_string(reader)?;
        Ok(Self { name })
    }

    fn write_body<W: Write>(&self, writer: &mut W) -> SharedResult<()> {
        write_dotnet_string(writer, &self.name)?;
        Ok(())
    }
}

/// Group invite from another player
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GroupInvite {
    pub name: String,
}

impl Packet for GroupInvite {
    const OPCODE: i16 = ServerPacketIds::GroupInvite as i16;

    fn read_body<R: Read>(reader: &mut R) -> SharedResult<Self> {
        let name = read_dotnet_string(reader)?;
        Ok(Self { name })
    }

    fn write_body<W: Write>(&self, writer: &mut W) -> SharedResult<()> {
        write_dotnet_string(writer, &self.name)?;
        Ok(())
    }
}

/// Add member to group
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AddMember {
    pub name: String,
}

impl Packet for AddMember {
    const OPCODE: i16 = ServerPacketIds::AddMember as i16;

    fn read_body<R: Read>(reader: &mut R) -> SharedResult<Self> {
        let name = read_dotnet_string(reader)?;
        Ok(Self { name })
    }

    fn write_body<W: Write>(&self, writer: &mut W) -> SharedResult<()> {
        write_dotnet_string(writer, &self.name)?;
        Ok(())
    }
}
