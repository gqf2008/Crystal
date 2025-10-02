//! Client Packets Module
//!
//! Packets sent from client to server.

pub mod account;
pub mod character;
pub mod chat;
pub mod combat;
pub mod connection;
pub mod item;
pub mod movement;

// Re-export all packet types for convenience
pub use account::*;
pub use character::*;
pub use chat::*;
pub use combat::*;
pub use connection::*;
pub use item::*;
pub use movement::*;
