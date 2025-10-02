//! Hero System Packets
//!
//! This module contains all hero-related packet definitions and parsers.

use std::io::{Read, Write};
use byteorder::{LittleEndian, ReadBytesExt, WriteBytesExt};
use crate::{
    enums::{HeroSpawnState, AttackMode, PetMode, ServerPacketIds},
    data::client_data::ClientHeroInformation,
};
use super::super::base::Packet;
use crate::data::stats::SharedResult;

// ============================================================================
// Packet Structures
// ============================================================================

/// Update hero spawn state
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct UpdateHeroSpawnState {
    pub state: HeroSpawnState,
}

/// Set auto potion value
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SetAutoPotValue {
    pub stat: u8,
    pub value: u32,
}

/// Set hero behaviour
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SetHeroBehaviour {
    pub attack_mode: AttackMode,
    pub pet_mode: PetMode,
}

/// Manage heroes list
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ManageHeroes {
    pub heroes: Vec<ClientHeroInformation>,
}

/// Hero creation request response
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct HeroCreateRequest {
    pub can_create_class: Vec<bool>,
}

// ============================================================================
// PacketMessage Implementations
// ============================================================================

impl Packet for UpdateHeroSpawnState {
    const OPCODE: i16 = ServerPacketIds::UpdateHeroSpawnState as i16;

    fn read_body<R: Read>(reader: &mut R) -> SharedResult<Self> {
        let state = HeroSpawnState::try_from(reader.read_u8()?)?;
        Ok(Self { state })
    }

    fn write_body<W: Write>(&self, writer: &mut W) -> SharedResult<()> {
        writer.write_u8(self.state as u8)?;
        Ok(())
    }
}

impl Packet for SetAutoPotValue {
    const OPCODE: i16 = ServerPacketIds::SetAutoPotValue as i16;

    fn read_body<R: Read>(reader: &mut R) -> SharedResult<Self> {
        let stat = reader.read_u8()?;
        let value = reader.read_u32::<LittleEndian>()?;
        Ok(Self { stat, value })
    }

    fn write_body<W: Write>(&self, writer: &mut W) -> SharedResult<()> {
        writer.write_u8(self.stat)?;
        writer.write_u32::<LittleEndian>(self.value)?;
        Ok(())
    }
}

impl Packet for SetHeroBehaviour {
    const OPCODE: i16 = ServerPacketIds::SetHeroBehaviour as i16;

    fn read_body<R: Read>(reader: &mut R) -> SharedResult<Self> {
        let attack_mode = AttackMode::try_from(reader.read_u8()?)?;
        let pet_mode = PetMode::try_from(reader.read_u8()?)?;
        Ok(Self {
            attack_mode,
            pet_mode,
        })
    }

    fn write_body<W: Write>(&self, writer: &mut W) -> SharedResult<()> {
        writer.write_u8(self.attack_mode as u8)?;
        writer.write_u8(self.pet_mode as u8)?;
        Ok(())
    }
}

impl Packet for ManageHeroes {
    const OPCODE: i16 = ServerPacketIds::ManageHeroes as i16;

    fn read_body<R: Read>(reader: &mut R) -> SharedResult<Self> {
        let count = reader.read_i32::<LittleEndian>()? as usize;
        let mut heroes = Vec::with_capacity(count);
        for _ in 0..count {
            heroes.push(ClientHeroInformation::read_from(reader)?);
        }
        Ok(Self { heroes })
    }

    fn write_body<W: Write>(&self, writer: &mut W) -> SharedResult<()> {
        use crate::binary::write_dotnet_string;
        
        writer.write_i32::<LittleEndian>(self.heroes.len() as i32)?;
        for hero in &self.heroes {
            // Manual serialization since ClientHeroInformation lacks write_to
            writer.write_i32::<LittleEndian>(hero.index)?;
            write_dotnet_string(writer, &hero.name)?;
            writer.write_u16::<LittleEndian>(hero.level)?;
            writer.write_u8(hero.class as u8)?;
            writer.write_u8(hero.gender as u8)?;
        }
        Ok(())
    }
}

impl Packet for HeroCreateRequest {
    const OPCODE: i16 = ServerPacketIds::HeroCreateRequest as i16;

    fn read_body<R: Read>(reader: &mut R) -> SharedResult<Self> {
        let count = reader.read_i32::<LittleEndian>()? as usize;
        let mut can_create_class = Vec::with_capacity(count);
        for _ in 0..count {
            can_create_class.push(reader.read_u8()? != 0);
        }
        Ok(Self { can_create_class })
    }

    fn write_body<W: Write>(&self, writer: &mut W) -> SharedResult<()> {
        writer.write_i32::<LittleEndian>(self.can_create_class.len() as i32)?;
        for &can_create in &self.can_create_class {
            writer.write_u8(if can_create { 1 } else { 0 })?;
        }
        Ok(())
    }
}
