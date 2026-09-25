//! Account & Character Management Packets
//!
//! This module contains account and character management packet definitions and parsers.

use super::super::base::Packet;
use crate::data::stats::SharedResult;
use crate::{
    binary::{read_dotnet_string, write_dotnet_string},
    enums::{MirClass, MirGender, ServerPacketIds},
};
use byteorder::{LittleEndian, ReadBytesExt, WriteBytesExt};
use std::io::{Read, Write};

// ============================================================================
// Packet Structures
// ============================================================================

/// New character creation response
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct NewCharacter {
    pub result: u8,
}

/// New character creation successful
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NewCharacterSuccess {
    pub character: super::super::CharacterSummary,
}

/// Delete character request response
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct DeleteCharacter {
    pub result: u8,
}

/// Delete character successful
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct DeleteCharacterSuccess {
    pub character_index: i32,
}

// ============================================================================
// PacketMessage Implementations
// ============================================================================

impl Packet for NewCharacter {
    const OPCODE: i16 = ServerPacketIds::NewCharacter as i16;

    fn read_body<R: Read>(reader: &mut R) -> SharedResult<Self> {
        let result = reader.read_u8()?;
        Ok(Self { result })
    }

    fn write_body<W: Write>(&self, writer: &mut W) -> SharedResult<()> {
        writer.write_u8(self.result)?;
        Ok(())
    }
}

impl Packet for NewCharacterSuccess {
    const OPCODE: i16 = ServerPacketIds::NewCharacterSuccess as i16;

    fn read_body<R: Read>(reader: &mut R) -> SharedResult<Self> {
        let index = reader.read_i32::<LittleEndian>()?;
        let name = read_dotnet_string(reader)?;
        let level = reader.read_u16::<LittleEndian>()?;
        let class = MirClass::try_from(reader.read_u8()?)?;
        let gender = MirGender::try_from(reader.read_u8()?)?;
        let ticks = reader.read_i64::<LittleEndian>()?;
        let unix_epoch_ticks = 621355968000000000i64;
        let unix_seconds = (ticks - unix_epoch_ticks) / 10000000;
        use chrono::{TimeZone, Utc};
        let last_access = Utc
            .timestamp_opt(unix_seconds, 0)
            .single()
            .ok_or(crate::data::stats::SharedError::InvalidDateTime)?;

        Ok(Self {
            character: super::super::CharacterSummary {
                index,
                name,
                level,
                class,
                gender,
                last_access,
            },
        })
    }

    fn write_body<W: Write>(&self, writer: &mut W) -> SharedResult<()> {
        writer.write_i32::<LittleEndian>(self.character.index)?;
        write_dotnet_string(writer, &self.character.name)?;
        writer.write_u16::<LittleEndian>(self.character.level)?;
        writer.write_u8(self.character.class as u8)?;
        writer.write_u8(self.character.gender as u8)?;
        let unix_epoch_ticks = 621355968000000000i64;
        let ticks = self.character.last_access.timestamp() * 10000000 + unix_epoch_ticks;
        writer.write_i64::<LittleEndian>(ticks)?;
        Ok(())
    }
}

impl Packet for DeleteCharacter {
    const OPCODE: i16 = ServerPacketIds::DeleteCharacter as i16;

    fn read_body<R: Read>(reader: &mut R) -> SharedResult<Self> {
        let result = reader.read_u8()?;
        Ok(Self { result })
    }

    fn write_body<W: Write>(&self, writer: &mut W) -> SharedResult<()> {
        writer.write_u8(self.result)?;
        Ok(())
    }
}

impl Packet for DeleteCharacterSuccess {
    const OPCODE: i16 = ServerPacketIds::DeleteCharacterSuccess as i16;

    fn read_body<R: Read>(reader: &mut R) -> SharedResult<Self> {
        let character_index = reader.read_i32::<LittleEndian>()?;
        Ok(Self { character_index })
    }

    fn write_body<W: Write>(&self, writer: &mut W) -> SharedResult<()> {
        writer.write_i32::<LittleEndian>(self.character_index)?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::packets::Packet;
    use chrono::TimeZone;

    /// `NewCharacterSuccess` 的读写必须**自洽**（字段序 index → name → level → class → gender → ticks）。
    ///
    /// 为什么专门钉它：2026-09-26 实测过一次"服务端手写 body 时把 name 放在 index 前面"的事故——
    /// 客户端 `read_body` 解析失败、`if let Ok(...)` 静默跳过，角色**建出来了但界面毫无反应**
    /// （玩家体感＝"无法创建角色"）。判据必须是**往返相等**，而不是"两边各自看着都对"。
    #[test]
    fn new_character_success_roundtrip() {
        let one = NewCharacterSuccess {
            character: crate::packets::CharacterSummary {
                index: 7,
                name: "小明明".to_string(),
                level: 3,
                class: crate::enums::MirClass::Taoist,
                gender: crate::enums::MirGender::Female,
                last_access: chrono::Utc.timestamp_opt(1_700_000_000, 0).unwrap(),
            },
        };
        let mut body = Vec::new();
        one.write_body(&mut body).expect("write_body");
        // 首 4 字节必须是 index（而不是 dotnet 字符串长度）——正是事故里被写反的那一处
        assert_eq!(
            i32::from_le_bytes([body[0], body[1], body[2], body[3]]),
            7,
            "首字段必须是 index；若这里变成字符串长度，就是字段序又写反了"
        );
        let two = NewCharacterSuccess::read_body(&mut body.as_slice()).expect("read_body");
        assert_eq!(two, one, "NewCharacterSuccess 往返必须相等");
    }
}
