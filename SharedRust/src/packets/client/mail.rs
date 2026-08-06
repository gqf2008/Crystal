//! Mail System Packets (Client → Server)

use super::super::base::Packet;
use crate::binary::{read_dotnet_string, write_dotnet_string};
use crate::data::stats::SharedResult;
use crate::enums::ClientPacketIds;
use byteorder::{LittleEndian, ReadBytesExt, WriteBytesExt};
use std::io::{Read, Write};

/// Send mail to another player
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SendMail {
    pub name: String,
    pub message: String,
    pub gold: u32,
    pub items_idx: [u64; 5],
    pub stamped: bool,
}

impl Packet for SendMail {
    const OPCODE: i16 = ClientPacketIds::SendMail as i16;

    fn read_body<R: Read>(reader: &mut R) -> SharedResult<Self> {
        let name = read_dotnet_string(reader)?;
        let message = read_dotnet_string(reader)?;
        let gold = reader.read_u32::<LittleEndian>()?;

        let mut items_idx = [0u64; 5];
        for item in &mut items_idx {
            *item = reader.read_u64::<LittleEndian>()?;
        }

        let stamped = reader.read_u8()? != 0;

        Ok(Self {
            name,
            message,
            gold,
            items_idx,
            stamped,
        })
    }

    fn write_body<W: Write>(&self, writer: &mut W) -> SharedResult<()> {
        write_dotnet_string(writer, &self.name)?;
        write_dotnet_string(writer, &self.message)?;
        writer.write_u32::<LittleEndian>(self.gold)?;

        for &item_idx in &self.items_idx {
            writer.write_u64::<LittleEndian>(item_idx)?;
        }

        writer.write_u8(if self.stamped { 1 } else { 0 })?;
        Ok(())
    }
}

/// Read mail
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ReadMail {
    pub mail_id: u64,
}

impl Packet for ReadMail {
    const OPCODE: i16 = ClientPacketIds::ReadMail as i16;

    fn read_body<R: Read>(reader: &mut R) -> SharedResult<Self> {
        let mail_id = reader.read_u64::<LittleEndian>()?;
        Ok(Self { mail_id })
    }

    fn write_body<W: Write>(&self, writer: &mut W) -> SharedResult<()> {
        writer.write_u64::<LittleEndian>(self.mail_id)?;
        Ok(())
    }
}

/// Collect parcel items from mail
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct CollectParcel {
    pub mail_id: u64,
}

impl Packet for CollectParcel {
    const OPCODE: i16 = ClientPacketIds::CollectParcel as i16;

    fn read_body<R: Read>(reader: &mut R) -> SharedResult<Self> {
        let mail_id = reader.read_u64::<LittleEndian>()?;
        Ok(Self { mail_id })
    }

    fn write_body<W: Write>(&self, writer: &mut W) -> SharedResult<()> {
        writer.write_u64::<LittleEndian>(self.mail_id)?;
        Ok(())
    }
}

/// Delete mail
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct DeleteMail {
    pub mail_id: u64,
}

impl Packet for DeleteMail {
    const OPCODE: i16 = ClientPacketIds::DeleteMail as i16;

    fn read_body<R: Read>(reader: &mut R) -> SharedResult<Self> {
        let mail_id = reader.read_u64::<LittleEndian>()?;
        Ok(Self { mail_id })
    }

    fn write_body<W: Write>(&self, writer: &mut W) -> SharedResult<()> {
        writer.write_u64::<LittleEndian>(self.mail_id)?;
        Ok(())
    }
}

/// Lock/unlock mail
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct LockMail {
    pub mail_id: u64,
    pub lock: bool,
}

impl Packet for LockMail {
    const OPCODE: i16 = ClientPacketIds::LockMail as i16;

    fn read_body<R: Read>(reader: &mut R) -> SharedResult<Self> {
        let mail_id = reader.read_u64::<LittleEndian>()?;
        let lock = reader.read_u8()? != 0;
        Ok(Self { mail_id, lock })
    }

    fn write_body<W: Write>(&self, writer: &mut W) -> SharedResult<()> {
        writer.write_u64::<LittleEndian>(self.mail_id)?;
        writer.write_u8(if self.lock { 1 } else { 0 })?;
        Ok(())
    }
}

/// Lock/unlock item for mailing
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct MailLockedItem {
    pub unique_id: u64,
    pub locked: bool,
}

impl Packet for MailLockedItem {
    const OPCODE: i16 = ClientPacketIds::MailLockedItem as i16;

    fn read_body<R: Read>(reader: &mut R) -> SharedResult<Self> {
        let unique_id = reader.read_u64::<LittleEndian>()?;
        let locked = reader.read_u8()? != 0;
        Ok(Self { unique_id, locked })
    }

    fn write_body<W: Write>(&self, writer: &mut W) -> SharedResult<()> {
        writer.write_u64::<LittleEndian>(self.unique_id)?;
        writer.write_u8(if self.locked { 1 } else { 0 })?;
        Ok(())
    }
}

/// Get mail sending cost
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct MailCost {
    pub gold: u32,
    pub items_idx: [u64; 5],
    pub stamped: bool,
}

impl Packet for MailCost {
    const OPCODE: i16 = ClientPacketIds::MailCost as i16;

    fn read_body<R: Read>(reader: &mut R) -> SharedResult<Self> {
        let gold = reader.read_u32::<LittleEndian>()?;

        let mut items_idx = [0u64; 5];
        for item in &mut items_idx {
            *item = reader.read_u64::<LittleEndian>()?;
        }

        let stamped = reader.read_u8()? != 0;

        Ok(Self {
            gold,
            items_idx,
            stamped,
        })
    }

    fn write_body<W: Write>(&self, writer: &mut W) -> SharedResult<()> {
        writer.write_u32::<LittleEndian>(self.gold)?;

        for &item_idx in &self.items_idx {
            writer.write_u64::<LittleEndian>(item_idx)?;
        }

        writer.write_u8(if self.stamped { 1 } else { 0 })?;
        Ok(())
    }
}
