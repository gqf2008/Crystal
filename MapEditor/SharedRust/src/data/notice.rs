//! Notice System Module
//!
//! This module contains game announcement and notice functionality.
//!
//! ## Overview
//! The `Notice` structure is used by the server to send important announcements,
//! maintenance notifications, or event information to all connected clients.
//!
//! ## Usage
//! ```ignore
//! use mir2_shared::data::notice::Notice;
//!
//! let notice = Notice {
//!     title: "Server Maintenance".to_string(),
//!     message: "The server will be down for maintenance at 2:00 AM.".to_string(),
//! };
//! ```
//!
//! ## Serialization
//! Notices are serialized using .NET-compatible string encoding for client-server
//! communication compatibility.

use std::io::{Read, Write};

use serde::{Deserialize, Serialize};

use crate::binary::{read_dotnet_string, write_dotnet_string};
use crate::data::stats::SharedResult;

/// Notice/announcement information
///
/// Represents a game-wide announcement or notification that can be displayed
/// to players. Typically used for server maintenance notices, events, or
/// important system messages.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Notice {
    pub title: String,
    pub message: String,
}

impl Notice {
    pub fn read_from<R: Read>(reader: &mut R) -> SharedResult<Self> {
        let title = read_dotnet_string(reader)?;
        let message = read_dotnet_string(reader)?;

        Ok(Self { title, message })
    }

    pub fn write_to<W: Write>(&self, writer: &mut W) -> SharedResult<()> {
        write_dotnet_string(writer, &self.title)?;
        write_dotnet_string(writer, &self.message)?;
        Ok(())
    }
}
