// Core modules - organized to match C# Client structure
mod error;
mod version;
mod settings;
mod key_bind_settings; // Renamed from keybinds
mod program;           // Renamed from runtime

// Main functional modules (matching C# Client directory structure)
mod forms;       // ← Client/Forms/
mod controls;    // ← Client/MirControls/ (renamed from ui)
mod graphics;    // ← Client/MirGraphics/
mod network;     // ← Client/MirNetwork/ (protocol, network moved here)
mod objects;     // ← Client/MirObjects/
mod scenes;      // ← Client/MirScenes/ (state moved here)
mod sounds;      // ← Client/MirSounds/ (renamed from audio)
mod resolution;  // ← Client/Resolution/
mod utils;       // ← Client/Utils/

// Legacy game module (will be gradually migrated into above modules)
mod game;

use anyhow::Result;
use program::ClientRuntime;

fn main() -> Result<()> {
    let args: Vec<String> = std::env::args().collect();
    let cli_flag = args
        .iter()
        .skip(1)
        .any(|arg| matches!(arg.to_ascii_lowercase().as_str(), "-tc" | "--test-config"));

    let env_flag = std::env::var("MIR2_CLIENT_USE_TEST_CONFIG")
        .ok()
        .map(|value| {
            let normalized = value.trim().to_ascii_lowercase();
            matches!(normalized.as_str(), "1" | "true" | "yes" | "on")
        })
        .unwrap_or(false);

    ClientRuntime::bootstrap(cli_flag || env_flag)
}
