//! Server to Client Packets
//! 
//! This module contains all packet definitions that are sent from the server to the client.
//! Ported from Crystal/Shared/ServerPackets.cs
//!
//! Total packets: 200+
//! Status: ⚠️ In Progress (Phase 1: Connection & Login - 20 packets)

use std::io::{Read, Write};
use byteorder::{LittleEndian, ReadBytesExt, WriteBytesExt};
use chrono::{DateTime, TimeZone, Utc};

use crate::binary::{read_dotnet_string, write_dotnet_string};
use crate::client_data::{SelectInfo, ClientMagic, ClientIntelligentCreature};
use crate::enums::{
    MirClass, MirGender, MirDirection, LightSetting,
    BuffType, Color, HeroBehaviour, LevelEffects, PoisonType, SpellEffect, WeatherSetting,
    IntelligentCreatureType,
};
use crate::item::UserItem;
use crate::map::Point;
use crate::packet::PacketMessage;
use crate::packet_ids::ServerPacketId;
use crate::stats::{SharedError, SharedResult};

//=============================================================================
// Connection Packets (5 packets)
//=============================================================================

/// Server connected confirmation
/// Sent immediately when client connects to server
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct Connected;

impl PacketMessage for Connected {
    const OPCODE: i16 = ServerPacketId::Connected as i16;

    fn read_body<R: Read>(_reader: &mut R) -> SharedResult<Self> {
        Ok(Connected)
    }

    fn write_body<W: Write>(&self, _writer: &mut W) -> SharedResult<()> {
        Ok(())
    }
}

/// Client version check result
/// Result codes:
/// - 0: Wrong Version
/// - 1: Correct Version
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ClientVersion {
    pub result: u8,
}

impl PacketMessage for ClientVersion {
    const OPCODE: i16 = ServerPacketId::ClientVersion as i16;

    fn read_body<R: Read>(reader: &mut R) -> SharedResult<Self> {
        Ok(ClientVersion {
            result: reader.read_u8()?,
        })
    }

    fn write_body<W: Write>(&self, writer: &mut W) -> SharedResult<()> {
        writer.write_u8(self.result)?;
        Ok(())
    }
}

/// Server disconnect notification
/// Reason codes:
/// - 0: Server Closing
/// - 1: Another User
/// - 2: Packet Error
/// - 3: Server Crashed
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Disconnect {
    pub reason: u8,
}

impl PacketMessage for Disconnect {
    const OPCODE: i16 = ServerPacketId::Disconnect as i16;

    fn read_body<R: Read>(reader: &mut R) -> SharedResult<Self> {
        Ok(Disconnect {
            reason: reader.read_u8()?,
        })
    }

    fn write_body<W: Write>(&self, writer: &mut W) -> SharedResult<()> {
        writer.write_u8(self.reason)?;
        Ok(())
    }
}

/// Keep alive ping
/// Used to maintain connection and measure latency
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct KeepAlive {
    pub time: i64,
}

impl PacketMessage for KeepAlive {
    const OPCODE: i16 = ServerPacketId::KeepAlive as i16;

    fn read_body<R: Read>(reader: &mut R) -> SharedResult<Self> {
        Ok(KeepAlive {
            time: reader.read_i64::<LittleEndian>()?,
        })
    }

    fn write_body<W: Write>(&self, writer: &mut W) -> SharedResult<()> {
        writer.write_i64::<LittleEndian>(self.time)?;
        Ok(())
    }
}

/// New account creation result
/// Result codes:
/// - 0: Disabled
/// - 1: Bad AccountID
/// - 2: Bad Password
/// - 3: Bad Email
/// - 4: Bad Name
/// - 5: Bad Question
/// - 6: Bad Answer
/// - 7: Account Exists
/// - 8: Success
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NewAccount {
    pub result: u8,
}

impl PacketMessage for NewAccount {
    const OPCODE: i16 = ServerPacketId::NewAccount as i16;

    fn read_body<R: Read>(reader: &mut R) -> SharedResult<Self> {
        Ok(NewAccount {
            result: reader.read_u8()?,
        })
    }

    fn write_body<W: Write>(&self, writer: &mut W) -> SharedResult<()> {
        writer.write_u8(self.result)?;
        Ok(())
    }
}

//=============================================================================
// Login & Account Packets (15 packets)
//=============================================================================

/// Password change result
/// Result codes:
/// - 0: Disabled
/// - 1: Bad AccountID
/// - 2: Bad Current Password
/// - 3: Bad New Password
/// - 4: Account Not Exist
/// - 5: Wrong Password
/// - 6: Success
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ChangePassword {
    pub result: u8,
}

impl PacketMessage for ChangePassword {
    const OPCODE: i16 = ServerPacketId::ChangePassword as i16;

    fn read_body<R: Read>(reader: &mut R) -> SharedResult<Self> {
        Ok(ChangePassword {
            result: reader.read_u8()?,
        })
    }

    fn write_body<W: Write>(&self, writer: &mut W) -> SharedResult<()> {
        writer.write_u8(self.result)?;
        Ok(())
    }
}

/// Password change banned notification
/// Sent when account is banned from changing password
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ChangePasswordBanned {
    pub reason: String,
    pub expiry_date: DateTime<Utc>,
}

impl PacketMessage for ChangePasswordBanned {
    const OPCODE: i16 = ServerPacketId::ChangePasswordBanned as i16;

    fn read_body<R: Read>(reader: &mut R) -> SharedResult<Self> {
        let reason = read_dotnet_string(reader)?;
        let ticks = reader.read_i64::<LittleEndian>()?;
        // .NET DateTime ticks: 100-nanosecond intervals since 0001-01-01 00:00:00
        // Convert to Unix timestamp (seconds since 1970-01-01 00:00:00)
        let unix_epoch_ticks = 621355968000000000i64; // .NET ticks at Unix epoch
        let unix_seconds = (ticks - unix_epoch_ticks) / 10000000;
        let expiry_date = Utc.timestamp_opt(unix_seconds, 0)
            .single()
            .ok_or(SharedError::InvalidDateTime)?;

        Ok(ChangePasswordBanned {
            reason,
            expiry_date,
        })
    }

    fn write_body<W: Write>(&self, writer: &mut W) -> SharedResult<()> {
        write_dotnet_string(writer, &self.reason)?;
        let unix_epoch_ticks = 621355968000000000i64;
        let ticks = self.expiry_date.timestamp() * 10000000 + unix_epoch_ticks;
        writer.write_i64::<LittleEndian>(ticks)?;
        Ok(())
    }
}

/// Login result
/// Result codes:
/// - 0: Disabled
/// - 1: Bad AccountID
/// - 2: Bad Password
/// - 3: Account Not Exist
/// - 4: Wrong Password
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Login {
    pub result: u8,
}

impl PacketMessage for Login {
    const OPCODE: i16 = ServerPacketId::Login as i16;

    fn read_body<R: Read>(reader: &mut R) -> SharedResult<Self> {
        Ok(Login {
            result: reader.read_u8()?,
        })
    }

    fn write_body<W: Write>(&self, writer: &mut W) -> SharedResult<()> {
        writer.write_u8(self.result)?;
        Ok(())
    }
}

/// Login banned notification
/// Sent when account is banned from logging in
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LoginBanned {
    pub reason: String,
    pub expiry_date: DateTime<Utc>,
}

impl PacketMessage for LoginBanned {
    const OPCODE: i16 = ServerPacketId::LoginBanned as i16;

    fn read_body<R: Read>(reader: &mut R) -> SharedResult<Self> {
        let reason = read_dotnet_string(reader)?;
        let ticks = reader.read_i64::<LittleEndian>()?;
        let unix_epoch_ticks = 621355968000000000i64;
        let unix_seconds = (ticks - unix_epoch_ticks) / 10000000;
        let expiry_date = Utc.timestamp_opt(unix_seconds, 0)
            .single()
            .ok_or(SharedError::InvalidDateTime)?;

        Ok(LoginBanned {
            reason,
            expiry_date,
        })
    }

    fn write_body<W: Write>(&self, writer: &mut W) -> SharedResult<()> {
        write_dotnet_string(writer, &self.reason)?;
        let unix_epoch_ticks = 621355968000000000i64;
        let ticks = self.expiry_date.timestamp() * 10000000 + unix_epoch_ticks;
        writer.write_i64::<LittleEndian>(ticks)?;
        Ok(())
    }
}

/// Login successful - character list
/// Contains list of characters available for selection
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LoginSuccess {
    pub characters: Vec<SelectInfo>,
}

impl PacketMessage for LoginSuccess {
    const OPCODE: i16 = ServerPacketId::LoginSuccess as i16;

    fn read_body<R: Read>(reader: &mut R) -> SharedResult<Self> {
        let count = reader.read_i32::<LittleEndian>()?;
        if count < 0 {
            return Err(SharedError::NegativeLength {
                field: "characters",
                length: count,
            });
        }
        
        let mut characters = Vec::with_capacity(count as usize);
        for _ in 0..count {
            characters.push(SelectInfo::read_from(reader)?);
        }

        Ok(LoginSuccess { characters })
    }

    fn write_body<W: Write>(&self, writer: &mut W) -> SharedResult<()> {
        let count = i32::try_from(self.characters.len())
            .map_err(|_| SharedError::PacketTooLarge(self.characters.len()))?;
        writer.write_i32::<LittleEndian>(count)?;
        
        for character in &self.characters {
            character.write_to(writer)?;
        }
        Ok(())
    }
}

/// New character creation result
/// Result codes:
/// - 0: Disabled
/// - 1: Bad Character Name
/// - 2: Bad Gender
/// - 3: Bad Class
/// - 4: Max Characters
/// - 5: Character Exists
/// - 10: Success
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NewCharacter {
    pub result: u8,
}

impl PacketMessage for NewCharacter {
    const OPCODE: i16 = ServerPacketId::NewCharacter as i16;

    fn read_body<R: Read>(reader: &mut R) -> SharedResult<Self> {
        Ok(NewCharacter {
            result: reader.read_u8()?,
        })
    }

    fn write_body<W: Write>(&self, writer: &mut W) -> SharedResult<()> {
        writer.write_u8(self.result)?;
        Ok(())
    }
}

/// New character created successfully
/// Returns the created character information
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NewCharacterSuccess {
    pub char_info: SelectInfo,
}

impl PacketMessage for NewCharacterSuccess {
    const OPCODE: i16 = ServerPacketId::NewCharacterSuccess as i16;

    fn read_body<R: Read>(reader: &mut R) -> SharedResult<Self> {
        Ok(NewCharacterSuccess {
            char_info: SelectInfo::read_from(reader)?,
        })
    }

    fn write_body<W: Write>(&self, writer: &mut W) -> SharedResult<()> {
        self.char_info.write_to(writer)?;
        Ok(())
    }
}

/// Delete character result
/// Result codes:
/// - 0: Disabled
/// - 1: Character Not Found
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DeleteCharacter {
    pub result: u8,
}

impl PacketMessage for DeleteCharacter {
    const OPCODE: i16 = ServerPacketId::DeleteCharacter as i16;

    fn read_body<R: Read>(reader: &mut R) -> SharedResult<Self> {
        Ok(DeleteCharacter {
            result: reader.read_u8()?,
        })
    }

    fn write_body<W: Write>(&self, writer: &mut W) -> SharedResult<()> {
        writer.write_u8(self.result)?;
        Ok(())
    }
}

/// Delete character successful
/// Returns the index of the deleted character
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DeleteCharacterSuccess {
    pub character_index: i32,
}

impl PacketMessage for DeleteCharacterSuccess {
    const OPCODE: i16 = ServerPacketId::DeleteCharacterSuccess as i16;

    fn read_body<R: Read>(reader: &mut R) -> SharedResult<Self> {
        Ok(DeleteCharacterSuccess {
            character_index: reader.read_i32::<LittleEndian>()?,
        })
    }

    fn write_body<W: Write>(&self, writer: &mut W) -> SharedResult<()> {
        writer.write_i32::<LittleEndian>(self.character_index)?;
        Ok(())
    }
}

/// Start game result
/// Result codes:
/// - 0: Disabled
/// - 1: Not logged in
/// - 2: Character not found
/// - 3: Start Game Error
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct StartGame {
    pub result: u8,
    pub resolution: i32,
}

impl PacketMessage for StartGame {
    const OPCODE: i16 = ServerPacketId::StartGame as i16;

    fn read_body<R: Read>(reader: &mut R) -> SharedResult<Self> {
        Ok(StartGame {
            result: reader.read_u8()?,
            resolution: reader.read_i32::<LittleEndian>()?,
        })
    }

    fn write_body<W: Write>(&self, writer: &mut W) -> SharedResult<()> {
        writer.write_u8(self.result)?;
        writer.write_i32::<LittleEndian>(self.resolution)?;
        Ok(())
    }
}

/// Start game banned notification
/// Sent when character/account is banned from starting game
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct StartGameBanned {
    pub reason: String,
    pub expiry_date: DateTime<Utc>,
}

impl PacketMessage for StartGameBanned {
    const OPCODE: i16 = ServerPacketId::StartGameBanned as i16;

    fn read_body<R: Read>(reader: &mut R) -> SharedResult<Self> {
        let reason = read_dotnet_string(reader)?;
        let ticks = reader.read_i64::<LittleEndian>()?;
        let unix_epoch_ticks = 621355968000000000i64;
        let unix_seconds = (ticks - unix_epoch_ticks) / 10000000;
        let expiry_date = Utc.timestamp_opt(unix_seconds, 0)
            .single()
            .ok_or(SharedError::InvalidDateTime)?;

        Ok(StartGameBanned {
            reason,
            expiry_date,
        })
    }

    fn write_body<W: Write>(&self, writer: &mut W) -> SharedResult<()> {
        write_dotnet_string(writer, &self.reason)?;
        let unix_epoch_ticks = 621355968000000000i64;
        let ticks = self.expiry_date.timestamp() * 10000000 + unix_epoch_ticks;
        writer.write_i64::<LittleEndian>(ticks)?;
        Ok(())
    }
}

/// Start game delay notification
/// Sent when there's a delay before game can start
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct StartGameDelay {
    pub milliseconds: i64,
}

impl PacketMessage for StartGameDelay {
    const OPCODE: i16 = ServerPacketId::StartGameDelay as i16;

    fn read_body<R: Read>(reader: &mut R) -> SharedResult<Self> {
        Ok(StartGameDelay {
            milliseconds: reader.read_i64::<LittleEndian>()?,
        })
    }

    fn write_body<W: Write>(&self, writer: &mut W) -> SharedResult<()> {
        writer.write_i64::<LittleEndian>(self.milliseconds)?;
        Ok(())
    }
}

/// Logout successful - character list
/// Returns updated character list after logout
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LogOutSuccess {
    pub characters: Vec<SelectInfo>,
}

impl PacketMessage for LogOutSuccess {
    const OPCODE: i16 = ServerPacketId::LogOutSuccess as i16;

    fn read_body<R: Read>(reader: &mut R) -> SharedResult<Self> {
        let count = reader.read_i32::<LittleEndian>()?;
        if count < 0 {
            return Err(SharedError::NegativeLength {
                field: "characters",
                length: count,
            });
        }
        
        let mut characters = Vec::with_capacity(count as usize);
        for _ in 0..count {
            characters.push(SelectInfo::read_from(reader)?);
        }

        Ok(LogOutSuccess { characters })
    }

    fn write_body<W: Write>(&self, writer: &mut W) -> SharedResult<()> {
        let count = i32::try_from(self.characters.len())
            .map_err(|_| SharedError::PacketTooLarge(self.characters.len()))?;
        writer.write_i32::<LittleEndian>(count)?;
        
        for character in &self.characters {
            character.write_to(writer)?;
        }
        Ok(())
    }
}

/// Logout failed notification
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct LogOutFailed;

impl PacketMessage for LogOutFailed {
    const OPCODE: i16 = ServerPacketId::LogOutFailed as i16;

    fn read_body<R: Read>(_reader: &mut R) -> SharedResult<Self> {
        Ok(LogOutFailed)
    }

    fn write_body<W: Write>(&self, _writer: &mut W) -> SharedResult<()> {
        Ok(())
    }
}

/// Return to login screen notification
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct ReturnToLogin;

impl PacketMessage for ReturnToLogin {
    const OPCODE: i16 = ServerPacketId::ReturnToLogin as i16;

    fn read_body<R: Read>(_reader: &mut R) -> SharedResult<Self> {
        Ok(ReturnToLogin)
    }

    fn write_body<W: Write>(&self, _writer: &mut W) -> SharedResult<()> {
        Ok(())
    }
}

//=============================================================================
// Unit Tests
//=============================================================================

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Cursor;

    #[test]
    fn test_connected() {
        let packet = Connected;
        let mut buffer = Vec::new();
        packet.write_body(&mut buffer).unwrap();
        assert_eq!(buffer.len(), 0); // Empty packet

        let mut cursor = Cursor::new(buffer);
        let decoded = Connected::read_body(&mut cursor).unwrap();
        assert_eq!(packet, decoded);
    }

    #[test]
    fn test_client_version() {
        let packet = ClientVersion { result: 1 };
        let mut buffer = Vec::new();
        packet.write_body(&mut buffer).unwrap();

        let mut cursor = Cursor::new(buffer);
        let decoded = ClientVersion::read_body(&mut cursor).unwrap();
        assert_eq!(packet, decoded);
        assert_eq!(decoded.result, 1);
    }

    #[test]
    fn test_disconnect() {
        let packet = Disconnect { reason: 2 };
        let mut buffer = Vec::new();
        packet.write_body(&mut buffer).unwrap();

        let mut cursor = Cursor::new(buffer);
        let decoded = Disconnect::read_body(&mut cursor).unwrap();
        assert_eq!(packet, decoded);
        assert_eq!(decoded.reason, 2);
    }

    #[test]
    fn test_keep_alive() {
        let packet = KeepAlive { time: 123456789 };
        let mut buffer = Vec::new();
        packet.write_body(&mut buffer).unwrap();

        let mut cursor = Cursor::new(buffer);
        let decoded = KeepAlive::read_body(&mut cursor).unwrap();
        assert_eq!(packet, decoded);
        assert_eq!(decoded.time, 123456789);
    }

    #[test]
    fn test_new_account() {
        let packet = NewAccount { result: 8 }; // Success
        let mut buffer = Vec::new();
        packet.write_body(&mut buffer).unwrap();

        let mut cursor = Cursor::new(buffer);
        let decoded = NewAccount::read_body(&mut cursor).unwrap();
        assert_eq!(packet, decoded);
        assert_eq!(decoded.result, 8);
    }

    #[test]
    fn test_change_password() {
        let packet = ChangePassword { result: 6 }; // Success
        let mut buffer = Vec::new();
        packet.write_body(&mut buffer).unwrap();

        let mut cursor = Cursor::new(buffer);
        let decoded = ChangePassword::read_body(&mut cursor).unwrap();
        assert_eq!(packet, decoded);
        assert_eq!(decoded.result, 6);
    }

    #[test]
    fn test_login() {
        let packet = Login { result: 0 }; // Disabled
        let mut buffer = Vec::new();
        packet.write_body(&mut buffer).unwrap();

        let mut cursor = Cursor::new(buffer);
        let decoded = Login::read_body(&mut cursor).unwrap();
        assert_eq!(packet, decoded);
    }

    #[test]
    fn test_new_character() {
        let packet = NewCharacter { result: 10 }; // Success
        let mut buffer = Vec::new();
        packet.write_body(&mut buffer).unwrap();

        let mut cursor = Cursor::new(buffer);
        let decoded = NewCharacter::read_body(&mut cursor).unwrap();
        assert_eq!(packet, decoded);
        assert_eq!(decoded.result, 10);
    }

    #[test]
    fn test_delete_character() {
        let packet = DeleteCharacter { result: 1 };
        let mut buffer = Vec::new();
        packet.write_body(&mut buffer).unwrap();

        let mut cursor = Cursor::new(buffer);
        let decoded = DeleteCharacter::read_body(&mut cursor).unwrap();
        assert_eq!(packet, decoded);
    }

    #[test]
    fn test_delete_character_success() {
        let packet = DeleteCharacterSuccess {
            character_index: 2,
        };
        let mut buffer = Vec::new();
        packet.write_body(&mut buffer).unwrap();

        let mut cursor = Cursor::new(buffer);
        let decoded = DeleteCharacterSuccess::read_body(&mut cursor).unwrap();
        assert_eq!(packet, decoded);
        assert_eq!(decoded.character_index, 2);
    }

    #[test]
    fn test_start_game() {
        let packet = StartGame {
            result: 0,
            resolution: 1024,
        };
        let mut buffer = Vec::new();
        packet.write_body(&mut buffer).unwrap();

        let mut cursor = Cursor::new(buffer);
        let decoded = StartGame::read_body(&mut cursor).unwrap();
        assert_eq!(packet, decoded);
        assert_eq!(decoded.resolution, 1024);
    }

    #[test]
    fn test_start_game_delay() {
        let packet = StartGameDelay {
            milliseconds: 5000,
        };
        let mut buffer = Vec::new();
        packet.write_body(&mut buffer).unwrap();

        let mut cursor = Cursor::new(buffer);
        let decoded = StartGameDelay::read_body(&mut cursor).unwrap();
        assert_eq!(packet, decoded);
        assert_eq!(decoded.milliseconds, 5000);
    }

    #[test]
    fn test_logout_failed() {
        let packet = LogOutFailed;
        let mut buffer = Vec::new();
        packet.write_body(&mut buffer).unwrap();
        assert_eq!(buffer.len(), 0); // Empty packet

        let mut cursor = Cursor::new(buffer);
        let decoded = LogOutFailed::read_body(&mut cursor).unwrap();
        assert_eq!(packet, decoded);
    }

    #[test]
    fn test_return_to_login() {
        let packet = ReturnToLogin;
        let mut buffer = Vec::new();
        packet.write_body(&mut buffer).unwrap();
        assert_eq!(buffer.len(), 0); // Empty packet

        let mut cursor = Cursor::new(buffer);
        let decoded = ReturnToLogin::read_body(&mut cursor).unwrap();
        assert_eq!(packet, decoded);
    }
}

// ==================== Phase 1.2: Player & Map Packets ====================

/// User's complete information (sent when entering game)
#[derive(Debug, Clone, PartialEq)]
pub struct UserInformation {
    pub object_id: u32,
    pub real_id: u32,
    pub name: String,
    pub guild_name: String,
    pub guild_rank: String,
    pub name_colour: Color,
    pub class: MirClass,
    pub gender: MirGender,
    pub level: u16,
    pub location: Point,
    pub direction: MirDirection,
    pub hair: u8,
    pub hp: i32,
    pub mp: i32,
    pub experience: i64,
    pub max_experience: i64,
    pub level_effects: LevelEffects,
    pub has_hero: bool,
    pub hero_behaviour: HeroBehaviour,
    pub inventory: Option<Vec<Option<UserItem>>>,
    pub equipment: Option<Vec<Option<UserItem>>>,
    pub quest_inventory: Option<Vec<Option<UserItem>>>,
    pub gold: u32,
    pub credit: u32,
    pub has_expanded_storage: bool,
    pub expanded_storage_expiry_time: DateTime<Utc>,
    pub magics: Vec<ClientMagic>,
    pub intelligent_creatures: Vec<ClientIntelligentCreature>,
    pub summoned_creature_type: IntelligentCreatureType,
    pub creature_summoned: bool,
    pub allow_observe: bool,
    pub observer: bool,
}

impl PacketMessage for UserInformation {
    const OPCODE: i16 = ServerPacketId::UserInformation as i16;

    fn read_body<R: Read>(reader: &mut R) -> SharedResult<Self> {
        let object_id = reader.read_u32::<LittleEndian>()?;
        let real_id = reader.read_u32::<LittleEndian>()?;
        let name = read_dotnet_string(reader)?;
        let guild_name = read_dotnet_string(reader)?;
        let guild_rank = read_dotnet_string(reader)?;
        let name_colour = Color::from_argb(reader.read_i32::<LittleEndian>()?);
        let class = MirClass::try_from(reader.read_u8()?)?;
        let gender = MirGender::try_from(reader.read_u8()?)?;
        let level = reader.read_u16::<LittleEndian>()?;
        let location = Point {
            x: reader.read_i32::<LittleEndian>()?,
            y: reader.read_i32::<LittleEndian>()?,
        };
        let direction = MirDirection::try_from(reader.read_u8()?)?;
        let hair = reader.read_u8()?;
        let hp = reader.read_i32::<LittleEndian>()?;
        let mp = reader.read_i32::<LittleEndian>()?;
        let experience = reader.read_i64::<LittleEndian>()?;
        let max_experience = reader.read_i64::<LittleEndian>()?;
        let level_effects = LevelEffects::from_bits_truncate(reader.read_u16::<LittleEndian>()?);
        let has_hero = reader.read_u8()? != 0;
        let hero_behaviour = HeroBehaviour::try_from(reader.read_u8()?)?;

        // Inventory
        let inventory = if reader.read_u8()? != 0 {
            let len = reader.read_i32::<LittleEndian>()? as usize;
            let mut items = Vec::with_capacity(len);
            for _ in 0..len {
                if reader.read_u8()? != 0 {
                    items.push(Some(UserItem::read_from(reader)?));
                } else {
                    items.push(None);
                }
            }
            Some(items)
        } else {
            None
        };

        // Equipment
        let equipment = if reader.read_u8()? != 0 {
            let len = reader.read_i32::<LittleEndian>()? as usize;
            let mut items = Vec::with_capacity(len);
            for _ in 0..len {
                if reader.read_u8()? != 0 {
                    items.push(Some(UserItem::read_from(reader)?));
                } else {
                    items.push(None);
                }
            }
            Some(items)
        } else {
            None
        };

        // Quest Inventory
        let quest_inventory = if reader.read_u8()? != 0 {
            let len = reader.read_i32::<LittleEndian>()? as usize;
            let mut items = Vec::with_capacity(len);
            for _ in 0..len {
                if reader.read_u8()? != 0 {
                    items.push(Some(UserItem::read_from(reader)?));
                } else {
                    items.push(None);
                }
            }
            Some(items)
        } else {
            None
        };

        let gold = reader.read_u32::<LittleEndian>()?;
        let credit = reader.read_u32::<LittleEndian>()?;
        let has_expanded_storage = reader.read_u8()? != 0;

        // DateTime conversion
        let ticks = reader.read_i64::<LittleEndian>()?;
        let unix_epoch_ticks = 621355968000000000i64;
        let unix_seconds = (ticks - unix_epoch_ticks) / 10000000;
        let expanded_storage_expiry_time = Utc
            .timestamp_opt(unix_seconds, 0)
            .single()
            .ok_or(SharedError::InvalidDateTime)?;

        // Magics
        let magic_count = reader.read_i32::<LittleEndian>()? as usize;
        let mut magics = Vec::with_capacity(magic_count);
        for _ in 0..magic_count {
            magics.push(ClientMagic::read_from(reader)?);
        }

        // Intelligent Creatures
        let creature_count = reader.read_i32::<LittleEndian>()? as usize;
        let mut intelligent_creatures = Vec::with_capacity(creature_count);
        for _ in 0..creature_count {
            intelligent_creatures.push(ClientIntelligentCreature::read_from(reader)?);
        }

        let summoned_creature_type = IntelligentCreatureType::try_from(reader.read_u8()?)?;
        let creature_summoned = reader.read_u8()? != 0;
        let allow_observe = reader.read_u8()? != 0;
        let observer = reader.read_u8()? != 0;

        Ok(UserInformation {
            object_id,
            real_id,
            name,
            guild_name,
            guild_rank,
            name_colour,
            class,
            gender,
            level,
            location,
            direction,
            hair,
            hp,
            mp,
            experience,
            max_experience,
            level_effects,
            has_hero,
            hero_behaviour,
            inventory,
            equipment,
            quest_inventory,
            gold,
            credit,
            has_expanded_storage,
            expanded_storage_expiry_time,
            magics,
            intelligent_creatures,
            summoned_creature_type,
            creature_summoned,
            allow_observe,
            observer,
        })
    }

    fn write_body<W: Write>(&self, writer: &mut W) -> SharedResult<()> {
        writer.write_u32::<LittleEndian>(self.object_id)?;
        writer.write_u32::<LittleEndian>(self.real_id)?;
        write_dotnet_string(writer, &self.name)?;
        write_dotnet_string(writer, &self.guild_name)?;
        write_dotnet_string(writer, &self.guild_rank)?;
        writer.write_i32::<LittleEndian>(self.name_colour.to_argb())?;
        writer.write_u8(self.class as u8)?;
        writer.write_u8(self.gender as u8)?;
        writer.write_u16::<LittleEndian>(self.level)?;
        writer.write_i32::<LittleEndian>(self.location.x)?;
        writer.write_i32::<LittleEndian>(self.location.y)?;
        writer.write_u8(self.direction as u8)?;
        writer.write_u8(self.hair)?;
        writer.write_i32::<LittleEndian>(self.hp)?;
        writer.write_i32::<LittleEndian>(self.mp)?;
        writer.write_i64::<LittleEndian>(self.experience)?;
        writer.write_i64::<LittleEndian>(self.max_experience)?;
        writer.write_u16::<LittleEndian>(self.level_effects.bits())?;
        writer.write_u8(if self.has_hero { 1 } else { 0 })?;
        writer.write_u8(self.hero_behaviour as u8)?;

        // Inventory
        if let Some(ref inv) = self.inventory {
            writer.write_u8(1)?;
            writer.write_i32::<LittleEndian>(inv.len() as i32)?;
            for item_opt in inv {
                if let Some(ref item) = item_opt {
                    writer.write_u8(1)?;
                    item.write_to(writer)?;
                } else {
                    writer.write_u8(0)?;
                }
            }
        } else {
            writer.write_u8(0)?;
        }

        // Equipment
        if let Some(ref eq) = self.equipment {
            writer.write_u8(1)?;
            writer.write_i32::<LittleEndian>(eq.len() as i32)?;
            for item_opt in eq {
                if let Some(ref item) = item_opt {
                    writer.write_u8(1)?;
                    item.write_to(writer)?;
                } else {
                    writer.write_u8(0)?;
                }
            }
        } else {
            writer.write_u8(0)?;
        }

        // Quest Inventory
        if let Some(ref qi) = self.quest_inventory {
            writer.write_u8(1)?;
            writer.write_i32::<LittleEndian>(qi.len() as i32)?;
            for item_opt in qi {
                if let Some(ref item) = item_opt {
                    writer.write_u8(1)?;
                    item.write_to(writer)?;
                } else {
                    writer.write_u8(0)?;
                }
            }
        } else {
            writer.write_u8(0)?;
        }

        writer.write_u32::<LittleEndian>(self.gold)?;
        writer.write_u32::<LittleEndian>(self.credit)?;
        writer.write_u8(if self.has_expanded_storage { 1 } else { 0 })?;

        // DateTime conversion
        let unix_epoch_ticks = 621355968000000000i64;
        let ticks = self.expanded_storage_expiry_time.timestamp() * 10000000 + unix_epoch_ticks;
        writer.write_i64::<LittleEndian>(ticks)?;

        // Magics
        writer.write_i32::<LittleEndian>(self.magics.len() as i32)?;
        for magic in &self.magics {
            magic.write_to(writer)?;
        }

        // Intelligent Creatures
        writer.write_i32::<LittleEndian>(self.intelligent_creatures.len() as i32)?;
        for creature in &self.intelligent_creatures {
            creature.write_to(writer)?;
        }

        writer.write_u8(self.summoned_creature_type as u8)?;
        writer.write_u8(if self.creature_summoned { 1 } else { 0 })?;
        writer.write_u8(if self.allow_observe { 1 } else { 0 })?;
        writer.write_u8(if self.observer { 1 } else { 0 })?;

        Ok(())
    }
}

/// User's location and direction
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct UserLocation {
    pub location: Point,
    pub direction: MirDirection,
}

impl PacketMessage for UserLocation {
    const OPCODE: i16 = ServerPacketId::UserLocation as i16;

    fn read_body<R: Read>(reader: &mut R) -> SharedResult<Self> {
        let location = Point {
            x: reader.read_i32::<LittleEndian>()?,
            y: reader.read_i32::<LittleEndian>()?,
        };
        let direction = MirDirection::try_from(reader.read_u8()?)?;
        Ok(UserLocation { location, direction })
    }

    fn write_body<W: Write>(&self, writer: &mut W) -> SharedResult<()> {
        writer.write_i32::<LittleEndian>(self.location.x)?;
        writer.write_i32::<LittleEndian>(self.location.y)?;
        writer.write_u8(self.direction as u8)?;
        Ok(())
    }
}

/// Refresh user's inventory slots
#[derive(Debug, Clone, PartialEq)]
pub struct UserSlotsRefresh {
    pub inventory: Option<Vec<Option<UserItem>>>,
    pub equipment: Option<Vec<Option<UserItem>>>,
}

impl PacketMessage for UserSlotsRefresh {
    const OPCODE: i16 = ServerPacketId::UserSlotsRefresh as i16;

    fn read_body<R: Read>(reader: &mut R) -> SharedResult<Self> {
        let inventory = if reader.read_u8()? != 0 {
            let len = reader.read_i32::<LittleEndian>()? as usize;
            let mut items = Vec::with_capacity(len);
            for _ in 0..len {
                if reader.read_u8()? != 0 {
                    items.push(Some(UserItem::read_from(reader)?));
                } else {
                    items.push(None);
                }
            }
            Some(items)
        } else {
            None
        };

        let equipment = if reader.read_u8()? != 0 {
            let len = reader.read_i32::<LittleEndian>()? as usize;
            let mut items = Vec::with_capacity(len);
            for _ in 0..len {
                if reader.read_u8()? != 0 {
                    items.push(Some(UserItem::read_from(reader)?));
                } else {
                    items.push(None);
                }
            }
            Some(items)
        } else {
            None
        };

        Ok(UserSlotsRefresh { inventory, equipment })
    }

    fn write_body<W: Write>(&self, writer: &mut W) -> SharedResult<()> {
        if let Some(ref inv) = self.inventory {
            writer.write_u8(1)?;
            writer.write_i32::<LittleEndian>(inv.len() as i32)?;
            for item_opt in inv {
                if let Some(ref item) = item_opt {
                    writer.write_u8(1)?;
                    item.write_to(writer)?;
                } else {
                    writer.write_u8(0)?;
                }
            }
        } else {
            writer.write_u8(0)?;
        }

        if let Some(ref eq) = self.equipment {
            writer.write_u8(1)?;
            writer.write_i32::<LittleEndian>(eq.len() as i32)?;
            for item_opt in eq {
                if let Some(ref item) = item_opt {
                    writer.write_u8(1)?;
                    item.write_to(writer)?;
                } else {
                    writer.write_u8(0)?;
                }
            }
        } else {
            writer.write_u8(0)?;
        }

        Ok(())
    }
}

/// Player object in view
#[derive(Debug, Clone, PartialEq)]
pub struct ObjectPlayer {
    pub object_id: u32,
    pub name: String,
    pub guild_name: String,
    pub guild_rank_name: String,
    pub name_colour: Color,
    pub class: MirClass,
    pub gender: MirGender,
    pub level: u16,
    pub location: Point,
    pub direction: MirDirection,
    pub hair: u8,
    pub light: u8,
    pub weapon: i16,
    pub weapon_effect: i16,
    pub armour: i16,
    pub poison: PoisonType,
    pub dead: bool,
    pub hidden: bool,
    pub effect: SpellEffect,
    pub wing_effect: u8,
    pub extra: bool,
    pub mount_type: i16,
    pub riding_mount: bool,
    pub fishing: bool,
    pub transform_type: i16,
    pub element_orb_effect: u32,
    pub element_orb_lvl: u32,
    pub element_orb_max: u32,
    pub buffs: Vec<BuffType>,
    pub level_effects: LevelEffects,
}

impl PacketMessage for ObjectPlayer {
    const OPCODE: i16 = ServerPacketId::ObjectPlayer as i16;

    fn read_body<R: Read>(reader: &mut R) -> SharedResult<Self> {
        let object_id = reader.read_u32::<LittleEndian>()?;
        let name = read_dotnet_string(reader)?;
        let guild_name = read_dotnet_string(reader)?;
        let guild_rank_name = read_dotnet_string(reader)?;
        let name_colour = Color::from_argb(reader.read_i32::<LittleEndian>()?);
        let class = MirClass::try_from(reader.read_u8()?)?;
        let gender = MirGender::try_from(reader.read_u8()?)?;
        let level = reader.read_u16::<LittleEndian>()?;
        let location = Point {
            x: reader.read_i32::<LittleEndian>()?,
            y: reader.read_i32::<LittleEndian>()?,
        };
        let direction = MirDirection::try_from(reader.read_u8()?)?;
        let hair = reader.read_u8()?;
        let light = reader.read_u8()?;
        let weapon = reader.read_i16::<LittleEndian>()?;
        let weapon_effect = reader.read_i16::<LittleEndian>()?;
        let armour = reader.read_i16::<LittleEndian>()?;
        let poison = PoisonType::from_bits_truncate(reader.read_u16::<LittleEndian>()?);
        let dead = reader.read_u8()? != 0;
        let hidden = reader.read_u8()? != 0;
        let effect = SpellEffect::try_from(reader.read_u8()?)?;
        let wing_effect = reader.read_u8()?;
        let extra = reader.read_u8()? != 0;
        let mount_type = reader.read_i16::<LittleEndian>()?;
        let riding_mount = reader.read_u8()? != 0;
        let fishing = reader.read_u8()? != 0;
        let transform_type = reader.read_i16::<LittleEndian>()?;
        let element_orb_effect = reader.read_u32::<LittleEndian>()?;
        let element_orb_lvl = reader.read_u32::<LittleEndian>()?;
        let element_orb_max = reader.read_u32::<LittleEndian>()?;

        let buff_count = reader.read_i32::<LittleEndian>()? as usize;
        let mut buffs = Vec::with_capacity(buff_count);
        for _ in 0..buff_count {
            buffs.push(BuffType::try_from(reader.read_u8()?)?);
        }

        let level_effects = LevelEffects::from_bits_truncate(reader.read_u16::<LittleEndian>()?);

        Ok(ObjectPlayer {
            object_id,
            name,
            guild_name,
            guild_rank_name,
            name_colour,
            class,
            gender,
            level,
            location,
            direction,
            hair,
            light,
            weapon,
            weapon_effect,
            armour,
            poison,
            dead,
            hidden,
            effect,
            wing_effect,
            extra,
            mount_type,
            riding_mount,
            fishing,
            transform_type,
            element_orb_effect,
            element_orb_lvl,
            element_orb_max,
            buffs,
            level_effects,
        })
    }

    fn write_body<W: Write>(&self, writer: &mut W) -> SharedResult<()> {
        writer.write_u32::<LittleEndian>(self.object_id)?;
        write_dotnet_string(writer, &self.name)?;
        write_dotnet_string(writer, &self.guild_name)?;
        write_dotnet_string(writer, &self.guild_rank_name)?;
        writer.write_i32::<LittleEndian>(self.name_colour.to_argb())?;
        writer.write_u8(self.class as u8)?;
        writer.write_u8(self.gender as u8)?;
        writer.write_u16::<LittleEndian>(self.level)?;
        writer.write_i32::<LittleEndian>(self.location.x)?;
        writer.write_i32::<LittleEndian>(self.location.y)?;
        writer.write_u8(self.direction as u8)?;
        writer.write_u8(self.hair)?;
        writer.write_u8(self.light)?;
        writer.write_i16::<LittleEndian>(self.weapon)?;
        writer.write_i16::<LittleEndian>(self.weapon_effect)?;
        writer.write_i16::<LittleEndian>(self.armour)?;
        writer.write_u16::<LittleEndian>(self.poison.bits())?;
        writer.write_u8(if self.dead { 1 } else { 0 })?;
        writer.write_u8(if self.hidden { 1 } else { 0 })?;
        writer.write_u8(self.effect as u8)?;
        writer.write_u8(self.wing_effect)?;
        writer.write_u8(if self.extra { 1 } else { 0 })?;
        writer.write_i16::<LittleEndian>(self.mount_type)?;
        writer.write_u8(if self.riding_mount { 1 } else { 0 })?;
        writer.write_u8(if self.fishing { 1 } else { 0 })?;
        writer.write_i16::<LittleEndian>(self.transform_type)?;
        writer.write_u32::<LittleEndian>(self.element_orb_effect)?;
        writer.write_u32::<LittleEndian>(self.element_orb_lvl)?;
        writer.write_u32::<LittleEndian>(self.element_orb_max)?;

        writer.write_i32::<LittleEndian>(self.buffs.len() as i32)?;
        for buff in &self.buffs {
            writer.write_u8(*buff as u8)?;
        }

        writer.write_u16::<LittleEndian>(self.level_effects.bits())?;

        Ok(())
    }
}

/// Hero object (extends ObjectPlayer)
#[derive(Debug, Clone, PartialEq)]
pub struct ObjectHero {
    pub player: ObjectPlayer,
    pub owner_name: String,
}

impl PacketMessage for ObjectHero {
    const OPCODE: i16 = ServerPacketId::ObjectHero as i16;

    fn read_body<R: Read>(reader: &mut R) -> SharedResult<Self> {
        let player = ObjectPlayer::read_body(reader)?;
        let owner_name = read_dotnet_string(reader)?;
        Ok(ObjectHero { player, owner_name })
    }

    fn write_body<W: Write>(&self, writer: &mut W) -> SharedResult<()> {
        self.player.write_body(writer)?;
        write_dotnet_string(writer, &self.owner_name)?;
        Ok(())
    }
}

/// Remove object from view
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct ObjectRemove {
    pub object_id: u32,
}

impl PacketMessage for ObjectRemove {
    const OPCODE: i16 = ServerPacketId::ObjectRemove as i16;

    fn read_body<R: Read>(reader: &mut R) -> SharedResult<Self> {
        Ok(ObjectRemove {
            object_id: reader.read_u32::<LittleEndian>()?,
        })
    }

    fn write_body<W: Write>(&self, writer: &mut W) -> SharedResult<()> {
        writer.write_u32::<LittleEndian>(self.object_id)?;
        Ok(())
    }
}

// ==================== Movement Packets ====================

/// Object turns to face a direction
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ObjectTurn {
    pub object_id: u32,
    pub location: Point,
    pub direction: MirDirection,
}

impl PacketMessage for ObjectTurn {
    const OPCODE: i16 = ServerPacketId::ObjectTurn as i16;

    fn read_body<R: Read>(reader: &mut R) -> SharedResult<Self> {
        let object_id = reader.read_u32::<LittleEndian>()?;
        let location = Point {
            x: reader.read_i32::<LittleEndian>()?,
            y: reader.read_i32::<LittleEndian>()?,
        };
        let direction = MirDirection::try_from(reader.read_u8()?)?;
        Ok(ObjectTurn { object_id, location, direction })
    }

    fn write_body<W: Write>(&self, writer: &mut W) -> SharedResult<()> {
        writer.write_u32::<LittleEndian>(self.object_id)?;
        writer.write_i32::<LittleEndian>(self.location.x)?;
        writer.write_i32::<LittleEndian>(self.location.y)?;
        writer.write_u8(self.direction as u8)?;
        Ok(())
    }
}

/// Object walks
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ObjectWalk {
    pub object_id: u32,
    pub location: Point,
    pub direction: MirDirection,
}

impl PacketMessage for ObjectWalk {
    const OPCODE: i16 = ServerPacketId::ObjectWalk as i16;

    fn read_body<R: Read>(reader: &mut R) -> SharedResult<Self> {
        let object_id = reader.read_u32::<LittleEndian>()?;
        let location = Point {
            x: reader.read_i32::<LittleEndian>()?,
            y: reader.read_i32::<LittleEndian>()?,
        };
        let direction = MirDirection::try_from(reader.read_u8()?)?;
        Ok(ObjectWalk { object_id, location, direction })
    }

    fn write_body<W: Write>(&self, writer: &mut W) -> SharedResult<()> {
        writer.write_u32::<LittleEndian>(self.object_id)?;
        writer.write_i32::<LittleEndian>(self.location.x)?;
        writer.write_i32::<LittleEndian>(self.location.y)?;
        writer.write_u8(self.direction as u8)?;
        Ok(())
    }
}

/// Object runs
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ObjectRun {
    pub object_id: u32,
    pub location: Point,
    pub direction: MirDirection,
}

impl PacketMessage for ObjectRun {
    const OPCODE: i16 = ServerPacketId::ObjectRun as i16;

    fn read_body<R: Read>(reader: &mut R) -> SharedResult<Self> {
        let object_id = reader.read_u32::<LittleEndian>()?;
        let location = Point {
            x: reader.read_i32::<LittleEndian>()?,
            y: reader.read_i32::<LittleEndian>()?,
        };
        let direction = MirDirection::try_from(reader.read_u8()?)?;
        Ok(ObjectRun { object_id, location, direction })
    }

    fn write_body<W: Write>(&self, writer: &mut W) -> SharedResult<()> {
        writer.write_u32::<LittleEndian>(self.object_id)?;
        writer.write_i32::<LittleEndian>(self.location.x)?;
        writer.write_i32::<LittleEndian>(self.location.y)?;
        writer.write_u8(self.direction as u8)?;
        Ok(())
    }
}

/// Player is pushed (by attack or spell)
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Pushed {
    pub location: Point,
    pub direction: MirDirection,
}

impl PacketMessage for Pushed {
    const OPCODE: i16 = ServerPacketId::Pushed as i16;

    fn read_body<R: Read>(reader: &mut R) -> SharedResult<Self> {
        let location = Point {
            x: reader.read_i32::<LittleEndian>()?,
            y: reader.read_i32::<LittleEndian>()?,
        };
        let direction = MirDirection::try_from(reader.read_u8()?)?;
        Ok(Pushed { location, direction })
    }

    fn write_body<W: Write>(&self, writer: &mut W) -> SharedResult<()> {
        writer.write_i32::<LittleEndian>(self.location.x)?;
        writer.write_i32::<LittleEndian>(self.location.y)?;
        writer.write_u8(self.direction as u8)?;
        Ok(())
    }
}

/// Object is pushed
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ObjectPushed {
    pub object_id: u32,
    pub location: Point,
    pub direction: MirDirection,
}

impl PacketMessage for ObjectPushed {
    const OPCODE: i16 = ServerPacketId::ObjectPushed as i16;

    fn read_body<R: Read>(reader: &mut R) -> SharedResult<Self> {
        let object_id = reader.read_u32::<LittleEndian>()?;
        let location = Point {
            x: reader.read_i32::<LittleEndian>()?,
            y: reader.read_i32::<LittleEndian>()?,
        };
        let direction = MirDirection::try_from(reader.read_u8()?)?;
        Ok(ObjectPushed { object_id, location, direction })
    }

    fn write_body<W: Write>(&self, writer: &mut W) -> SharedResult<()> {
        writer.write_u32::<LittleEndian>(self.object_id)?;
        writer.write_i32::<LittleEndian>(self.location.x)?;
        writer.write_i32::<LittleEndian>(self.location.y)?;
        writer.write_u8(self.direction as u8)?;
        Ok(())
    }
}

// ==================== Player Appearance & Information Packets ====================

/// Update player appearance (equipment change)
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct PlayerUpdate {
    pub object_id: u32,
    pub light: u8,
    pub weapon: i16,
    pub weapon_effect: i16,
    pub armour: i16,
    pub wing_effect: u8,
}

impl PacketMessage for PlayerUpdate {
    const OPCODE: i16 = ServerPacketId::PlayerUpdate as i16;

    fn read_body<R: Read>(reader: &mut R) -> SharedResult<Self> {
        Ok(PlayerUpdate {
            object_id: reader.read_u32::<LittleEndian>()?,
            light: reader.read_u8()?,
            weapon: reader.read_i16::<LittleEndian>()?,
            weapon_effect: reader.read_i16::<LittleEndian>()?,
            armour: reader.read_i16::<LittleEndian>()?,
            wing_effect: reader.read_u8()?,
        })
    }

    fn write_body<W: Write>(&self, writer: &mut W) -> SharedResult<()> {
        writer.write_u32::<LittleEndian>(self.object_id)?;
        writer.write_u8(self.light)?;
        writer.write_i16::<LittleEndian>(self.weapon)?;
        writer.write_i16::<LittleEndian>(self.weapon_effect)?;
        writer.write_i16::<LittleEndian>(self.armour)?;
        writer.write_u8(self.wing_effect)?;
        Ok(())
    }
}

/// Inspect player equipment
#[derive(Debug, Clone, PartialEq)]
pub struct PlayerInspect {
    pub name: String,
    pub guild_name: String,
    pub guild_rank: String,
    pub equipment: Vec<Option<UserItem>>,
    pub class: MirClass,
    pub gender: MirGender,
    pub hair: u8,
    pub level: u16,
    pub lover_name: String,
    pub allow_observe: bool,
    pub is_hero: bool,
}

impl PacketMessage for PlayerInspect {
    const OPCODE: i16 = ServerPacketId::PlayerInspect as i16;

    fn read_body<R: Read>(reader: &mut R) -> SharedResult<Self> {
        let name = read_dotnet_string(reader)?;
        let guild_name = read_dotnet_string(reader)?;
        let guild_rank = read_dotnet_string(reader)?;
        
        let equipment_len = reader.read_i32::<LittleEndian>()? as usize;
        let mut equipment = Vec::with_capacity(equipment_len);
        for _ in 0..equipment_len {
            if reader.read_u8()? != 0 {
                equipment.push(Some(UserItem::read_from(reader)?));
            } else {
                equipment.push(None);
            }
        }

        let class = MirClass::try_from(reader.read_u8()?)?;
        let gender = MirGender::try_from(reader.read_u8()?)?;
        let hair = reader.read_u8()?;
        let level = reader.read_u16::<LittleEndian>()?;
        let lover_name = read_dotnet_string(reader)?;
        let allow_observe = reader.read_u8()? != 0;
        let is_hero = reader.read_u8()? != 0;

        Ok(PlayerInspect {
            name,
            guild_name,
            guild_rank,
            equipment,
            class,
            gender,
            hair,
            level,
            lover_name,
            allow_observe,
            is_hero,
        })
    }

    fn write_body<W: Write>(&self, writer: &mut W) -> SharedResult<()> {
        write_dotnet_string(writer, &self.name)?;
        write_dotnet_string(writer, &self.guild_name)?;
        write_dotnet_string(writer, &self.guild_rank)?;
        
        writer.write_i32::<LittleEndian>(self.equipment.len() as i32)?;
        for item_opt in &self.equipment {
            if let Some(ref item) = item_opt {
                writer.write_u8(1)?;
                item.write_to(writer)?;
            } else {
                writer.write_u8(0)?;
            }
        }

        writer.write_u8(self.class as u8)?;
        writer.write_u8(self.gender as u8)?;
        writer.write_u8(self.hair)?;
        writer.write_u16::<LittleEndian>(self.level)?;
        write_dotnet_string(writer, &self.lover_name)?;
        writer.write_u8(if self.allow_observe { 1 } else { 0 })?;
        writer.write_u8(if self.is_hero { 1 } else { 0 })?;

        Ok(())
    }
}

/// Player name colour changed
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ColourChanged {
    pub name_colour: Color,
}

impl PacketMessage for ColourChanged {
    const OPCODE: i16 = ServerPacketId::ColourChanged as i16;

    fn read_body<R: Read>(reader: &mut R) -> SharedResult<Self> {
        Ok(ColourChanged {
            name_colour: Color::from_argb(reader.read_i32::<LittleEndian>()?),
        })
    }

    fn write_body<W: Write>(&self, writer: &mut W) -> SharedResult<()> {
        writer.write_i32::<LittleEndian>(self.name_colour.to_argb())?;
        Ok(())
    }
}

/// Object name colour changed
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ObjectColourChanged {
    pub object_id: u32,
    pub name_colour: Color,
}

impl PacketMessage for ObjectColourChanged {
    const OPCODE: i16 = ServerPacketId::ObjectColourChanged as i16;

    fn read_body<R: Read>(reader: &mut R) -> SharedResult<Self> {
        Ok(ObjectColourChanged {
            object_id: reader.read_u32::<LittleEndian>()?,
            name_colour: Color::from_argb(reader.read_i32::<LittleEndian>()?),
        })
    }

    fn write_body<W: Write>(&self, writer: &mut W) -> SharedResult<()> {
        writer.write_u32::<LittleEndian>(self.object_id)?;
        writer.write_i32::<LittleEndian>(self.name_colour.to_argb())?;
        Ok(())
    }
}

// ==================== Map Packets ====================

/// Map changed notification
#[derive(Debug, Clone, PartialEq)]
pub struct MapChanged {
    pub map_index: i32,
    pub file_name: String,
    pub title: String,
    pub mini_map: u16,
    pub big_map: u16,
    pub lights: LightSetting,
    pub location: Point,
    pub direction: MirDirection,
    pub map_dark_light: u8,
    pub music: u16,
    pub weather: WeatherSetting,
}

impl PacketMessage for MapChanged {
    const OPCODE: i16 = ServerPacketId::MapChanged as i16;

    fn read_body<R: Read>(reader: &mut R) -> SharedResult<Self> {
        let map_index = reader.read_i32::<LittleEndian>()?;
        let file_name = read_dotnet_string(reader)?;
        let title = read_dotnet_string(reader)?;
        let mini_map = reader.read_u16::<LittleEndian>()?;
        let big_map = reader.read_u16::<LittleEndian>()?;
        let lights = LightSetting::try_from(reader.read_u8()?)?;
        let location = Point {
            x: reader.read_i32::<LittleEndian>()?,
            y: reader.read_i32::<LittleEndian>()?,
        };
        let direction = MirDirection::try_from(reader.read_u8()?)?;
        let map_dark_light = reader.read_u8()?;
        let music = reader.read_u16::<LittleEndian>()?;
        let weather = WeatherSetting::from_bits_truncate(reader.read_u16::<LittleEndian>()?);

        Ok(MapChanged {
            map_index,
            file_name,
            title,
            mini_map,
            big_map,
            lights,
            location,
            direction,
            map_dark_light,
            music,
            weather,
        })
    }

    fn write_body<W: Write>(&self, writer: &mut W) -> SharedResult<()> {
        writer.write_i32::<LittleEndian>(self.map_index)?;
        write_dotnet_string(writer, &self.file_name)?;
        write_dotnet_string(writer, &self.title)?;
        writer.write_u16::<LittleEndian>(self.mini_map)?;
        writer.write_u16::<LittleEndian>(self.big_map)?;
        writer.write_u8(self.lights as u8)?;
        writer.write_i32::<LittleEndian>(self.location.x)?;
        writer.write_i32::<LittleEndian>(self.location.y)?;
        writer.write_u8(self.direction as u8)?;
        writer.write_u8(self.map_dark_light)?;
        writer.write_u16::<LittleEndian>(self.music)?;
        writer.write_u16::<LittleEndian>(self.weather.bits())?;

        Ok(())
    }
}

/// Map information
#[derive(Debug, Clone, PartialEq)]
pub struct MapInformation {
    pub map_index: i32,
    pub file_name: String,
    pub title: String,
    pub mini_map: u16,
    pub big_map: u16,
    pub lights: LightSetting,
    pub lightning: bool,
    pub fire: bool,
    pub map_dark_light: u8,
    pub music: u16,
    pub weather_particles: WeatherSetting,
}

impl PacketMessage for MapInformation {
    const OPCODE: i16 = ServerPacketId::MapInformation as i16;

    fn read_body<R: Read>(reader: &mut R) -> SharedResult<Self> {
        let map_index = reader.read_i32::<LittleEndian>()?;
        let file_name = read_dotnet_string(reader)?;
        let title = read_dotnet_string(reader)?;
        let mini_map = reader.read_u16::<LittleEndian>()?;
        let big_map = reader.read_u16::<LittleEndian>()?;
        let lights = LightSetting::try_from(reader.read_u8()?)?;
        let bools = reader.read_u8()?;
        let lightning = (bools & 0x01) == 0x01;
        let fire = (bools & 0x02) == 0x02;
        let map_dark_light = reader.read_u8()?;
        let music = reader.read_u16::<LittleEndian>()?;
        let weather_particles = WeatherSetting::from_bits_truncate(reader.read_u16::<LittleEndian>()?);

        Ok(MapInformation {
            map_index,
            file_name,
            title,
            mini_map,
            big_map,
            lights,
            lightning,
            fire,
            map_dark_light,
            music,
            weather_particles,
        })
    }

    fn write_body<W: Write>(&self, writer: &mut W) -> SharedResult<()> {
        writer.write_i32::<LittleEndian>(self.map_index)?;
        write_dotnet_string(writer, &self.file_name)?;
        write_dotnet_string(writer, &self.title)?;
        writer.write_u16::<LittleEndian>(self.mini_map)?;
        writer.write_u16::<LittleEndian>(self.big_map)?;
        writer.write_u8(self.lights as u8)?;
        let mut bools = 0u8;
        if self.lightning {
            bools |= 0x01;
        }
        if self.fire {
            bools |= 0x02;
        }
        writer.write_u8(bools)?;
        writer.write_u8(self.map_dark_light)?;
        writer.write_u16::<LittleEndian>(self.music)?;
        writer.write_u16::<LittleEndian>(self.weather_particles.bits())?;

        Ok(())
    }
}

/// Search map result (NPC finder)
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SearchMapResult {
    pub map_index: i32,
    pub npc_index: u32,
}

impl PacketMessage for SearchMapResult {
    const OPCODE: i16 = ServerPacketId::SearchMapResult as i16;

    fn read_body<R: Read>(reader: &mut R) -> SharedResult<Self> {
        Ok(SearchMapResult {
            map_index: reader.read_i32::<LittleEndian>()?,
            npc_index: reader.read_u32::<LittleEndian>()?,
        })
    }

    fn write_body<W: Write>(&self, writer: &mut W) -> SharedResult<()> {
        writer.write_i32::<LittleEndian>(self.map_index)?;
        writer.write_u32::<LittleEndian>(self.npc_index)?;
        Ok(())
    }
}

/// Time of day (lighting change)
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct TimeOfDay {
    pub lights: LightSetting,
}

impl PacketMessage for TimeOfDay {
    const OPCODE: i16 = ServerPacketId::TimeOfDay as i16;

    fn read_body<R: Read>(reader: &mut R) -> SharedResult<Self> {
        Ok(TimeOfDay {
            lights: LightSetting::try_from(reader.read_u8()?)?,
        })
    }

    fn write_body<W: Write>(&self, writer: &mut W) -> SharedResult<()> {
        writer.write_u8(self.lights as u8)?;
        Ok(())
    }
}

/// Object teleport out effect
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ObjectTeleportOut {
    pub object_id: u32,
    pub teleport_type: u8,
}

impl PacketMessage for ObjectTeleportOut {
    const OPCODE: i16 = ServerPacketId::ObjectTeleportOut as i16;

    fn read_body<R: Read>(reader: &mut R) -> SharedResult<Self> {
        Ok(ObjectTeleportOut {
            object_id: reader.read_u32::<LittleEndian>()?,
            teleport_type: reader.read_u8()?,
        })
    }

    fn write_body<W: Write>(&self, writer: &mut W) -> SharedResult<()> {
        writer.write_u32::<LittleEndian>(self.object_id)?;
        writer.write_u8(self.teleport_type)?;
        Ok(())
    }
}

/// Object teleport in effect
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ObjectTeleportIn {
    pub object_id: u32,
    pub teleport_type: u8,
}

impl PacketMessage for ObjectTeleportIn {
    const OPCODE: i16 = ServerPacketId::ObjectTeleportIn as i16;

    fn read_body<R: Read>(reader: &mut R) -> SharedResult<Self> {
        Ok(ObjectTeleportIn {
            object_id: reader.read_u32::<LittleEndian>()?,
            teleport_type: reader.read_u8()?,
        })
    }

    fn write_body<W: Write>(&self, writer: &mut W) -> SharedResult<()> {
        writer.write_u32::<LittleEndian>(self.object_id)?;
        writer.write_u8(self.teleport_type)?;
        Ok(())
    }
}

/// Player teleporting in (to trigger effect)
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct TeleportIn;

impl PacketMessage for TeleportIn {
    const OPCODE: i16 = ServerPacketId::TeleportIn as i16;

    fn read_body<R: Read>(_reader: &mut R) -> SharedResult<Self> {
        Ok(TeleportIn)
    }

    fn write_body<W: Write>(&self, _writer: &mut W) -> SharedResult<()> {
        Ok(())
    }
}

/// Object hidden
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ObjectHide {
    pub object_id: u32,
}

impl PacketMessage for ObjectHide {
    const OPCODE: i16 = ServerPacketId::ObjectHide as i16;

    fn read_body<R: Read>(reader: &mut R) -> SharedResult<Self> {
        Ok(ObjectHide {
            object_id: reader.read_u32::<LittleEndian>()?,
        })
    }

    fn write_body<W: Write>(&self, writer: &mut W) -> SharedResult<()> {
        writer.write_u32::<LittleEndian>(self.object_id)?;
        Ok(())
    }
}

/// Object shown
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ObjectShow {
    pub object_id: u32,
}

impl PacketMessage for ObjectShow {
    const OPCODE: i16 = ServerPacketId::ObjectShow as i16;

    fn read_body<R: Read>(reader: &mut R) -> SharedResult<Self> {
        Ok(ObjectShow {
            object_id: reader.read_u32::<LittleEndian>()?,
        })
    }

    fn write_body<W: Write>(&self, writer: &mut W) -> SharedResult<()> {
        writer.write_u32::<LittleEndian>(self.object_id)?;
        Ok(())
    }
}
