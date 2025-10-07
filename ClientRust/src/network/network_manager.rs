// Network manager - Coordinates GameClient with NetworkStack
// Handles connection lifecycle and packet dispatching

use std::sync::Arc;
use anyhow::Result;
use parking_lot::RwLock;
use tokio::sync::mpsc;

use crate::network::{NetworkStack, GameClient, GameEvent, NetworkCommand};
use crate::network::protocol::{dispatch_packet, PacketHeader};
use crate::settings::ClientSettings;

/// Network manager that coordinates networking and game client
pub struct NetworkManager {
    /// Network stack for TCP connection
    network: NetworkStack,
    
    /// Game client for packet handling
    game_client: Arc<RwLock<GameClient>>,
    
    /// Event sender to UI layer
    event_tx: mpsc::UnboundedSender<GameEvent>,
    
    /// Command receiver from UI layer
    command_rx: mpsc::UnboundedReceiver<NetworkCommand>,
    
    /// Settings
    settings: Arc<RwLock<ClientSettings>>,
}

impl NetworkManager {
    /// Create new network manager
    pub fn new(
        settings: Arc<RwLock<ClientSettings>>,
        event_tx: mpsc::UnboundedSender<GameEvent>,
        command_rx: mpsc::UnboundedReceiver<NetworkCommand>,
    ) -> Self {
        let network_settings = settings.read().network.clone();
        let network = NetworkStack::new(&network_settings);
        
        let mut game_client = GameClient::new();
        game_client.set_event_channel(event_tx.clone());
        
        Self {
            network,
            game_client: Arc::new(RwLock::new(game_client)),
            event_tx,
            command_rx,
            settings,
        }
    }
    
    /// Get shared reference to game client
    pub fn game_client(&self) -> Arc<RwLock<GameClient>> {
        self.game_client.clone()
    }
    
    /// Connect to server
    pub async fn connect(&mut self) -> Result<()> {
        let network_settings = self.settings.read().network.clone();
        
        tracing::info!(
            "Connecting to server: {}:{}",
            network_settings.ip_address,
            network_settings.port
        );
        
        self.network.connect(&network_settings).await?;
        
        tracing::info!("Connected to server (attempt {})", self.network.connect_attempt());
        
        Ok(())
    }
    
    /// Disconnect from server
    pub fn disconnect(&mut self) {
        tracing::info!("Disconnecting from server");
        self.network.disconnect();
        
        // Send disconnect event
        let _ = self.event_tx.send(GameEvent::Disconnected {
            reason: "User disconnected".to_string(),
        });
    }
    
    /// Check if connected
    pub fn is_connected(&self) -> bool {
        self.network.is_connected()
    }
    
    /// Send a packet to server
    pub fn send_packet<P: mir2_shared::packets::Packet>(&mut self, packet: &P) -> Result<()> {
        self.network.enqueue(packet)?;
        tracing::debug!("Enqueued packet: {}", std::any::type_name::<P>());
        Ok(())
    }
    
    /// Process commands from UI thread
    fn process_commands(&mut self) {
        // Process all pending commands
        while let Ok(command) = self.command_rx.try_recv() {
            if let Err(e) = self.handle_command(command) {
                tracing::error!("Failed to handle network command: {}", e);
            }
        }
    }
    
    /// Handle a single command
    fn handle_command(&mut self, command: NetworkCommand) -> Result<()> {
        use mir2_shared::packets::client;
        
        match command {
            NetworkCommand::Login { username, password } => {
                tracing::info!("Handling login command for user: {}", username);
                let packet = client::Login {
                    account_id: username,
                    password,
                };
                self.send_packet(&packet)?;
            }
            
            NetworkCommand::NewAccount { account_id, password, email, username, secret_question, secret_answer } => {
                tracing::info!("Handling new account command");
                let packet = client::NewAccount {
                    account_id,
                    password,
                    email_address: email,
                    user_name: username,
                    secret_question,
                    secret_answer,
                    birth_date_binary: 0, // TODO: Get actual birthdate
                };
                self.send_packet(&packet)?;
            }
            
            NetworkCommand::ChangePassword { account_id, current_password, new_password } => {
                tracing::info!("Handling change password command");
                let packet = client::ChangePassword {
                    account_id,
                    current_password,
                    new_password,
                };
                self.send_packet(&packet)?;
            }
            
            NetworkCommand::SelectCharacter { index } => {
                tracing::info!("Handling select character command: index={}", index);
                // TODO: Send SelectCharacter packet when available in SharedRust
            }
            
            NetworkCommand::NewCharacter { name, class, gender } => {
                tracing::info!("Handling new character command: name={}, class={}, gender={}", name, class, gender);
                let class_enum = mir2_shared::enums::MirClass::try_from(class).unwrap_or(mir2_shared::enums::MirClass::Warrior);
                let gender_enum = mir2_shared::enums::MirGender::try_from(gender).unwrap_or(mir2_shared::enums::MirGender::Male);
                tracing::info!("📦 Sending NewCharacter: name={}, class={:?} ({}), gender={:?} ({})", 
                    name, class_enum, class_enum as u8, gender_enum, gender_enum as u8);
                let packet = client::NewCharacter {
                    name,
                    class: class_enum,
                    gender: gender_enum,
                };
                self.send_packet(&packet)?;
            }
            
            NetworkCommand::DeleteCharacter { index } => {
                tracing::info!("Handling delete character command: index={}", index);
                let packet = client::DeleteCharacter {
                    character_index: index,
                };
                self.send_packet(&packet)?;
            }
            
            NetworkCommand::StartGame { character_index } => {
                tracing::info!("Handling start game command: character_index={}", character_index);
                let packet = client::StartGame {
                    character_index,
                };
                self.send_packet(&packet)?;
            }
            
            NetworkCommand::Disconnect => {
                tracing::info!("Handling disconnect command");
                self.disconnect();
            }
        }
        
        Ok(())
    }
    
    /// Process network I/O and handle received packets
    pub async fn process(&mut self) -> Result<()> {
        let network_settings = self.settings.read().network.clone();
        
        // Process commands from UI
        self.process_commands();
        
        // Process network I/O
        self.network.process(&network_settings).await?;
        
        // Process received packets
        while let Some(event) = self.network.poll_event() {
            match event {
                crate::network::NetworkEvent::Connected => {
                    tracing::info!("Network connected event");
                    let _ = self.event_tx.send(GameEvent::Connected);
                    
                    // Send ClientVersion packet
                    self.send_client_version()?;
                }
                crate::network::NetworkEvent::Disconnected => {
                    tracing::warn!("Network disconnected event");
                    let _ = self.event_tx.send(GameEvent::Disconnected {
                        reason: "Connection lost".to_string(),
                    });
                }
                crate::network::NetworkEvent::ServerPacket { header, payload } => {
                    // Dispatch packet to GameClient
                    self.dispatch_server_packet(header, &payload);
                }
                crate::network::NetworkEvent::Error(err) => {
                    tracing::error!("Network error: {}", err);
                    let _ = self.event_tx.send(GameEvent::SystemMessage {
                        message: format!("Network error: {}", err),
                    });
                }
            }
        }
        
        Ok(())
    }
    
    /// Send ClientVersion packet to server
    fn send_client_version(&mut self) -> Result<()> {
        use mir2_shared::packets::client::ClientVersion;
        
        // Calculate version hash from client binary
        let version_hash = match crate::version::client_binary_hash() {
            Ok(hash) => hash,
            Err(e) => {
                tracing::warn!("Failed to calculate client hash: {}, using default", e);
                vec![0u8; 16] // Use zeros as fallback
            }
        };
        
        let packet = ClientVersion {
            version_hash: version_hash.clone(),
        };
        
        tracing::info!("Sending ClientVersion: hash={}", 
            crate::version::hash_to_hex(&version_hash));
        self.send_packet(&packet)?;
        
        Ok(())
    }
    
    /// Dispatch server packet to GameClient
    /// 
    /// # 参数
    /// - `header`: 包头信息 (length + opcode)
    /// - `payload`: 完整的包数据,包括4字节头部 [length(2)][opcode(2)][body...]
    fn dispatch_server_packet(&self, header: mir2_shared::packets::PacketHeader, payload: &[u8]) {
        let mut client = self.game_client.write();
        
        // Convert mir2_shared::PacketHeader to protocol::PacketHeader
        let protocol_header = PacketHeader {
            length: header.length,
            opcode: header.opcode,
        };
        
        // Log packet details for debugging
        tracing::debug!(
            "📦 Received packet: opcode={}, length={}, payload_len={}",
            header.opcode,
            header.length,
            payload.len()
        );
        
        // Dispatch packet using protocol module
        if let Err(e) = dispatch_packet(protocol_header, payload, &mut *client) {
            tracing::error!(
                "❌ Failed to dispatch packet (opcode={}, length={}, payload_len={}): {}",
                header.opcode,
                header.length,
                payload.len(),
                e
            );
        }
    }
}

/// Background network task that runs continuously
pub async fn network_task(mut manager: NetworkManager) {
    tracing::info!("Network task started");
    
    // Try to connect initially
    if let Err(e) = manager.connect().await {
        tracing::error!("Initial connection failed: {}", e);
    }
    
    // Main network loop
    loop {
        if let Err(e) = manager.process().await {
            tracing::error!("Network process error: {}", e);
        }
        
        // Small delay to prevent busy-waiting
        tokio::time::sleep(tokio::time::Duration::from_millis(16)).await; // ~60 FPS
    }
}
