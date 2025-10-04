// Core modules - organized to match C# Client structure
mod error;
mod version;
mod settings;
mod key_bind_settings; // Renamed from keybinds
mod program;           // Renamed from runtime
mod app;               // Main egui application

// Main functional modules (matching C# Client directory structure)
mod forms;       // ← Client/Forms/
mod controls;    // ← Client/MirControls/ (renamed from ui)
mod graphics;    // ← Client/MirGraphics/
// mod map;      // ← 已废弃，功能移至 objects/map_code.rs
mod network;     // ← Client/MirNetwork/ (protocol, network moved here)
mod objects;     // ← Client/MirObjects/ (包含 map_code.rs)
mod scenes;      // ← Client/MirScenes/ (state moved here)
mod sounds;      // ← Client/MirSounds/ (renamed from audio)
mod resolution;  // ← Client/Resolution/
mod utils;       // ← Client/Utils/

use anyhow::Result;
use settings::ClientSettings;
use app::MirClientApp;
use network::{NetworkManager, network_task};
use std::sync::Arc;
use parking_lot::RwLock;

fn main() -> Result<()> {
    // Initialize tracing
    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| tracing_subscriber::EnvFilter::new("info"))
        )
        .init();

    // Parse command line arguments
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

    // Load settings
    let use_test_config = cli_flag || env_flag;
    let settings = ClientSettings::load(use_test_config, None)?;

    tracing::info!("Starting Legend of Mir 2 - Rust Edition");
    tracing::info!("Using test config: {}", use_test_config);
    
    // Create event channel for game events (network -> UI)
    let (event_tx, event_rx) = tokio::sync::mpsc::unbounded_channel();
    
    // Create command channel for network commands (UI -> network)
    let (command_tx, command_rx) = tokio::sync::mpsc::unbounded_channel();
    
    // Create network manager
    let settings_arc = Arc::new(RwLock::new(settings.clone()));
    let network_manager = NetworkManager::new(settings_arc.clone(), event_tx, command_rx);
    let game_client = network_manager.game_client();
    
    // Spawn network task in background
    std::thread::spawn(move || {
        let rt = tokio::runtime::Runtime::new().expect("Failed to create tokio runtime");
        rt.block_on(network_task(network_manager));
    });
    
    tracing::info!("Network task started in background");

    // Configure window options
    let native_options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_inner_size([1024.0, 768.0])
            .with_title("Legend of Mir 2"),
        ..Default::default()
    };

    // Run the application
    eframe::run_native(
        "mir2_client",
        native_options,
        Box::new(move |cc| Ok(Box::new(MirClientApp::new(cc, settings, game_client, event_rx, command_tx)))),
    ).map_err(|e| anyhow::anyhow!("Failed to run application: {}", e))
}
