//! Account & Character Management Packets
//!
//! This module contains account and character management packet definitions and parsers.

use byteorder::ReadBytesExt;
use crate::binary::read_dotnet_string;
use std::io::Cursor;

// Re-use parse_character_summary from player module
use super::player::parse_character_summary;

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
// Parser Functions
// ============================================================================

pub(crate) fn parse_new_character(payload: &[u8]) -> Result<NewCharacter, String> {
    let mut cursor = Cursor::new(payload);
    let result = cursor
        .read_u8()
        .map_err(|e| format!("Failed to read result: {}", e))?;
    Ok(NewCharacter { result })
}

pub(crate) fn parse_new_character_success(payload: &[u8]) -> Result<NewCharacterSuccess, String> {
    let mut cursor = Cursor::new(payload);
    let character = parse_character_summary(&mut cursor)?;
    Ok(NewCharacterSuccess { character })
}

pub(crate) fn parse_delete_character(payload: &[u8]) -> Result<DeleteCharacter, String> {
    let mut cursor = Cursor::new(payload);
    let result = cursor
        .read_u8()
        .map_err(|e| format!("Failed to read result: {}", e))?;
    Ok(DeleteCharacter { result })
}

pub(crate) fn parse_delete_character_success(
    payload: &[u8],
) -> Result<DeleteCharacterSuccess, String> {
    let mut cursor = Cursor::new(payload);
    let character_index = cursor
        .read_i32::<byteorder::LittleEndian>()
        .map_err(|e| format!("Failed to read character_index: {}", e))?;
    Ok(DeleteCharacterSuccess { character_index })
}
