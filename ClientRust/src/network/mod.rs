// Network module - Client networking functionality
// Corresponds to: Client/MirNetwork/

pub mod network;
pub mod protocol;
pub mod game_client;
pub mod network_manager;
pub mod network_command;

// Re-exports for convenience
pub use network::{NetworkStack, NetworkEvent};
pub use game_client::{GameClient, new_shared_client, GameEvent};
pub use network_manager::{NetworkManager, network_task};
pub use network_command::NetworkCommand;

// Note: SharedGameClient and dispatch_packet are available but not re-exported
// to avoid unused import warnings. Use them via game_client:: and protocol:: if needed.
