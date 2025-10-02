//! Hero System Packets
//!
//! This module contains all hero-related packet definitions and parsers.

use crate::{
    enums::{HeroBehaviour, HeroSpawnState, AttackMode, PetMode},
    data::client_data::ClientHeroInformation,
};

#[cfg(feature = "client-parse")]
use std::io::Cursor;
#[cfg(feature = "client-parse")]
use byteorder::{LittleEndian, ReadBytesExt};

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
// Parser Functions
// ============================================================================

#[cfg(feature = "client-parse")]
pub(crate) fn parse_update_hero_spawn_state(
    payload: &[u8],
) -> Result<UpdateHeroSpawnState, String> {
    let mut cursor = Cursor::new(payload);
    let state_byte = cursor
        .read_u8()
        .map_err(|e| format!("Failed to read state: {}", e))?;
    let state = HeroSpawnState::try_from(state_byte)
        .map_err(|_| format!("Unknown hero spawn state: {}", state_byte))?;
    Ok(UpdateHeroSpawnState { state })
}

#[cfg(feature = "client-parse")]
pub(crate) fn parse_set_auto_pot_value(payload: &[u8]) -> Result<SetAutoPotValue, String> {
    let mut cursor = Cursor::new(payload);
    let stat = cursor
        .read_u8()
        .map_err(|e| format!("Failed to read stat: {}", e))?;
    let value = cursor
        .read_u32::<LittleEndian>()
        .map_err(|e| format!("Failed to read value: {}", e))?;
    Ok(SetAutoPotValue { stat, value })
}

#[cfg(feature = "client-parse")]
pub(crate) fn parse_set_hero_behaviour(payload: &[u8]) -> Result<SetHeroBehaviour, String> {
    let mut cursor = Cursor::new(payload);
    let attack_mode_byte = cursor
        .read_u8()
        .map_err(|e| format!("Failed to read attack_mode: {}", e))?;
    let attack_mode = AttackMode::try_from(attack_mode_byte)
        .map_err(|_| format!("Unknown attack mode: {}", attack_mode_byte))?;
    let pet_mode_byte = cursor
        .read_u8()
        .map_err(|e| format!("Failed to read pet_mode: {}", e))?;
    let pet_mode = PetMode::try_from(pet_mode_byte)
        .map_err(|_| format!("Unknown pet mode: {}", pet_mode_byte))?;
    Ok(SetHeroBehaviour {
        attack_mode,
        pet_mode,
    })
}

#[cfg(feature = "client-parse")]
pub(crate) fn parse_manage_heroes(payload: &[u8]) -> Result<ManageHeroes, String> {
    let mut cursor = Cursor::new(payload);
    let count = cursor
        .read_i32::<LittleEndian>()
        .map_err(|e| format!("Failed to read hero count: {}", e))?;
    let mut heroes = Vec::new();
    for _ in 0..count {
        heroes.push(ClientHeroInformation::read_from(&mut cursor)?);
    }
    Ok(ManageHeroes { heroes })
}

#[cfg(feature = "client-parse")]
pub(crate) fn parse_hero_create_request(payload: &[u8]) -> Result<HeroCreateRequest, String> {
    let mut cursor = Cursor::new(payload);
    let count = cursor
        .read_i32::<LittleEndian>()
        .map_err(|e| format!("Failed to read class count: {}", e))?;
    let mut can_create_class = Vec::new();
    for _ in 0..count {
        can_create_class.push(
            cursor
                .read_u8()
                .map_err(|e| format!("Failed to read can_create flag: {}", e))?
                != 0,
        );
    }
    Ok(HeroCreateRequest { can_create_class })
}
