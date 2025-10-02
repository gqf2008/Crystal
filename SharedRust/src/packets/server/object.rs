//! Object Status Packets
//!
//! This module contains object status-related packet definitions and parsers.

use crate::{enums::SpellEffect, map::Point};

#[cfg(feature = "client-parse")]
use std::io::Cursor;
#[cfg(feature = "client-parse")]
use byteorder::{LittleEndian, ReadBytesExt};

// ============================================================================
// Packet Structures
// ============================================================================

/// Object health update
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ObjectHealth {
    pub object_id: u32,
    pub percent: u8,
    pub expire: u16,
}

/// Object mana update
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ObjectMana {
    pub object_id: u32,
    pub percent: u8,
}

/// Object hidden status changed
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ObjectHidden {
    pub object_id: u32,
    pub hidden: bool,
}

/// Map effect
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct MapEffect {
    pub location: Point,
    pub effect: SpellEffect,
    pub value: i32,
}

// ============================================================================
// Parser Functions
// ============================================================================

#[cfg(feature = "client-parse")]
pub(crate) fn parse_object_health(payload: &[u8]) -> Result<ObjectHealth, String> {
    let mut cursor = Cursor::new(payload);
    let object_id = cursor
        .read_u32::<LittleEndian>()
        .map_err(|e| format!("Failed to read object_id: {}", e))?;
    let percent = cursor
        .read_u8()
        .map_err(|e| format!("Failed to read percent: {}", e))?;
    let expire = cursor
        .read_u16::<LittleEndian>()
        .map_err(|e| format!("Failed to read expire: {}", e))?;
    Ok(ObjectHealth {
        object_id,
        percent,
        expire,
    })
}

#[cfg(feature = "client-parse")]
pub(crate) fn parse_object_mana(payload: &[u8]) -> Result<ObjectMana, String> {
    let mut cursor = Cursor::new(payload);
    let object_id = cursor
        .read_u32::<LittleEndian>()
        .map_err(|e| format!("Failed to read object_id: {}", e))?;
    let percent = cursor
        .read_u8()
        .map_err(|e| format!("Failed to read percent: {}", e))?;
    Ok(ObjectMana { object_id, percent })
}

#[cfg(feature = "client-parse")]
pub(crate) fn parse_object_hidden(payload: &[u8]) -> Result<ObjectHidden, String> {
    let mut cursor = Cursor::new(payload);
    let object_id = cursor
        .read_u32::<LittleEndian>()
        .map_err(|e| format!("Failed to read object_id: {}", e))?;
    let hidden = cursor
        .read_u8()
        .map_err(|e| format!("Failed to read hidden: {}", e))?
        != 0;
    Ok(ObjectHidden { object_id, hidden })
}

#[cfg(feature = "client-parse")]
pub(crate) fn parse_map_effect(payload: &[u8]) -> Result<MapEffect, String> {
    let mut cursor = Cursor::new(payload);
    let x = cursor
        .read_i32::<LittleEndian>()
        .map_err(|e| format!("Failed to read x: {}", e))?;
    let y = cursor
        .read_i32::<LittleEndian>()
        .map_err(|e| format!("Failed to read y: {}", e))?;
    let location = Point { x, y };
    let effect_byte = cursor
        .read_u8()
        .map_err(|e| format!("Failed to read effect: {}", e))?;
    let effect = SpellEffect::try_from(effect_byte)
        .map_err(|_| format!("Unknown spell effect: {}", effect_byte))?;
    let value = cursor
        .read_i32::<LittleEndian>()
        .map_err(|e| format!("Failed to read value: {}", e))?;
    Ok(MapEffect {
        location,
        effect,
        value,
    })
}
