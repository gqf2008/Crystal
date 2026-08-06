//! Group System Packets
//!
//! This module contains group/party-related packet definitions and parsers.

use super::super::base::Packet;
use crate::data::stats::SharedResult;
use crate::{
    binary::{read_dotnet_string, write_dotnet_string},
    enums::ServerPacketIds,
    map::Point,
};
use byteorder::{LittleEndian, ReadBytesExt, WriteBytesExt};
use std::io::{Read, Write};

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

/// 组队成员信息（ServerRust send_group_members_map wire：name + is_leader + online）
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GroupMember {
    pub name: String,
    pub is_leader: bool,
    pub online: bool,
}

/// Group members map info（ServerRust wire：count + 成员列表，非 C# 逐成员包）
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GroupMembersMap {
    pub members: Vec<GroupMember>,
}

impl Packet for GroupMembersMap {
    const OPCODE: i16 = ServerPacketIds::GroupMembersMap as i16;

    fn read_body<R: Read>(reader: &mut R) -> SharedResult<Self> {
        let count = reader.read_i32::<LittleEndian>()? as usize;
        let mut members = Vec::with_capacity(count);
        for _ in 0..count {
            let name = read_dotnet_string(reader)?;
            let is_leader = reader.read_u8()? != 0;
            let online = reader.read_u8()? != 0;
            members.push(GroupMember {
                name,
                is_leader,
                online,
            });
        }
        Ok(Self { members })
    }

    fn write_body<W: Write>(&self, writer: &mut W) -> SharedResult<()> {
        writer.write_i32::<LittleEndian>(self.members.len() as i32)?;
        for m in &self.members {
            write_dotnet_string(writer, &m.name)?;
            writer.write_u8(if m.is_leader { 1 } else { 0 })?;
            writer.write_u8(if m.online { 1 } else { 0 })?;
        }
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

/// Group invite from another player（ServerRust wire：inviter_name + inviter_id u64）
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GroupInvite {
    pub name: String,
    pub inviter_id: u64,
}

impl Packet for GroupInvite {
    const OPCODE: i16 = ServerPacketIds::GroupInvite as i16;

    fn read_body<R: Read>(reader: &mut R) -> SharedResult<Self> {
        let name = read_dotnet_string(reader)?;
        let inviter_id = reader.read_u64::<LittleEndian>()?;
        Ok(Self { name, inviter_id })
    }

    fn write_body<W: Write>(&self, writer: &mut W) -> SharedResult<()> {
        write_dotnet_string(writer, &self.name)?;
        writer.write_u64::<LittleEndian>(self.inviter_id)?;
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
