//! Group System Packets
//!
//! This module contains group/party-related packet definitions and parsers.

use crate::map::Point;

#[cfg(feature = "client-parse")]
use std::io::Cursor;
#[cfg(feature = "client-parse")]
use byteorder::{LittleEndian, ReadBytesExt};
#[cfg(feature = "client-parse")]
use crate::binary::read_dotnet_string;

// ============================================================================
// Packet Structures
// ============================================================================

/// Switch group mode
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SwitchGroup {
    pub allow_group: bool,
}

/// Group members map info
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GroupMembersMap {
    pub members: Vec<String>,
}

/// Send member location
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SendMemberLocation {
    pub member_name: String,
    pub location: Point,
}

// ============================================================================
// Parser Functions
// ============================================================================

#[cfg(feature = "client-parse")]
pub(crate) fn parse_switch_group(payload: &[u8]) -> Result<SwitchGroup, String> {
    let mut cursor = Cursor::new(payload);
    let allow_group = cursor
        .read_u8()
        .map_err(|e| format!("Failed to read allow_group: {}", e))?
        != 0;
    Ok(SwitchGroup { allow_group })
}

#[cfg(feature = "client-parse")]
pub(crate) fn parse_group_members_map(payload: &[u8]) -> Result<GroupMembersMap, String> {
    let mut cursor = Cursor::new(payload);
    let count = cursor
        .read_i32::<LittleEndian>()
        .map_err(|e| format!("Failed to read member count: {}", e))?;
    let mut members = Vec::new();
    for _ in 0..count {
        members.push(read_dotnet_string(&mut cursor)?);
    }
    Ok(GroupMembersMap { members })
}

#[cfg(feature = "client-parse")]
pub(crate) fn parse_send_member_location(payload: &[u8]) -> Result<SendMemberLocation, String> {
    let mut cursor = Cursor::new(payload);
    let member_name = read_dotnet_string(&mut cursor)?;
    let x = cursor
        .read_i32::<LittleEndian>()
        .map_err(|e| format!("Failed to read x: {}", e))?;
    let y = cursor
        .read_i32::<LittleEndian>()
        .map_err(|e| format!("Failed to read y: {}", e))?;
    let location = Point { x, y };
    Ok(SendMemberLocation {
        member_name,
        location,
    })
}
