//! Group/Party System Packets (Client → Server)

use super::super::base::PacketMessage;
use crate::binary::{read_dotnet_string, write_dotnet_string};
use crate::data::stats::SharedResult;
use crate::enums::ClientPacketIds;
use byteorder::ReadBytesExt;
use std::io::{Read, Write};

/// Switch group mode (allow/disallow grouping)
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SwitchGroup {
    pub allow_group: bool,
}

impl PacketMessage for SwitchGroup {
    const OPCODE: i16 = ClientPacketIds::SwitchGroup as i16;

    fn read_body<R: Read>(reader: &mut R) -> SharedResult<Self> {
        let allow_group = reader.read_u8()? != 0;
        Ok(Self { allow_group })
    }

    fn write_body<W: Write>(&self, writer: &mut W) -> SharedResult<()> {
        writer.write_all(&[if self.allow_group { 1 } else { 0 }])?;
        Ok(())
    }
}

/// Add member to group
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AddMember {
    pub name: String,
}

impl PacketMessage for AddMember {
    const OPCODE: i16 = ClientPacketIds::AddMember as i16;

    fn read_body<R: Read>(reader: &mut R) -> SharedResult<Self> {
        let name = read_dotnet_string(reader)?;
        Ok(Self { name })
    }

    fn write_body<W: Write>(&self, writer: &mut W) -> SharedResult<()> {
        write_dotnet_string(writer, &self.name)?;
        Ok(())
    }
}

/// Delete member from group
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DellMember {
    pub name: String,
}

impl PacketMessage for DellMember {
    const OPCODE: i16 = ClientPacketIds::DellMember as i16;

    fn read_body<R: Read>(reader: &mut R) -> SharedResult<Self> {
        let name = read_dotnet_string(reader)?;
        Ok(Self { name })
    }

    fn write_body<W: Write>(&self, writer: &mut W) -> SharedResult<()> {
        write_dotnet_string(writer, &self.name)?;
        Ok(())
    }
}

/// Reply to group invite
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct GroupInvite {
    pub accept_invite: bool,
}

impl PacketMessage for GroupInvite {
    const OPCODE: i16 = ClientPacketIds::GroupInvite as i16;

    fn read_body<R: Read>(reader: &mut R) -> SharedResult<Self> {
        let accept_invite = reader.read_u8()? != 0;
        Ok(Self { accept_invite })
    }

    fn write_body<W: Write>(&self, writer: &mut W) -> SharedResult<()> {
        writer.write_all(&[if self.accept_invite { 1 } else { 0 }])?;
        Ok(())
    }
}
