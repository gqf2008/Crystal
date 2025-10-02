// Network module - Client networking functionality
// Corresponds to: Client/MirNetwork/

pub mod network;
pub mod protocol;

// Re-exports for convenience
pub use network::Network;
pub use protocol::{ServerMessage, ServerPacketId};
