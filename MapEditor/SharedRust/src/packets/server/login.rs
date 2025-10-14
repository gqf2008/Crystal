//! Login and Authentication Packets
//!
//! Packets related to login, account creation, and authentication.

use std::io::{Read, Write};
use byteorder::{LittleEndian, ReadBytesExt, WriteBytesExt};
use crate::SelectInfo;
use crate::data::stats::SharedResult;
use crate::binary::{read_dotnet_string, write_dotnet_string};
use crate::enums::ServerPacketIds;
use super::super::base::Packet;

/// NewAccount packet - response to account creation request
#[derive(Debug, Clone)]
pub struct NewAccount {
    pub result: u8,
    // 0: Disabled
    // 1: Bad AccountID
    // 2: Bad Password
    // 3: Bad Email
    // 4: Bad Name
    // 5: Bad Question
    // 6: Bad Answer
    // 7: Account Exists
    // 8: Success
}

impl Packet for NewAccount {
    const OPCODE: i16 = ServerPacketIds::NewAccount as i16;

    fn read_body<R: Read>(reader: &mut R) -> SharedResult<Self> {
        let result = reader.read_u8()?;
        Ok(Self { result })
    }

    fn write_body<W: Write>(&self, writer: &mut W) -> SharedResult<()> {
        writer.write_u8(self.result)?;
        Ok(())
    }
}

/// ChangePassword packet - response to password change request
#[derive(Debug, Clone)]
pub struct ChangePassword {
    pub result: u8,
    // 0: Disabled
    // 1: Bad AccountID
    // 2: Bad Current Password
    // 3: Bad New Password
    // 4: Account Not Exist
    // 5: Wrong Password
    // 6: Success
}

impl Packet for ChangePassword {
    const OPCODE: i16 = ServerPacketIds::ChangePassword as i16;

    fn read_body<R: Read>(reader: &mut R) -> SharedResult<Self> {
        let result = reader.read_u8()?;
        Ok(Self { result })
    }

    fn write_body<W: Write>(&self, writer: &mut W) -> SharedResult<()> {
        writer.write_u8(self.result)?;
        Ok(())
    }
}

/// ChangePasswordBanned packet - notification of password change ban
#[derive(Debug, Clone)]
pub struct ChangePasswordBanned {
    pub reason: String,
    pub expiry_date: i64, // .NET DateTime ticks
}

impl Packet for ChangePasswordBanned {
    const OPCODE: i16 = ServerPacketIds::ChangePasswordBanned as i16;

    fn read_body<R: Read>(reader: &mut R) -> SharedResult<Self> {
        let reason = read_dotnet_string(reader)?;
        let expiry_date = reader.read_i64::<LittleEndian>()?;
        Ok(Self { reason, expiry_date })
    }

    fn write_body<W: Write>(&self, writer: &mut W) -> SharedResult<()> {
        write_dotnet_string(writer, &self.reason)?;
        writer.write_i64::<LittleEndian>(self.expiry_date)?;
        Ok(())
    }
}

/// Login packet - response to login request
#[derive(Debug, Clone)]
pub struct Login {
    pub result: u8,
    // 0: Disabled
    // 1: Bad AccountID
    // 2: Bad Password
    // 3: Account Not Exist
    // 4: Wrong Password
}

impl Packet for Login {
    const OPCODE: i16 = ServerPacketIds::Login as i16;

    fn read_body<R: Read>(reader: &mut R) -> SharedResult<Self> {
        let result = reader.read_u8()?;
        Ok(Self { result })
    }

    fn write_body<W: Write>(&self, writer: &mut W) -> SharedResult<()> {
        writer.write_u8(self.result)?;
        Ok(())
    }
}

/// LoginBanned packet - notification of login ban
#[derive(Debug, Clone)]
pub struct LoginBanned {
    pub reason: String,
    pub expiry_date: i64, // .NET DateTime ticks
}

impl Packet for LoginBanned {
    const OPCODE: i16 = ServerPacketIds::LoginBanned as i16;

    fn read_body<R: Read>(reader: &mut R) -> SharedResult<Self> {
        let reason = read_dotnet_string(reader)?;
        let expiry_date = reader.read_i64::<LittleEndian>()?;
        Ok(Self { reason, expiry_date })
    }

    fn write_body<W: Write>(&self, writer: &mut W) -> SharedResult<()> {
        write_dotnet_string(writer, &self.reason)?;
        writer.write_i64::<LittleEndian>(self.expiry_date)?;
        Ok(())
    }
}

/// LoginSuccess packet - successful login with character list
#[derive(Debug, Clone)]
pub struct LoginSuccess {
    pub characters: Vec<SelectInfo>,
}


impl Packet for LoginSuccess {
    const OPCODE: i16 = ServerPacketIds::LoginSuccess as i16;

    fn read_body<R: Read>(reader: &mut R) -> SharedResult<Self> {
        let count = reader.read_i32::<LittleEndian>()?;
        let mut characters = Vec::with_capacity(count as usize);
        
        for _ in 0..count {
            characters.push(SelectInfo::read_from(reader)?);
        }
        
        Ok(Self { characters })
    }

    fn write_body<W: Write>(&self, writer: &mut W) -> SharedResult<()> {
        writer.write_i32::<LittleEndian>(self.characters.len() as i32)?;
        
        for character in &self.characters {
            character.write_to(writer)?;
        }
        
        Ok(())
    }
}

/// StartGame packet - response to start game request
#[derive(Debug, Clone)]
pub struct StartGame {
    pub result: u8,
    // 0: Not logged in
    // 1: Character not found
    // 2: Already in game
    // 3: Success
}

impl Packet for StartGame {
    const OPCODE: i16 = ServerPacketIds::StartGame as i16;

    fn read_body<R: Read>(reader: &mut R) -> SharedResult<Self> {
        let result = reader.read_u8()?;
        Ok(Self { result })
    }

    fn write_body<W: Write>(&self, writer: &mut W) -> SharedResult<()> {
        writer.write_u8(self.result)?;
        Ok(())
    }
}

/// StartGameBanned packet - notification of game ban
#[derive(Debug, Clone)]
pub struct StartGameBanned {
    pub reason: String,
    pub expiry_date: i64, // .NET DateTime ticks
}

impl Packet for StartGameBanned {
    const OPCODE: i16 = ServerPacketIds::StartGameBanned as i16;

    fn read_body<R: Read>(reader: &mut R) -> SharedResult<Self> {
        let reason = read_dotnet_string(reader)?;
        let expiry_date = reader.read_i64::<LittleEndian>()?;
        Ok(Self { reason, expiry_date })
    }

    fn write_body<W: Write>(&self, writer: &mut W) -> SharedResult<()> {
        write_dotnet_string(writer, &self.reason)?;
        writer.write_i64::<LittleEndian>(self.expiry_date)?;
        Ok(())
    }
}

/// StartGameDelay packet - delay before starting game
#[derive(Debug, Clone)]
pub struct StartGameDelay {
    pub milliseconds: i64,
}

impl Packet for StartGameDelay {
    const OPCODE: i16 = ServerPacketIds::StartGameDelay as i16;

    fn read_body<R: Read>(reader: &mut R) -> SharedResult<Self> {
        let milliseconds = reader.read_i64::<LittleEndian>()?;
        Ok(Self { milliseconds })
    }

    fn write_body<W: Write>(&self, writer: &mut W) -> SharedResult<()> {
        writer.write_i64::<LittleEndian>(self.milliseconds)?;
        Ok(())
    }
}

/// LogOutFailed packet - notification that logout failed
#[derive(Debug, Clone)]
pub struct LogOutFailed;

impl Packet for LogOutFailed {
    const OPCODE: i16 = ServerPacketIds::LogOutFailed as i16;

    fn read_body<R: Read>(_reader: &mut R) -> SharedResult<Self> {
        Ok(Self)
    }

    fn write_body<W: Write>(&self, _writer: &mut W) -> SharedResult<()> {
        Ok(())
    }
}

/// ReturnToLogin packet - notification to return to login screen
#[derive(Debug, Clone)]
pub struct ReturnToLogin;

impl Packet for ReturnToLogin {
    const OPCODE: i16 = ServerPacketIds::ReturnToLogin as i16;

    fn read_body<R: Read>(_reader: &mut R) -> SharedResult<Self> {
        Ok(Self)
    }

    fn write_body<W: Write>(&self, _writer: &mut W) -> SharedResult<()> {
        Ok(())
    }
}
