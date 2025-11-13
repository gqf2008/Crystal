// Connection Handler - 连接相关数据包处理
// 
// 处理连接、断开、心跳等基础网络事件

use mir2_shared::packets::{PacketHeader, Packet, server};
use mir2_shared::enums::ServerPacketIds;
use crate::network::handlers::{GameEvent, PacketHandler};
use std::io::Cursor;

/// Connection handler - processes connection-related packets
pub struct ConnectionHandler;

impl PacketHandler for ConnectionHandler {
    fn handle(&self, header: &PacketHeader, payload: &[u8]) -> Vec<GameEvent> {
        let mut events = Vec::new();
        let mut cursor = Cursor::new(payload);
        
        // Determine packet type by opcode (using ServerPacketIds)
        match header.opcode as u16 {
            x if x == ServerPacketIds::Connected as u16 => {
                if let Ok(_packet) = server::Connected::read_body(&mut cursor) {
                    events.push(GameEvent::Connected);
                    tracing::info!("✅ Connected to server");
                }
            }
            
            x if x == ServerPacketIds::ClientVersion as u16 => {
                if let Ok(packet) = server::ClientVersion::read_body(&mut cursor) {
                    events.push(GameEvent::ClientVersionResponse { result: packet.result });
                    if packet.result == 1 {
                        tracing::info!("✅ ClientVersion accepted by server");
                    } else {
                        tracing::error!("❌ ClientVersion rejected by server (wrong version)");
                    }
                }
            }
            
            x if x == ServerPacketIds::Disconnect as u16 => {
                if let Ok(packet) = server::Disconnect::read_body(&mut cursor) {
                    let reason = format!("Disconnect reason: {}", packet.reason);
                    events.push(GameEvent::Disconnected { reason: reason.clone() });
                    tracing::warn!("🔌 Disconnected: {}", reason);
                } else {
                    events.push(GameEvent::Disconnected { 
                        reason: "Server disconnected".to_string() 
                    });
                }
            }
            
            x if x == ServerPacketIds::KeepAlive as u16 => {
                if let Ok(_packet) = server::KeepAlive::read_body(&mut cursor) {
                    tracing::trace!("💓 KeepAlive received");
                }
            }
            
            _ => {
                tracing::warn!("⚠️ ConnectionHandler: Unknown opcode {:04X}", header.opcode);
                events.push(GameEvent::UnhandledPacket { opcode: header.opcode });
            }
        }
        
        events
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_connection_handler_creation() {
        let handler = ConnectionHandler;
        assert!(handler.handle(&PacketHeader::new(4, 0x0001), &[]).len() > 0);
    }
}
