//! Magic System Packets
//!
//! This module contains all magic/spell-related packet definitions and parsers.

#[cfg(feature = "client-parse")]
use std::io::Cursor;
#[cfg(feature = "client-parse")]
use byteorder::{LittleEndian, ReadBytesExt};

use crate::{data::client_data::ClientMagic, enums::Spell};

// ============================================================================
// Packet Structures
// ============================================================================

/// Player learned a new magic/spell
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NewMagic {
    pub magic: ClientMagic,
    pub hero: bool,
}

/// Magic leveled up
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct MagicLeveled {
    pub spell: Spell,
    pub level: u8,
    pub hero: bool,
}

/// Magic removed from player
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct RemoveMagic {
    pub spell: Spell,
    pub hero: bool,
}

/// Spell toggle status changed
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SpellToggle {
    pub spell: Spell,
    pub can_use: bool,
    pub hero: bool,
}

// ============================================================================
// Parser Functions
// ============================================================================

#[cfg(feature = "client-parse")]
pub(crate) fn parse_new_magic(payload: &[u8]) -> Result<NewMagic, String> {
    let mut cursor = Cursor::new(payload);
    let magic = ClientMagic::read_from(&mut cursor)?;
    let hero = cursor
        .read_u8()
        .map_err(|e| format!("Failed to read hero: {}", e))?
        != 0;
    Ok(NewMagic { magic, hero })
}

#[cfg(feature = "client-parse")]
pub(crate) fn parse_magic_leveled(payload: &[u8]) -> Result<MagicLeveled, String> {
    let mut cursor = Cursor::new(payload);
    let spell_byte = cursor
        .read_u8()
        .map_err(|e| format!("Failed to read spell: {}", e))?;
    let spell =
        Spell::try_from(spell_byte).map_err(|_| format!("Unknown spell: {}", spell_byte))?;
    let level = cursor
        .read_u8()
        .map_err(|e| format!("Failed to read level: {}", e))?;
    let hero = cursor
        .read_u8()
        .map_err(|e| format!("Failed to read hero: {}", e))?
        != 0;
    Ok(MagicLeveled { spell, level, hero })
}

#[cfg(feature = "client-parse")]
pub(crate) fn parse_remove_magic(payload: &[u8]) -> Result<RemoveMagic, String> {
    let mut cursor = Cursor::new(payload);
    let spell_byte = cursor
        .read_u8()
        .map_err(|e| format!("Failed to read spell: {}", e))?;
    let spell =
        Spell::try_from(spell_byte).map_err(|_| format!("Unknown spell: {}", spell_byte))?;
    let hero = cursor
        .read_u8()
        .map_err(|e| format!("Failed to read hero: {}", e))?
        != 0;
    Ok(RemoveMagic { spell, hero })
}

#[cfg(feature = "client-parse")]
pub(crate) fn parse_spell_toggle(payload: &[u8]) -> Result<SpellToggle, String> {
    let mut cursor = Cursor::new(payload);
    let spell_byte = cursor
        .read_u8()
        .map_err(|e| format!("Failed to read spell: {}", e))?;
    let spell =
        Spell::try_from(spell_byte).map_err(|_| format!("Unknown spell: {}", spell_byte))?;
    let can_use = cursor
        .read_u8()
        .map_err(|e| format!("Failed to read can_use: {}", e))?
        != 0;
    let hero = cursor
        .read_u8()
        .map_err(|e| format!("Failed to read hero: {}", e))?
        != 0;
    Ok(SpellToggle {
        spell,
        can_use,
        hero,
    })
}
