// Network module - Client networking functionality
// Corresponds to: Client/MirNetwork/Network.cs
//
// This module handles TCP connection, packet sending/receiving,
// and maintains connection state similar to the C# client implementation.

use std::collections::VecDeque;
use std::net::SocketAddr;
use std::time::{Duration, Instant};

use anyhow::{anyhow, Context, Result};
use byteorder::{ByteOrder, LittleEndian};
use mir2_shared::packets::{serialize_packet, PacketHeader, Packet};
use mir2_shared::packets::client; // For KeepAlive
use tokio::io::AsyncWriteExt;
use tokio::net::TcpStream;

use crate::settings::NetworkSettings;

/// Network events that can be received from the server
#[derive(Debug, Clone)]
pub enum NetworkEvent {
    Connected,
    Disconnected,
    ServerPacket {
        header: PacketHeader,
        payload: Vec<u8>,
    },
    Error(String),
}

/// Connection state
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum ConnectionState {
    Disconnected,
    Connecting,
    Connected,
}

/// Network stack for client-server communication
/// 
/// Mirrors the functionality of C# Client/MirNetwork/Network.cs:
/// - Maintains TCP connection
/// - Queues outgoing packets
/// - Processes incoming packets
/// - Handles connection timeouts and retries
pub struct NetworkStack {
    state: ConnectionState,
    stream: Option<TcpStream>,
    
    // Packet queues (similar to C# ConcurrentQueue)
    receive_queue: VecDeque<NetworkEvent>,
    send_queue: VecDeque<Vec<u8>>,
    
    // Buffer for incoming data
    raw_data: Vec<u8>,
    
    // Connection timing
    time_connected: Option<Instant>,
    timeout_time: Instant,
    retry_time: Instant,
    
    // Settings
    connect_attempt: u32,
    max_attempts: u32,
    timeout_duration: Duration,
    
    // Statistics
    pub bytes_sent: u64,
    pub bytes_received: u64,
}

impl NetworkStack {
    /// Create a new NetworkStack instance
    pub fn new(settings: &NetworkSettings) -> Self {
        Self {
            state: ConnectionState::Disconnected,
            stream: None,
            receive_queue: VecDeque::new(),
            send_queue: VecDeque::new(),
            raw_data: Vec::new(),
            time_connected: None,
            timeout_time: Instant::now() + Duration::from_millis(settings.timeout_ms),
            retry_time: Instant::now() + Duration::from_secs(5),
            connect_attempt: 0,
            max_attempts: 20,
            timeout_duration: Duration::from_millis(settings.timeout_ms),
            bytes_sent: 0,
            bytes_received: 0,
        }
    }

    /// Check if currently connected to server
    pub fn is_connected(&self) -> bool {
        self.state == ConnectionState::Connected
    }

    /// Get current connection attempt number
    pub fn connect_attempt(&self) -> u32 {
        self.connect_attempt
    }

    /// Attempt to connect to the server
    /// 
    /// Corresponds to: Network.Connect()
    pub async fn connect(&mut self, settings: &NetworkSettings) -> Result<()> {
        if self.connect_attempt >= self.max_attempts {
            return Err(anyhow!(
                "Maximum connection attempts reached: {}",
                self.max_attempts
            ));
        }

        self.connect_attempt += 1;
        self.state = ConnectionState::Connecting;

        let addr = format!("{}:{}", settings.ip_address, settings.port);
        let addr: SocketAddr = addr
            .parse()
            .with_context(|| format!("Failed to parse server address: {}", addr))?;

        match TcpStream::connect(addr).await {
            Ok(stream) => {
                stream.set_nodelay(true)?;
                self.stream = Some(stream);
                self.state = ConnectionState::Connected;
                self.time_connected = Some(Instant::now());
                self.timeout_time = Instant::now() + self.timeout_duration;
                self.raw_data.clear();
                
                self.receive_queue.push_back(NetworkEvent::Connected);
                
                Ok(())
            }
            Err(err) => {
                self.state = ConnectionState::Disconnected;
                Err(err.into())
            }
        }
    }

    /// Disconnect from server
    /// 
    /// Corresponds to: Network.Disconnect()
    pub fn disconnect(&mut self) {
        self.stream = None;
        self.state = ConnectionState::Disconnected;
        self.time_connected = None;
        self.send_queue.clear();
        
        if !self.receive_queue.is_empty() {
            self.receive_queue.push_back(NetworkEvent::Disconnected);
        }
    }

    /// Enqueue a packet to be sent
    /// 
    /// Corresponds to: Network.Enqueue(Packet p)
    pub fn enqueue<P: Packet>(&mut self, packet: &P) -> Result<()> {
        let mut buffer = Vec::new();
        serialize_packet(&mut buffer, packet)
            .map_err(|e| anyhow!("Failed to serialize packet: {}", e))?;
        
        self.send_queue.push_back(buffer);
        Ok(())
    }

    /// Process network I/O and connection state
    /// 
    /// Corresponds to: Network.Process()
    /// This should be called regularly from the main game loop
    pub async fn process(&mut self, settings: &NetworkSettings) -> Result<()> {
        let now = Instant::now();

        // Handle disconnected state
        if !self.is_connected() {
            if self.state == ConnectionState::Connected {
                // Just lost connection
                tracing::warn!("🔌 Connection lost - will retry in 5 seconds");
                self.receive_queue.push_back(NetworkEvent::Disconnected);
                self.disconnect();
            } else if now >= self.retry_time {
                // Retry connection
                tracing::info!("🔄 Attempting reconnect (attempt {})", self.connect_attempt + 1);
                self.retry_time = now + Duration::from_secs(5);
                match self.connect(settings).await {
                    Ok(_) => {
                        tracing::info!("✅ Reconnected successfully");
                    }
                    Err(e) => {
                        tracing::warn!("❌ Reconnect failed: {} - will retry", e);
                    }
                }
            }
            return Ok(());
        }

        // Check connection timeout (5 seconds after connecting)
        if let Some(connected_time) = self.time_connected {
            if now > connected_time + Duration::from_secs(5) && !self.is_connected() {
                tracing::warn!("⏱️ Connection timeout detected");
                self.disconnect();
                let _ = self.connect(settings).await;
                return Ok(());
            }
        }

        // Receive data
        if self.stream.is_some() {
            // Take ownership temporarily to avoid double borrow
            let mut stream = self.stream.take().unwrap();
            let result = self.receive_data(&mut stream).await;
            self.stream = Some(stream);
            result?;
        }

        // Send keepalive if needed
        if now > self.timeout_time && self.send_queue.is_empty() {
            // Queue KeepAlive packet with current timestamp
            let timestamp = std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_millis() as i64;
            let keepalive = client::KeepAlive { time: timestamp };
            self.enqueue(&keepalive)?;
            tracing::debug!("Sent KeepAlive packet to maintain connection");
        }

        // Send queued packets
        if !self.send_queue.is_empty() {
            if self.stream.is_some() {
                // Take ownership temporarily to avoid double borrow
                let mut stream = self.stream.take().unwrap();
                let result = self.send_data(&mut stream).await;
                self.stream = Some(stream);
                result?;
            }
            self.timeout_time = now + self.timeout_duration;
        }

        Ok(())
    }

    /// Receive and process incoming data
    /// 
    /// Corresponds to: Network.ReceiveData(IAsyncResult result)
    async fn receive_data(&mut self, stream: &mut TcpStream) -> Result<()> {
        let mut buf = vec![0u8; 8192];
        
        // Non-blocking read
        match stream.try_read(&mut buf) {
            Ok(0) => {
                // Connection closed by server
                tracing::warn!("🔌 Connection closed by server");
                self.disconnect();
                return Ok(());
            }
            Ok(n) => {
                // Data received
                self.raw_data.extend_from_slice(&buf[..n]);
                self.bytes_received += n as u64;
                tracing::trace!("📥 Received {} bytes (total: {})", n, self.bytes_received);
                
                // Process complete packets from buffer
                self.process_received_data()?;
            }
            Err(ref e) if e.kind() == std::io::ErrorKind::WouldBlock => {
                // No data available right now, that's ok
            }
            Err(e) => {
                tracing::error!("❌ Network receive error: {}", e);
                self.disconnect();
                return Err(e.into());
            }
        }
        
        Ok(())
    }

    /// Process received data buffer and extract complete packets
    fn process_received_data(&mut self) -> Result<()> {
        loop {
            // Need at least 4 bytes for header (2 bytes length + 2 bytes opcode)
            if self.raw_data.len() < 4 {
                break;
            }

            // Read packet length
            let length = LittleEndian::read_u16(&self.raw_data[0..2]) as usize;
            
            if length < 4 {
                tracing::error!("⚠️ Invalid packet length: {} - discarding buffer", length);
                self.raw_data.clear(); // 清除损坏的数据
                return Err(anyhow!("Invalid packet length: {}", length));
            }
            
            // 检测异常大的包 (可能是损坏的数据)
            if length > 65536 {
                tracing::error!("⚠️ Suspiciously large packet: {} bytes - may be corrupted", length);
            }

            // Wait for complete packet
            if self.raw_data.len() < length {
                break;
            }

            // Extract complete packet (including 4-byte header)
            let opcode = LittleEndian::read_i16(&self.raw_data[2..4]);
            let full_packet = self.raw_data[..length].to_vec();  // ✅ 包含完整数据: [length][opcode][body...]
            
            // Remove processed data
            self.raw_data.drain(..length);

            // Queue event
            let header = PacketHeader::new(length as u16, opcode);
            self.receive_queue.push_back(NetworkEvent::ServerPacket {
                header,
                payload: full_packet,  // payload包含完整的包数据(4字节头+包体)
            });
        }

        Ok(())
    }

    /// Send queued data to server
    /// 
    /// Corresponds to: Network.BeginSend(List<byte> data)
    async fn send_data(&mut self, stream: &mut TcpStream) -> Result<()> {
        while let Some(packet_data) = self.send_queue.pop_front() {
            stream.write_all(&packet_data).await?;
            self.bytes_sent += packet_data.len() as u64;
        }
        Ok(())
    }

    /// Get next network event from receive queue
    /// 
    /// Returns None if queue is empty
    pub fn poll_event(&mut self) -> Option<NetworkEvent> {
        self.receive_queue.pop_front()
    }

    /// Check if there are pending events
    pub fn has_events(&self) -> bool {
        !self.receive_queue.is_empty()
    }
    
    /// Get connection statistics
    pub fn stats(&self) -> NetworkStats {
        NetworkStats {
            bytes_sent: self.bytes_sent,
            bytes_received: self.bytes_received,
            packets_queued: self.send_queue.len(),
            events_queued: self.receive_queue.len(),
            connected: self.is_connected(),
        }
    }
}

/// Network statistics
#[derive(Debug, Clone, Copy)]
pub struct NetworkStats {
    pub bytes_sent: u64,
    pub bytes_received: u64,
    pub packets_queued: usize,
    pub events_queued: usize,
    pub connected: bool,
}

#[cfg(test)]
mod tests {
    use super::*;
    
    fn mock_settings() -> NetworkSettings {
        NetworkSettings {
            use_config: false,
            ip_address: "127.0.0.1".to_string(),
            port: 7000,
            timeout_ms: 5000,
        }
    }
    
    #[test]
    fn test_network_stack_creation() {
        let settings = mock_settings();
        let stack = NetworkStack::new(&settings);
        
        assert_eq!(stack.state, ConnectionState::Disconnected);
        assert!(!stack.is_connected());
        assert_eq!(stack.bytes_sent, 0);
        assert_eq!(stack.bytes_received, 0);
    }
    
    #[test]
    fn test_disconnected_state() {
        let settings = mock_settings();
        let mut stack = NetworkStack::new(&settings);
        
        assert!(!stack.is_connected());
        assert!(!stack.has_events());
        assert!(stack.poll_event().is_none());
    }
    
    #[test]
    fn test_packet_enqueue() {
        let settings = mock_settings();
        let mut stack = NetworkStack::new(&settings);
        
        // Create a KeepAlive packet
        let timestamp = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_millis() as i64;
        let keepalive = client::KeepAlive { time: timestamp };
        
        // Enqueue packet
        let result = stack.enqueue(&keepalive);
        assert!(result.is_ok());
        
        // Verify packet was queued
        assert_eq!(stack.send_queue.len(), 1);
    }
    
    #[test]
    fn test_multiple_packet_enqueue() {
        let settings = mock_settings();
        let mut stack = NetworkStack::new(&settings);
        
        // Enqueue multiple packets
        let timestamp = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_millis() as i64;
        for _ in 0..5 {
            let keepalive = client::KeepAlive { time: timestamp };
            stack.enqueue(&keepalive).unwrap();
        }
        
        assert_eq!(stack.send_queue.len(), 5);
    }
    
    #[test]
    fn test_disconnect() {
        let settings = mock_settings();
        let mut stack = NetworkStack::new(&settings);
        
        // Manually set connected state for testing
        stack.state = ConnectionState::Connected;
        stack.time_connected = Some(Instant::now());
        
        // Disconnect
        stack.disconnect();
        
        assert_eq!(stack.state, ConnectionState::Disconnected);
        assert!(stack.time_connected.is_none());
        assert!(stack.stream.is_none());
    }
    
    #[test]
    fn test_process_received_data_incomplete() {
        let settings = mock_settings();
        let mut stack = NetworkStack::new(&settings);
        
        // Add incomplete header (less than 4 bytes)
        stack.raw_data.extend_from_slice(&[0x01, 0x02]);
        
        // Should not process incomplete packet
        let result = stack.process_received_data();
        assert!(result.is_ok());
        assert_eq!(stack.raw_data.len(), 2); // Data remains in buffer
        assert_eq!(stack.receive_queue.len(), 0); // No events queued
    }
    
    #[test]
    fn test_process_received_data_invalid_length() {
        let settings = mock_settings();
        let mut stack = NetworkStack::new(&settings);
        
        // Add invalid packet (length = 0)
        let mut data = vec![0u8; 4];
        LittleEndian::write_u16(&mut data[0..2], 0); // Invalid length
        LittleEndian::write_i16(&mut data[2..4], 1); // Opcode
        stack.raw_data.extend_from_slice(&data);
        
        // Should return error for invalid length
        let result = stack.process_received_data();
        assert!(result.is_err());
    }
    
    #[test]
    fn test_process_received_data_complete_packet() {
        let settings = mock_settings();
        let mut stack = NetworkStack::new(&settings);
        
        // Create a valid packet
        let packet_length = 6u16; // 4 bytes header + 2 bytes payload
        let opcode = 100i16;
        let payload = vec![0xAA, 0xBB];
        
        let mut data = vec![0u8; packet_length as usize];
        LittleEndian::write_u16(&mut data[0..2], packet_length);
        LittleEndian::write_i16(&mut data[2..4], opcode);
        data[4..6].copy_from_slice(&payload);
        
        stack.raw_data.extend_from_slice(&data);
        
        // Process packet
        let result = stack.process_received_data();
        assert!(result.is_ok());
        
        // Verify packet was queued as event
        assert_eq!(stack.receive_queue.len(), 1);
        assert_eq!(stack.raw_data.len(), 0); // Buffer cleared
        
        // Check event contents
        if let Some(NetworkEvent::ServerPacket { header, payload: p }) = stack.poll_event() {
            assert_eq!(header.length, packet_length);
            assert_eq!(header.opcode, opcode);
            assert_eq!(p, payload);
        } else {
            panic!("Expected ServerPacket event");
        }
    }
    
    #[test]
    fn test_process_multiple_packets() {
        let settings = mock_settings();
        let mut stack = NetworkStack::new(&settings);
        
        // Add two complete packets to buffer
        for i in 0..2 {
            let packet_length = 5u16;
            let opcode = (100 + i) as i16;
            let payload = vec![i as u8];
            
            let mut data = vec![0u8; packet_length as usize];
            LittleEndian::write_u16(&mut data[0..2], packet_length);
            LittleEndian::write_i16(&mut data[2..4], opcode);
            data[4] = payload[0];
            
            stack.raw_data.extend_from_slice(&data);
        }
        
        // Process packets
        let result = stack.process_received_data();
        assert!(result.is_ok());
        
        // Verify both packets were queued
        assert_eq!(stack.receive_queue.len(), 2);
        assert_eq!(stack.raw_data.len(), 0);
    }
    
    #[test]
    fn test_keepalive_packet_serialization() {
        // Test that KeepAlive packet can be serialized
        let keepalive = client::KeepAlive {
            time: 1234567890,
        };
        
        let mut buffer = Vec::new();
        let result = serialize_packet(&mut buffer, &keepalive);
        assert!(result.is_ok());
        
        // Header (4 bytes) + time (8 bytes) = 12 bytes
        assert_eq!(buffer.len(), 12);
        
        // Verify opcode
        let opcode = LittleEndian::read_i16(&buffer[2..4]);
        assert_eq!(opcode, client::KeepAlive::OPCODE);
    }
    
    #[test]
    fn test_network_stats() {
        let settings = mock_settings();
        let mut stack = NetworkStack::new(&settings);
        
        // Set some values
        stack.bytes_sent = 1024;
        stack.bytes_received = 2048;
        stack.state = ConnectionState::Connected;
        
        // Enqueue some packets
        let keepalive = client::KeepAlive { time: 0 };
        stack.enqueue(&keepalive).unwrap();
        stack.enqueue(&keepalive).unwrap();
        
        // Add some events
        stack.receive_queue.push_back(NetworkEvent::Connected);
        
        let stats = stack.stats();
        assert_eq!(stats.bytes_sent, 1024);
        assert_eq!(stats.bytes_received, 2048);
        assert_eq!(stats.packets_queued, 2);
        assert_eq!(stats.events_queued, 1);
        assert!(stats.connected);
    }
    
    #[test]
    fn test_connection_state_transitions() {
        let settings = mock_settings();
        let mut stack = NetworkStack::new(&settings);
        
        // Initial state
        assert_eq!(stack.state, ConnectionState::Disconnected);
        
        // Simulate connecting
        stack.state = ConnectionState::Connecting;
        assert_eq!(stack.state, ConnectionState::Connecting);
        
        // Simulate connected
        stack.state = ConnectionState::Connected;
        assert!(stack.is_connected());
        
        // Disconnect
        stack.disconnect();
        assert_eq!(stack.state, ConnectionState::Disconnected);
        assert!(!stack.is_connected());
    }
}
