//! Movement Packets (Client → Server)

use std::io::{Read, Write};
use byteorder::{ReadBytesExt, WriteBytesExt};
use crate::enums::{ClientPacketIds, MirDirection};
use super::super::base::PacketMessage;
use crate::data::stats::SharedResult;

/// Client requests to turn in a direction
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Turn {
    pub direction: MirDirection,
}

impl PacketMessage for Turn {
    const OPCODE: i16 = ClientPacketIds::Turn as i16;

    fn read_body<R: Read>(reader: &mut R) -> SharedResult<Self> {
        let direction = MirDirection::try_from(reader.read_u8()?).unwrap_or(MirDirection::Up);
        Ok(Self { direction })
    }

    fn write_body<W: Write>(&self, writer: &mut W) -> SharedResult<()> {
        writer.write_u8(self.direction as u8)?;
        Ok(())
    }
}

/// Client requests to walk in a direction
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Walk {
    pub direction: MirDirection,
}

impl PacketMessage for Walk {
    const OPCODE: i16 = ClientPacketIds::Walk as i16;

    fn read_body<R: Read>(reader: &mut R) -> SharedResult<Self> {
        let direction = MirDirection::try_from(reader.read_u8()?).unwrap_or(MirDirection::Up);
        Ok(Self { direction })
    }

    fn write_body<W: Write>(&self, writer: &mut W) -> SharedResult<()> {
        writer.write_u8(self.direction as u8)?;
        Ok(())
    }
}

/// Client requests to run in a direction
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Run {
    pub direction: MirDirection,
}

impl PacketMessage for Run {
    const OPCODE: i16 = ClientPacketIds::Run as i16;

    fn read_body<R: Read>(reader: &mut R) -> SharedResult<Self> {
        let direction = MirDirection::try_from(reader.read_u8()?).unwrap_or(MirDirection::Up);
        Ok(Self { direction })
    }

    fn write_body<W: Write>(&self, writer: &mut W) -> SharedResult<()> {
        writer.write_u8(self.direction as u8)?;
        Ok(())
    }
}
