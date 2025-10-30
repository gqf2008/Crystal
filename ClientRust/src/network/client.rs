// 简单网络客户端
// 
// 设计：满足 Read + Write trait 即可，内部两个线程处理读写
// - read_thread: 持续读packet → 解析为 GameEvent → 发送到游戏线程
// - write_thread: 持续recv GameEvent → 转换为packet → 发送到服务器
// 
// 使用：
//   let (tx, rx) = Network::new(tcp_stream);
//   tx.send(GameEvent::LoginRequest {...});  // 游戏 → 网络
//   let events = rx.try_iter().collect();    // 网络 → 游戏

use std::io::{Read, Write};
use crossbeam_channel::{Sender, Receiver, unbounded};
use anyhow::Result;

use crate::network::handlers::GameEvent;

/// 简单网络客户端
pub struct Network;

impl Network {
    /// 创建并启动网络客户端
    /// 
    /// 返回：(发送channel, 接收channel)
    /// - 发送channel: 游戏 → 网络 (GameEvent)
    /// - 接收channel: 网络 → 游戏 (GameEvent)
    pub fn new<S>(stream: S) -> (Sender<GameEvent>, Receiver<GameEvent>)
    where
        S: Read + Write + Send + 'static,
    {
        let (game_to_net_tx, game_to_net_rx) = unbounded();
        let (net_to_game_tx, net_to_game_rx) = unbounded();
        
        // TcpStream 需要 clone 才能在两个线程中使用
        // 简化：要求 S 必须是可 clone 的（TcpStream 满足）
        let stream = std::sync::Arc::new(std::sync::Mutex::new(stream));
        
        // 读线程：packet → GameEvent
        {
            let stream = stream.clone();
            let tx = net_to_game_tx.clone();
            std::thread::Builder::new()
                .name("net-read".into())
                .spawn(move || {
                    read_loop(stream, tx);
                })
                .expect("Failed to spawn read thread");
        }
        
        // 写线程：GameEvent → packet
        {
            let stream = stream.clone();
            let rx = game_to_net_rx;
            std::thread::Builder::new()
                .name("net-write".into())
                .spawn(move || {
                    write_loop(stream, rx);
                })
                .expect("Failed to spawn write thread");
        }
        
        (game_to_net_tx, net_to_game_rx)
    }
}

/// 读线程：持续读取 packet 并转换为 GameEvent
fn read_loop<S: Read + Send>(
    stream: std::sync::Arc<std::sync::Mutex<S>>,
    tx: Sender<GameEvent>,
) {
    use mir2_shared::packets::PacketHeader;
    
    loop {
        // 读取 packet header (4 bytes: length + opcode)
        let header = {
            let mut stream = stream.lock().unwrap();
            match PacketHeader::read_from(&mut *stream) {
                Ok(h) => h,
                Err(e) => {
                    tracing::error!("Read header error: {}", e);
                    let _ = tx.send(GameEvent::Disconnected { reason: e.to_string() });
                    break;
                }
            }
        };
        
        // 读取 payload
        let payload_len = (header.length as usize).saturating_sub(PacketHeader::HEADER_SIZE);
        let mut payload = vec![0u8; payload_len];
        {
            let mut stream = stream.lock().unwrap();
            if let Err(e) = stream.read_exact(&mut payload) {
                tracing::error!("Read payload error: {}", e);
                let _ = tx.send(GameEvent::Disconnected { reason: e.to_string() });
                break;
            }
        }
        
        // 转换为 GameEvent（使用现有的 handlers）
        let events = dispatch_packet(&header, &payload);
        for event in events {
            if tx.send(event).is_err() {
                tracing::error!("Game thread disconnected");
                return;
            }
        }
    }
}

/// 写线程：持续接收 GameEvent 并转换为 packet
fn write_loop<S: Write + Send>(
    stream: std::sync::Arc<std::sync::Mutex<S>>,
    rx: Receiver<GameEvent>,
) {
    
    loop {
        // 接收 GameEvent
        let event = match rx.recv() {
            Ok(e) => e,
            Err(_) => {
                tracing::info!("Game thread closed, stopping write thread");
                return;
            }
        };
        
        // 转换为 packet 并发送
        if let Err(e) = handle_outgoing_event(&stream, event) {
            tracing::error!("Send packet error: {}", e);
            return;
        }
    }
}

/// 分发 packet 到 handler（复用现有逻辑）
fn dispatch_packet(header: &mir2_shared::packets::PacketHeader, _payload: &[u8]) -> Vec<GameEvent> {
    use crate::network::handlers::*;
    use mir2_shared::enums::ServerPacketIds;
    
    let opcode = header.opcode;
    
    // 简化：只处理核心 packet，其他返回 UnhandledPacket
    // TODO: 解析 payload 并转换为具体的 GameEvent
    match opcode {
        x if x == ServerPacketIds::Connected as i16 => {
            vec![GameEvent::Connected]
        }
        x if x == ServerPacketIds::Disconnect as i16 => {
            vec![GameEvent::Disconnected { reason: "Server closed".into() }]
        }
        _ => {
            tracing::warn!("Unhandled packet opcode: {}", opcode);
            vec![GameEvent::UnhandledPacket { opcode }]
        }
    }
}

/// 处理出站事件（复用现有逻辑）
fn handle_outgoing_event<S: Write>(
    stream: &std::sync::Arc<std::sync::Mutex<S>>,
    event: GameEvent,
) -> Result<()> {
    use mir2_shared::packets::{client, serialize_packet};
    
    let mut stream = stream.lock().unwrap();
    
    match event {
        GameEvent::LoginRequest { username, password } => {
            let packet = client::Login { account_id: username, password };
            serialize_packet(&mut *stream, &packet)?;
        }
        GameEvent::WalkRequest { direction } => {
            let packet = client::movement::Walk { direction };
            serialize_packet(&mut *stream, &packet)?;
        }
        GameEvent::DisconnectRequest => {
            return Ok(()); // 直接关闭连接
        }
        _ => {
            tracing::warn!("Unhandled outgoing event: {:?}", event);
            return Ok(());
        }
    }
    
    stream.flush()?;
    Ok(())
}
