//! Quest System Packets
//!
//! This module contains quest-related packet definitions and parsers.

use crate::client_data::{ClientQuestInfo, ClientQuestProgress};
use std::io::Cursor;

// ============================================================================
// Packet Structures
// ============================================================================

/// Quest status changed
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ChangeQuest {
    pub quest: ClientQuestProgress,
}

/// New quest information
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NewQuestInfo {
    pub quest: ClientQuestInfo,
}

// ============================================================================
// Parser Functions
// ============================================================================

pub(crate) fn parse_change_quest(payload: &[u8]) -> Result<ChangeQuest, String> {
    let mut cursor = Cursor::new(payload);
    let quest = ClientQuestProgress::read_from(&mut cursor)?;
    Ok(ChangeQuest { quest })
}

pub(crate) fn parse_new_quest_info(payload: &[u8]) -> Result<NewQuestInfo, String> {
    let mut cursor = Cursor::new(payload);
    let quest = ClientQuestInfo::read_from(&mut cursor)?;
    Ok(NewQuestInfo { quest })
}
