//! Storage (Warehouse) Password Packets (Client → Server)
//!
//! PR #1169 — KR Mir2 Warehouse password feature.
//! These packets are sent from the client to unlock or manage the warehouse password.

use std::io::{Read, Write};
use crate::binary::{read_dotnet_string, write_dotnet_string};
use crate::enums::ClientPacketIds;
use super::super::base::Packet;
use crate::data::stats::SharedResult;

/// Client requests to unlock the warehouse with a password
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct UnlockStorage {
    pub password: String,
}

impl Packet for UnlockStorage {
    const OPCODE: i16 = ClientPacketIds::UnlockStorage as i16;

    fn read_body<R: Read>(reader: &mut R) -> SharedResult<Self> {
        Ok(Self {
            password: read_dotnet_string(reader)?,
        })
    }

    fn write_body<W: Write>(&self, writer: &mut W) -> SharedResult<()> {
        write_dotnet_string(writer, &self.password)?;
        Ok(())
    }
}

/// Client requests to set or change the warehouse password
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct SetStoragePassword {
    pub current_password: String,
    pub new_password: String,
}

impl Packet for SetStoragePassword {
    const OPCODE: i16 = ClientPacketIds::SetStoragePassword as i16;

    fn read_body<R: Read>(reader: &mut R) -> SharedResult<Self> {
        Ok(Self {
            current_password: read_dotnet_string(reader)?,
            new_password: read_dotnet_string(reader)?,
        })
    }

    fn write_body<W: Write>(&self, writer: &mut W) -> SharedResult<()> {
        write_dotnet_string(writer, &self.current_password)?;
        write_dotnet_string(writer, &self.new_password)?;
        Ok(())
    }
}

/// Client requests to remove the warehouse password
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct RemoveStoragePassword {
    pub current_password: String,
}

impl Packet for RemoveStoragePassword {
    const OPCODE: i16 = ClientPacketIds::RemoveStoragePassword as i16;

    fn read_body<R: Read>(reader: &mut R) -> SharedResult<Self> {
        Ok(Self {
            current_password: read_dotnet_string(reader)?,
        })
    }

    fn write_body<W: Write>(&self, writer: &mut W) -> SharedResult<()> {
        write_dotnet_string(writer, &self.current_password)?;
        Ok(())
    }
}
