// MirSounds - Sound and music playback system
// Mirrors Client.MirSounds

use anyhow::Result;

pub mod sound_list;
// pub mod sound_loader;  // TODO: Legacy, to be replaced by libraries
pub mod sound_manager;
pub mod libraries;

// Re-export commonly used items
pub use sound_list::{SoundId, load_sound_list, generate_filename};
pub use sound_manager::SoundManager;

// Legacy exports for compatibility (disabled for now)
// pub use sound_loader::{SoundType, SoundInfo};

// Re-export libraries
pub use libraries::{CachedSound, SoundLibrary, OneShotProvider, LoopProvider};
