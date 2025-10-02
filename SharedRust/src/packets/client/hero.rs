//! Hero System Packets (Client → Server)

use std::io::{Read, Write};
use byteorder::{LittleEndian, ReadBytesExt, WriteBytesExt};
use crate::binary::{read_dotnet_string, write_dotnet_string};
use crate::enums::{ClientPacketIds, MirGender, MirClass, HeroBehaviour};
use crate::data::stats::SharedResult;
use super::super::base::PacketMessage;

/// Create new hero
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NewHero {
    pub name: String,
    pub gender: MirGender,
    pub class: MirClass,
}

impl PacketMessage for NewHero {
    const OPCODE: i16 = ClientPacketIds::NewHero as i16;

    fn read_body<R: Read>(reader: &mut R) -> SharedResult<Self> {
        let name = read_dotnet_string(reader)?;
        let gender = MirGender::try_from(reader.read_u8()?)?;
        let class = MirClass::try_from(reader.read_u8()?)?;
        Ok(Self { name, gender, class })
    }

    fn write_body<W: Write>(&self, writer: &mut W) -> SharedResult<()> {
        write_dotnet_string(writer, &self.name)?;
        writer.write_u8(self.gender as u8)?;
        writer.write_u8(self.class as u8)?;
        Ok(())
    }
}

/// Set hero auto-potion value (HP/MP threshold)
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SetAutoPotValue {
    pub stat: u8, // 0 = HP, 1 = MP
    pub value: u8, // Percentage threshold
}

impl PacketMessage for SetAutoPotValue {
    const OPCODE: i16 = ClientPacketIds::SetAutoPotValue as i16;

    fn read_body<R: Read>(reader: &mut R) -> SharedResult<Self> {
        let stat = reader.read_u8()?;
        let value = reader.read_u8()?;
        Ok(Self { stat, value })
    }

    fn write_body<W: Write>(&self, writer: &mut W) -> SharedResult<()> {
        writer.write_u8(self.stat)?;
        writer.write_u8(self.value)?;
        Ok(())
    }
}

/// Set hero auto-potion item
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SetAutoPotItem {
    pub grid: u32,
    pub item_index: u64,
}

impl PacketMessage for SetAutoPotItem {
    const OPCODE: i16 = ClientPacketIds::SetAutoPotItem as i16;

    fn read_body<R: Read>(reader: &mut R) -> SharedResult<Self> {
        let grid = reader.read_u32::<LittleEndian>()?;
        let item_index = reader.read_u64::<LittleEndian>()?;
        Ok(Self { grid, item_index })
    }

    fn write_body<W: Write>(&self, writer: &mut W) -> SharedResult<()> {
        writer.write_u32::<LittleEndian>(self.grid)?;
        writer.write_u64::<LittleEndian>(self.item_index)?;
        Ok(())
    }
}

/// Set hero behaviour
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SetHeroBehaviour {
    pub behaviour: HeroBehaviour,
}

impl PacketMessage for SetHeroBehaviour {
    const OPCODE: i16 = ClientPacketIds::SetHeroBehaviour as i16;

    fn read_body<R: Read>(reader: &mut R) -> SharedResult<Self> {
        let behaviour = HeroBehaviour::try_from(reader.read_u8()?)?;
        Ok(Self { behaviour })
    }

    fn write_body<W: Write>(&self, writer: &mut W) -> SharedResult<()> {
        writer.write_u8(self.behaviour as u8)?;
        Ok(())
    }
}

/// Change active hero
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ChangeHero {
    pub list_index: u8, // Which hero slot (0-2)
}

impl PacketMessage for ChangeHero {
    const OPCODE: i16 = ClientPacketIds::ChangeHero as i16;

    fn read_body<R: Read>(reader: &mut R) -> SharedResult<Self> {
        let list_index = reader.read_u8()?;
        Ok(Self { list_index })
    }

    fn write_body<W: Write>(&self, writer: &mut W) -> SharedResult<()> {
        writer.write_u8(self.list_index)?;
        Ok(())
    }
}
