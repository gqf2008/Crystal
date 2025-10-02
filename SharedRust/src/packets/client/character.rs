//! Character Management Packets (Client → Server)

use std::io::{Read, Write};
use byteorder::{LittleEndian, ReadBytesExt, WriteBytesExt};
use crate::binary::{read_dotnet_string, write_dotnet_string};
use crate::enums::{ClientPacketIds, MirClass, MirGender};
use super::super::base::PacketMessage;
use crate::data::stats::SharedResult;

/// Client requests to create a new character
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NewCharacter {
    pub name: String,
    pub class: MirClass,
    pub gender: MirGender,
}

impl PacketMessage for NewCharacter {
    const OPCODE: i16 = ClientPacketIds::NewCharacter as i16;

    fn read_body<R: Read>(reader: &mut R) -> SharedResult<Self> {
        Ok(Self {
            name: read_dotnet_string(reader)?,
            class: MirClass::try_from(reader.read_u8()?).unwrap_or(MirClass::Warrior),
            gender: MirGender::try_from(reader.read_u8()?).unwrap_or(MirGender::Male),
        })
    }

    fn write_body<W: Write>(&self, writer: &mut W) -> SharedResult<()> {
        write_dotnet_string(writer, &self.name)?;
        writer.write_u8(self.class as u8)?;
        writer.write_u8(self.gender as u8)?;
        Ok(())
    }
}

/// Client requests to delete a character
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DeleteCharacter {
    pub character_index: i32,
}

impl PacketMessage for DeleteCharacter {
    const OPCODE: i16 = ClientPacketIds::DeleteCharacter as i16;

    fn read_body<R: Read>(reader: &mut R) -> SharedResult<Self> {
        let character_index = reader.read_i32::<LittleEndian>()?;
        Ok(Self { character_index })
    }

    fn write_body<W: Write>(&self, writer: &mut W) -> SharedResult<()> {
        writer.write_i32::<LittleEndian>(self.character_index)?;
        Ok(())
    }
}

/// Client requests to log out
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct LogOut;

impl PacketMessage for LogOut {
    const OPCODE: i16 = ClientPacketIds::LogOut as i16;

    fn read_body<R: Read>(_: &mut R) -> SharedResult<Self> {
        Ok(Self)
    }

    fn write_body<W: Write>(&self, _: &mut W) -> SharedResult<()> {
        Ok(())
    }
}
