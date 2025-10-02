//! NPC System Packets
//!
//! This module contains all NPC-related packet definitions and parsers.

#[cfg(feature = "client-parse")]
use std::io::Cursor;
#[cfg(feature = "client-parse")]
use byteorder::{LittleEndian, ReadBytesExt};
#[cfg(feature = "client-parse")]
use crate::binary::read_dotnet_string;

// ============================================================================
// Packet Structures
// ============================================================================

/// NPC opens sell dialog
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct NPCSell;

/// NPC repair dialog with repair rate
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct NPCRepair {
    pub rate: f32,
}

/// NPC special repair dialog with repair rate
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct NPCSRepair {
    pub rate: f32,
}

/// NPC refine dialog
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct NPCRefine {
    pub rate: f32,
    pub refining: bool,
}

/// NPC check refine status
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct NPCCheckRefine;

/// NPC collect refine result
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct NPCCollectRefine {
    pub success: bool,
}

/// NPC replace wedding ring dialog
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct NPCReplaceWedRing {
    pub rate: f32,
}

/// NPC storage dialog
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct NPCStorage;

/// NPC requests input from player
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NPCRequestInput {
    pub message: String,
    pub max_length: u8,
}

// ============================================================================
// Parser Functions
// ============================================================================

#[cfg(feature = "client-parse")]
pub(crate) fn parse_npc_sell(_payload: &[u8]) -> Result<NPCSell, String> {
    Ok(NPCSell)
}

#[cfg(feature = "client-parse")]
pub(crate) fn parse_npc_repair(payload: &[u8]) -> Result<NPCRepair, String> {
    let mut cursor = Cursor::new(payload);
    let rate = cursor
        .read_f32::<LittleEndian>()
        .map_err(|e| format!("Failed to read rate: {}", e))?;
    Ok(NPCRepair { rate })
}

#[cfg(feature = "client-parse")]
pub(crate) fn parse_npc_srepair(payload: &[u8]) -> Result<NPCSRepair, String> {
    let mut cursor = Cursor::new(payload);
    let rate = cursor
        .read_f32::<LittleEndian>()
        .map_err(|e| format!("Failed to read rate: {}", e))?;
    Ok(NPCSRepair { rate })
}

#[cfg(feature = "client-parse")]
pub(crate) fn parse_npc_refine(payload: &[u8]) -> Result<NPCRefine, String> {
    let mut cursor = Cursor::new(payload);
    let rate = cursor
        .read_f32::<LittleEndian>()
        .map_err(|e| format!("Failed to read rate: {}", e))?;
    let refining = cursor
        .read_u8()
        .map_err(|e| format!("Failed to read refining: {}", e))?
        != 0;
    Ok(NPCRefine { rate, refining })
}

#[cfg(feature = "client-parse")]
pub(crate) fn parse_npc_check_refine(_payload: &[u8]) -> Result<NPCCheckRefine, String> {
    Ok(NPCCheckRefine)
}

#[cfg(feature = "client-parse")]
pub(crate) fn parse_npc_collect_refine(payload: &[u8]) -> Result<NPCCollectRefine, String> {
    let mut cursor = Cursor::new(payload);
    let success = cursor
        .read_u8()
        .map_err(|e| format!("Failed to read success: {}", e))?
        != 0;
    Ok(NPCCollectRefine { success })
}

#[cfg(feature = "client-parse")]
pub(crate) fn parse_npc_replace_wed_ring(payload: &[u8]) -> Result<NPCReplaceWedRing, String> {
    let mut cursor = Cursor::new(payload);
    let rate = cursor
        .read_f32::<LittleEndian>()
        .map_err(|e| format!("Failed to read rate: {}", e))?;
    Ok(NPCReplaceWedRing { rate })
}

#[cfg(feature = "client-parse")]
pub(crate) fn parse_npc_storage(_payload: &[u8]) -> Result<NPCStorage, String> {
    Ok(NPCStorage)
}

#[cfg(feature = "client-parse")]
pub(crate) fn parse_npc_request_input(payload: &[u8]) -> Result<NPCRequestInput, String> {
    let mut cursor = Cursor::new(payload);
    let message = read_dotnet_string(&mut cursor)?;
    let max_length = cursor
        .read_u8()
        .map_err(|e| format!("Failed to read max_length: {}", e))?;
    Ok(NPCRequestInput {
        message,
        max_length,
    })
}
