// Network module - Client networking functionality
// Corresponds to: Client/MirNetwork/

pub mod network;
pub mod protocol;

// Re-exports for convenience
pub use network::{NetworkStack, NetworkEvent};
pub use protocol::*; // Re-export all protocol types for easy access
