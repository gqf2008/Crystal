// Network module - Client networking functionality
// Corresponds to: Client/MirNetwork/

pub mod network;
pub mod protocol;
pub mod game_client;
pub mod examples;

// Re-exports for convenience
pub use network::{NetworkStack, NetworkEvent};
pub use game_client::{GameClient, new_shared_client, GameEvent};

// Note: SharedGameClient and dispatch_packet are available but not re-exported
// to avoid unused import warnings. Use them via game_client:: and protocol:: if needed.
