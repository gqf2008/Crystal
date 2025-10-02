//! Packets Module
//!
//! This module contains all network protocol packet definitions and infrastructure.
//! 
//! ## Structure
//! - `base`: Core packet infrastructure (PacketHeader, PacketMessage trait, serialization)
//! - `ids`: Packet ID enumerations (ClientPacketId, ServerPacketId)
//! - `client`: Packets sent from client to server
//! - `server`: Packets sent from server to client

use crate::data::client_data::SelectInfo;

// Core infrastructure
pub mod base;

// Packet definitions
pub mod client;
pub mod server;

// Type alias for compatibility
pub type CharacterSummary = SelectInfo;

// Re-export packet infrastructure for convenience
pub use base::*;
// pub use ids::*;

// Re-export all packet types for convenience
pub use client::*;
pub use server::*;
