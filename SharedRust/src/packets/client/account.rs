//! Account Management Packets (Client → Server)

use std::io::{Read, Write};
use byteorder::{LittleEndian, ReadBytesExt, WriteBytesExt};
use crate::binary::{read_dotnet_string, write_dotnet_string};
use crate::enums::ClientPacketIds;
use super::super::base::Packet;
use crate::data::stats::SharedResult;

#[derive(Debug, Clone, PartialEq, Eq)]
#[derive(Default)]
pub struct NewAccount {
    pub account_id: String,
    pub password: String,
    pub birth_date_binary: i64,
    pub user_name: String,
    pub secret_question: String,
    pub secret_answer: String,
    pub email_address: String,
}


impl Packet for NewAccount {
    const OPCODE: i16 = ClientPacketIds::NewAccount as i16;

    fn read_body<R: Read>(reader: &mut R) -> SharedResult<Self> {
        Ok(Self {
            account_id: read_dotnet_string(reader)?,
            password: read_dotnet_string(reader)?,
            birth_date_binary: reader.read_i64::<LittleEndian>()?,
            user_name: read_dotnet_string(reader)?,
            secret_question: read_dotnet_string(reader)?,
            secret_answer: read_dotnet_string(reader)?,
            email_address: read_dotnet_string(reader)?,
        })
    }

    fn write_body<W: Write>(&self, writer: &mut W) -> SharedResult<()> {
        write_dotnet_string(writer, &self.account_id)?;
        write_dotnet_string(writer, &self.password)?;
        writer.write_i64::<LittleEndian>(self.birth_date_binary)?;
        write_dotnet_string(writer, &self.user_name)?;
        write_dotnet_string(writer, &self.secret_question)?;
        write_dotnet_string(writer, &self.secret_answer)?;
        write_dotnet_string(writer, &self.email_address)?;
        Ok(())
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
#[derive(Default)]
pub struct ChangePassword {
    pub account_id: String,
    pub current_password: String,
    pub new_password: String,
}


impl Packet for ChangePassword {
    const OPCODE: i16 = ClientPacketIds::ChangePassword as i16;

    fn read_body<R: Read>(reader: &mut R) -> SharedResult<Self> {
        Ok(Self {
            account_id: read_dotnet_string(reader)?,
            current_password: read_dotnet_string(reader)?,
            new_password: read_dotnet_string(reader)?,
        })
    }

    fn write_body<W: Write>(&self, writer: &mut W) -> SharedResult<()> {
        write_dotnet_string(writer, &self.account_id)?;
        write_dotnet_string(writer, &self.current_password)?;
        write_dotnet_string(writer, &self.new_password)?;
        Ok(())
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
#[derive(Default)]
pub struct Login {
    pub account_id: String,
    pub password: String,
}


impl Packet for Login {
    const OPCODE: i16 = ClientPacketIds::Login as i16;

    fn read_body<R: Read>(reader: &mut R) -> SharedResult<Self> {
        Ok(Self {
            account_id: read_dotnet_string(reader)?,
            password: read_dotnet_string(reader)?,
        })
    }

    fn write_body<W: Write>(&self, writer: &mut W) -> SharedResult<()> {
        write_dotnet_string(writer, &self.account_id)?;
        write_dotnet_string(writer, &self.password)?;
        Ok(())
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct StartGame {
    pub character_index: i32,
}

impl Packet for StartGame {
    const OPCODE: i16 = ClientPacketIds::StartGame as i16;

    fn read_body<R: Read>(reader: &mut R) -> SharedResult<Self> {
        let character_index = reader.read_i32::<LittleEndian>()?;
        Ok(Self { character_index })
    }

    fn write_body<W: Write>(&self, writer: &mut W) -> SharedResult<()> {
        writer.write_i32::<LittleEndian>(self.character_index)?;
        Ok(())
    }
}
