// Network resources for Bevy integration
// Manages global NetworkManager instance and communication channels

use bevy::prelude::*;
use tokio::sync::mpsc;
use std::sync::Arc;
use parking_lot::RwLock;

use crate::network::{NetworkManager, GameEvent, NetworkCommand, GameClient};
use crate::settings::ClientSettings;

/// Global network command sender resource
/// This can be cloned and shared across multiple scenes
#[derive(Resource, Clone)]
pub struct NetworkCommandSender {
    pub tx: mpsc::UnboundedSender<NetworkCommand>,
}

/// Global network event receiver resource
/// Receives events from the network thread (like server responses)
#[derive(Resource)]
pub struct NetworkEventReceiver {
    pub rx: mpsc::UnboundedReceiver<GameEvent>,
}

/// Global NetworkManager handle resource
/// Allows access to the NetworkManager from Bevy systems
#[derive(Resource)]
pub struct NetworkManagerHandle {
    /// Shared reference to GameClient for accessing game state
    pub game_client: Arc<RwLock<GameClient>>,
    
    /// Handle to the network task
    pub task_handle: Option<tokio::task::JoinHandle<()>>,
}

/// Initialize the global NetworkManager
/// This should be called once during application startup
pub fn init_global_network_manager(
    mut commands: Commands,
    settings: Res<crate::bevy::GameConfig>,
) {
    info!("🌐 Initializing global NetworkManager...");
    info!("⚠️ NetworkManager 暂时禁用 (需要 Tokio 运行时)");
    
    // TODO: 修复 Tokio 运行时集成
    // 当前问题: "there is no reactor running"
    // 解决方案: 使用 bevy_tokio 或 bevy_async_task
    
    return; // 暂时禁用
    
    // Create communication channels
    let (event_tx, event_rx) = mpsc::unbounded_channel::<GameEvent>();
    let (command_tx, command_rx) = mpsc::unbounded_channel::<NetworkCommand>();
    
    // Create client settings (use default for now, can be loaded from config)
    let client_settings = Arc::new(RwLock::new(ClientSettings::default()));
    
    // Create NetworkManager
    let mut network_manager = NetworkManager::new(
        client_settings.clone(),
        event_tx,
        command_rx,
    );
    
    // Get GameClient reference before moving NetworkManager
    let game_client = network_manager.game_client();
    
    // Spawn network task
    let task_handle = tokio::spawn(async move {
        info!("🚀 NetworkManager task started");
        
        // Try to connect initially (optional, can be triggered by UI command)
        // if let Err(e) = network_manager.connect().await {
        //     error!("❌ Initial connection failed: {}", e);
        // }
        
        // Network manager main loop
        loop {
            // Process commands and packets
            if let Err(e) = network_manager.process().await {
                error!("❌ NetworkManager error: {}", e);
                // Continue running even on errors
            }
            
            // Small delay to prevent busy-waiting (~60 FPS)
            tokio::time::sleep(tokio::time::Duration::from_millis(16)).await;
        }
    });
    
    // Insert resources into Bevy world
    commands.insert_resource(NetworkCommandSender { tx: command_tx });
    commands.insert_resource(NetworkEventReceiver { rx: event_rx });
    commands.insert_resource(NetworkManagerHandle {
        game_client,
        task_handle: Some(task_handle),
    });
    
    info!("✅ Global NetworkManager initialized");
}

/// System to process network events
/// This should run in Update to handle server responses
pub fn process_network_events(
    event_rx: Option<ResMut<NetworkEventReceiver>>,
) {
    // Skip if NetworkManager is not initialized
    let Some(mut event_rx) = event_rx else {
        return;
    };
    
    // Process all pending events
    while let Ok(event) = event_rx.rx.try_recv() {
        match &event {
            GameEvent::Connected => {
                info!("🟢 Connected to server");
            }
            GameEvent::Disconnected { reason } => {
                warn!("🔴 Disconnected from server: {}", reason);
            }
            GameEvent::LoginSuccess { characters } => {
                info!("✅ Login successful, {} characters", characters.len());
            }
            GameEvent::LoginResponse { result } => {
                info!("📨 Login response: result={}", result);
            }
            GameEvent::NewCharacterSuccess { character } => {
                info!("✅ New character created: {}", character.name);
            }
            GameEvent::NewCharacterResponse { result } => {
                info!("📨 New character response: result={}", result);
            }
            GameEvent::DeleteCharacterSuccess { character_index } => {
                info!("✅ Character deleted: index={}", character_index);
            }
            GameEvent::DeleteCharacterResponse { result } => {
                info!("📨 Delete character response: result={}", result);
            }
            GameEvent::StartGameResponse { result } => {
                info!("📨 Start game response: result={}", result);
            }
            GameEvent::StartGameBanned { reason, expiry_date } => {
                error!("❌ Start game banned: reason={}, expiry={}", reason, expiry_date);
            }
            GameEvent::StartGameDelay { milliseconds } => {
                info!("⏱️ Start game delay: {}ms", milliseconds);
            }
            _ => {
                // Handle other events as needed
                debug!("📨 Network event: {:?}", event);
            }
        }
        
        // TODO: Send events to appropriate scenes via Bevy events
        // For now, just log them
    }
}

/// Cleanup network resources on app exit
pub fn cleanup_network_manager(
    mut handle: ResMut<NetworkManagerHandle>,
) {
    info!("🧹 Cleaning up NetworkManager...");
    
    if let Some(task_handle) = handle.task_handle.take() {
        task_handle.abort();
        info!("✅ NetworkManager task stopped");
    }
}
