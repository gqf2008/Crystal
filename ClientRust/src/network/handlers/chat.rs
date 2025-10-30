// Chat Handler - 聊天相关数据包处理

use mir2_shared::packets::{PacketHeader, Packet, server};
use mir2_shared::enums::{ServerPacketIds, ChatType};
use crate::network::handlers::{GameEvent, PacketHandler};
use std::io::Cursor;

pub struct ChatHandler;

impl ChatHandler {
    pub fn new() -> Self {
        Self
    }
}

impl PacketHandler for ChatHandler {
    fn handle(&self, header: &PacketHeader, payload: &[u8]) -> Vec<GameEvent> {
        let mut events = Vec::new();
        let mut cursor = Cursor::new(payload);
        
        match header.opcode as u16 {
            // Chat message
            x if x == ServerPacketIds::Chat as u16 => {
                if let Ok(packet) = server::Chat::read_body(&mut cursor) {
                    events.push(GameEvent::ChatMessage {
                        sender: String::new(), // Server chat has no sender
                        message: packet.message.clone(),
                        chat_type: ChatType::System,
                    });
                    tracing::trace!("💬 Chat: {}", packet.message);
                }
            }
            
            // ObjectChat - chat from another player/NPC
            x if x == ServerPacketIds::ObjectChat as u16 => {
                if let Ok(packet) = server::ObjectChat::read_body(&mut cursor) {
                    events.push(GameEvent::ChatMessage {
                        sender: format!("Object {}", packet.object_id),  // ObjectChat没有name字段，只有object_id
                        message: packet.text.clone(),
                        chat_type: packet.chat_type,
                    });
                    tracing::trace!("💬 Object {}: {}", packet.object_id, packet.text);
                }
            }
            
            _ => {
                events.push(GameEvent::UnhandledPacket { opcode: header.opcode });
            }
        }
        
        events
    }
}

impl Default for ChatHandler {
    fn default() -> Self {
        Self::new()
    }
}
