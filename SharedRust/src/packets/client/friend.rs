//! Friend System Packets (Client → Server)

use std::io::{Read, Write};
use byteorder::{LittleEndian, ReadBytesExt, WriteBytesExt};
use crate::binary::{read_dotnet_string, write_dotnet_string};
use crate::enums::ClientPacketIds;
use super::super::base::PacketMessage;
use crate::data::stats::SharedResult;

/// Add friend or blocked user
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AddFriend {
    pub name: String,
    pub blocked: bool,
}

impl PacketMessage for AddFriend {
    const OPCODE: i16 = ClientPacketIds::AddFriend as i16;

    fn read_body<R: Read>(reader: &mut R) -> SharedResult<Self> {
        let name = read_dotnet_string(reader)?;
        let blocked = reader.read_u8()? != 0;
        Ok(Self { name, blocked })
    }

    fn write_body<W: Write>(&self, writer: &mut W) -> SharedResult<()> {
        write_dotnet_string(writer, &self.name)?;
        writer.write_u8(if self.blocked { 1 } else { 0 })?;
        Ok(())
    }
}

/// Remove friend
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct RemoveFriend {
    pub character_index: i32,
}

impl PacketMessage for RemoveFriend {
    const OPCODE: i16 = ClientPacketIds::RemoveFriend as i16;

    fn read_body<R: Read>(reader: &mut R) -> SharedResult<Self> {
        let character_index = reader.read_i32::<LittleEndian>()?;
        Ok(Self { character_index })
    }

    fn write_body<W: Write>(&self, writer: &mut W) -> SharedResult<()> {
        writer.write_i32::<LittleEndian>(self.character_index)?;
        Ok(())
    }
}

/// Request refresh friend list
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct RefreshFriends;

impl PacketMessage for RefreshFriends {
    const OPCODE: i16 = ClientPacketIds::RefreshFriends as i16;

    fn read_body<R: Read>(_reader: &mut R) -> SharedResult<Self> {
        Ok(Self)
    }

    fn write_body<W: Write>(&self, _writer: &mut W) -> SharedResult<()> {
        Ok(())
    }
}

/// Add memo to friend
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AddMemo {
    pub character_index: i32,
    pub memo: String,
}

impl PacketMessage for AddMemo {
    const OPCODE: i16 = ClientPacketIds::AddMemo as i16;

    fn read_body<R: Read>(reader: &mut R) -> SharedResult<Self> {
        let character_index = reader.read_i32::<LittleEndian>()?;
        let memo = read_dotnet_string(reader)?;
        Ok(Self { character_index, memo })
    }

    fn write_body<W: Write>(&self, writer: &mut W) -> SharedResult<()> {
        writer.write_i32::<LittleEndian>(self.character_index)?;
        write_dotnet_string(writer, &self.memo)?;
        Ok(())
    }
}
