//! Protocol Module - Modularized Structure
//!
//! This module contains all network protocol definitions for client-server communication.
//! The original monolithic protocol.rs (5300+ lines) has been refactored into logical sub-modules:
//!
//! - `packets/` - All packet definitions organized by system
//!   - `npc.rs` - NPC interaction packets (9 packets)
//!   - `magic.rs` - Magic/spell system packets (4 packets)
//!   - `item.rs` - Item management packets (10 packets)
//!   - `player.rs` - Player status packets (8 packets)
//!   - `object.rs` - Object status packets (4 packets)
//!   - `group.rs` - Group/party packets (3 packets)
//!   - `guild.rs` - Guild management packets (3 packets)
//!   - `hero.rs` - Hero system packets (5 packets)
//!   - `quest.rs` - Quest system packets (2 packets)
//!   - `account.rs` - Account/character management packets (4 packets)
//!
//! ## Migration Status
//!
//! ✅ **Phase 1 Complete** - 51 new packets modularized (10 modules created)
//! ⏳ **Phase 2 Pending** - Remaining legacy packets to be migrated
//!
//! The new modular structure makes it easier to:
//! - Find and modify specific packet types
//! - Add new packets to appropriate categories
//! - Review and test changes in isolation
//! - Collaborate without merge conflicts

use crate::client_data::SelectInfo;

// Sub-modules
pub mod packets;

// Type alias for compatibility with ClientRust naming
pub type CharacterSummary = SelectInfo;

// Re-export all packet types at the root level for backward compatibility
// This allows existing code to continue using `crate::protocol::NPCSell`
// without needing to change to `crate::protocol::packets::npc::NPCSell`
pub use packets::*;
