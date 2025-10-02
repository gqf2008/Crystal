// Game modules - mirrors Client/ structure
pub mod objects; // MirObjects/
pub mod scenes; // MirScenes/

// Re-exports
pub use objects::*;
pub use scenes::*;

#[path = "../protocol.rs"]
pub mod protocol;
#[path = "../state.rs"]
pub mod state;
