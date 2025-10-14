//! Character Management Packets (Client → Server)

use std::io::{Read, Write};
use byteorder::{LittleEndian, ReadBytesExt, WriteBytesExt};
use crate::binary::{read_dotnet_string, write_dotnet_string};
use crate::enums::{ClientPacketIds, MirClass, MirGender};
use super::super::base::Packet;
use crate::data::stats::SharedResult;

/// Client requests to create a new character
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NewCharacter {
    pub name: String,
    pub gender: MirGender,  // C# 顺序: Name, Gender, Class
    pub class: MirClass,
}

impl Packet for NewCharacter {
    const OPCODE: i16 = ClientPacketIds::NewCharacter as i16;

    fn read_body<R: Read>(reader: &mut R) -> SharedResult<Self> {
        Ok(Self {
            name: read_dotnet_string(reader)?,
            gender: MirGender::try_from(reader.read_u8()?).unwrap_or(MirGender::Male),  // 先读gender
            class: MirClass::try_from(reader.read_u8()?).unwrap_or(MirClass::Warrior),   // 再读class
        })
    }

    fn write_body<W: Write>(&self, writer: &mut W) -> SharedResult<()> {
        write_dotnet_string(writer, &self.name)?;
        writer.write_u8(self.gender as u8)?;  // 先写gender
        writer.write_u8(self.class as u8)?;   // 再写class
        Ok(())
    }
}

/// Client requests to delete a character
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DeleteCharacter {
    pub character_index: i32,
}

impl Packet for DeleteCharacter {
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

impl Packet for LogOut {
    const OPCODE: i16 = ClientPacketIds::LogOut as i16;

    fn read_body<R: Read>(_: &mut R) -> SharedResult<Self> {
        Ok(Self)
    }

    fn write_body<W: Write>(&self, _: &mut W) -> SharedResult<()> {
        Ok(())
    }
}
