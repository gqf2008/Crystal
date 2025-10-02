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
use mir2_shared::packets::{serialize_packet, PacketHeader, PacketMessage};
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
            timeout_time: Instant::now() + Duration::from_millis(settings.timeout),
            retry_time: Instant::now() + Duration::from_secs(5),
            connect_attempt: 0,
            max_attempts: 20,
            timeout_duration: Duration::from_millis(settings.timeout),
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
    pub fn enqueue<P: PacketMessage>(&mut self, packet: &P) -> Result<()> {
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
                self.receive_queue.push_back(NetworkEvent::Disconnected);
                self.disconnect();
            } else if now >= self.retry_time {
                // Retry connection
                self.retry_time = now + Duration::from_secs(5);
                let _ = self.connect(settings).await; // Ignore errors, will retry
            }
            return Ok(());
        }

        // Check connection timeout (5 seconds after connecting)
        if let Some(connected_time) = self.time_connected {
            if now > connected_time + Duration::from_secs(5) && !self.is_connected() {
                self.disconnect();
                let _ = self.connect(settings).await;
                return Ok(());
            }
        }

        // Receive data
        if let Some(stream) = &mut self.stream {
            self.receive_data(stream).await?;
        }

        // Send keepalive if needed
        if now > self.timeout_time && self.send_queue.is_empty() {
            // Queue KeepAlive packet
            // TODO: Implement KeepAlive packet type
            // self.enqueue(&KeepAlive)?;
        }

        // Send queued packets
        if !self.send_queue.is_empty() {
            if let Some(stream) = &mut self.stream {
                self.send_data(stream).await?;
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
                // Connection closed
                self.disconnect();
                return Ok(());
            }
            Ok(n) => {
                // Data received
                self.raw_data.extend_from_slice(&buf[..n]);
                self.bytes_received += n as u64;
                
                // Process complete packets from buffer
                self.process_received_data()?;
            }
            Err(ref e) if e.kind() == std::io::ErrorKind::WouldBlock => {
                // No data available right now, that's ok
            }
            Err(e) => {
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
                return Err(anyhow!("Invalid packet length: {}", length));
            }

            // Wait for complete packet
            if self.raw_data.len() < length {
                break;
            }

            // Extract packet
            let opcode = LittleEndian::read_i16(&self.raw_data[2..4]);
            let payload = self.raw_data[4..length].to_vec();
            
            // Remove processed data
            self.raw_data.drain(..length);

            // Queue event
            let header = PacketHeader::new(length as u16, opcode);
            self.receive_queue.push_back(NetworkEvent::ServerPacket {
                header,
                payload,
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
}
