// Character Handler - 角色相关数据包处理
// 
// 处理登录、角色创建、选择、删除等

use mir2_shared::packets::{PacketHeader, Packet, server};
use mir2_shared::enums::ServerPacketIds;
use crate::network::handlers::{GameEvent, PacketHandler};
use std::io::Cursor;

/// Character handler - processes character-related packets
pub struct CharacterHandler;

impl CharacterHandler {
    pub fn new() -> Self {
        Self
    }
}

impl PacketHandler for CharacterHandler {
    fn handle(&self, header: &PacketHeader, payload: &[u8]) -> Vec<GameEvent> {
        let mut events = Vec::new();
        let mut cursor = Cursor::new(payload);
        
        match header.opcode as u16 {
            // LoginSuccess
            x if x == ServerPacketIds::LoginSuccess as u16 => {
                if let Ok(_packet) = server::LoginSuccess::read_body(&mut cursor) {
                    events.push(GameEvent::LoginSuccess);
                    tracing::info!("✅ Login successful");
                }
            }
            
            // StartGameDelay
            x if x == ServerPacketIds::StartGameDelay as u16 => {
                if let Ok(packet) = server::StartGameDelay::read_body(&mut cursor) {
                    events.push(GameEvent::StartGame { delay: packet.milliseconds as i32 });
                    tracing::info!("🎮 Starting game with delay: {}ms", packet.milliseconds);
                }
            }
            
            // NewCharacter (character created)
            x if x == ServerPacketIds::NewCharacter as u16 => {
                if let Ok(_packet) = server::NewCharacter::read_body(&mut cursor) {
                    events.push(GameEvent::CharacterCreated { 
                        name: "New Character".to_string()  // NewCharacter只有result字段
                    });
                    tracing::info!("👤 Character creation response received");
                }
            }
            
            // DeleteCharacter (character deleted)
            x if x == ServerPacketIds::DeleteCharacter as u16 => {
                if let Ok(_packet) = server::DeleteCharacter::read_body(&mut cursor) {
                    events.push(GameEvent::CharacterDeleted { 
                        index: 0  // DeleteCharacter只有result字段
                    });
                    tracing::info!("🗑️ Character deletion response received");
                }
            }
            
            // UserInformation (player data after login)
            x if x == ServerPacketIds::UserInformation as u16 => {
                if let Ok(packet) = server::UserInformation::read_body(&mut cursor) {
                    // Generate multiple events for complete player state
                    events.push(GameEvent::PlayerLocationChanged {
                        x: packet.location_x,
                        y: packet.location_y,
                    });
                    
                    events.push(GameEvent::HealthChanged {
                        current: packet.hp as u32,
                        max: packet.hp as u32,  // UserInformation没有max_hp字段
                    });
                    
                    events.push(GameEvent::ManaChanged {
                        current: packet.mp as u32,
                        max: packet.mp as u32,  // UserInformation没有max_mp字段
                    });
                    
                    events.push(GameEvent::GoldChanged {
                        amount: packet.gold,
                    });
                    
                    tracing::info!("📊 User information loaded for player at ({}, {})", 
                        packet.location_x, packet.location_y);
                }
            }
            
            _ => {
                tracing::debug!("⚠️ CharacterHandler: Unhandled opcode {:04X}", header.opcode);
                events.push(GameEvent::UnhandledPacket { opcode: header.opcode });
            }
        }
        
        events
    }
}

impl Default for CharacterHandler {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_character_handler_creation() {
        let handler = CharacterHandler::new();
        // Test with a dummy packet
        let events = handler.handle(&PacketHeader::new(4, 0x0010), &[]);
        assert_eq!(events.len(), 1);
    }
}
