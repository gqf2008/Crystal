// Character Handler - 角色相关数据包处理
// 
// 处理登录、角色创建、选择、删除等

use mir2_shared::packets::{PacketHeader, Packet, server};
use mir2_shared::enums::ServerPacketIds;
use crate::network::handlers::{NetworkEvent, PacketHandler};
use std::io::Cursor;

/// Character handler - processes character-related packets
pub struct CharacterHandler;

impl PacketHandler for CharacterHandler {
    fn handle(&self, header: &PacketHeader, payload: &[u8]) -> Vec<NetworkEvent> {
        let mut events = Vec::new();
        let mut cursor = Cursor::new(payload);
        
        match header.opcode as u16 {
            // LoginSuccess
            x if x == ServerPacketIds::LoginSuccess as u16 => {
                if let Ok(packet) = server::LoginSuccess::read_body(&mut cursor) {
                    tracing::info!("✅ Login successful, received {} characters", packet.characters.len());
                    events.push(NetworkEvent::LoginSuccess { 
                        characters: packet.characters 
                    });
                }
            }
            
            // StartGameDelay
            x if x == ServerPacketIds::StartGameDelay as u16 => {
                if let Ok(packet) = server::StartGameDelay::read_body(&mut cursor) {
                    events.push(NetworkEvent::StartGame { delay: packet.milliseconds as i32 });
                    tracing::info!("🎮 Starting game with delay: {}ms", packet.milliseconds);
                }
            }
            
            // NewCharacter (character created)
            x if x == ServerPacketIds::NewCharacter as u16 => {
                if let Ok(_packet) = server::NewCharacter::read_body(&mut cursor) {
                    events.push(NetworkEvent::CharacterCreated { 
                        name: "New Character".to_string()  // NewCharacter只有result字段
                    });
                    tracing::info!("👤 Character creation response received");
                }
            }
            
            // DeleteCharacter (character deleted)
            x if x == ServerPacketIds::DeleteCharacter as u16 => {
                if let Ok(_packet) = server::DeleteCharacter::read_body(&mut cursor) {
                    events.push(NetworkEvent::CharacterDeleted { 
                        index: 0  // DeleteCharacter只有result字段
                    });
                    tracing::info!("🗑️ Character deletion response received");
                }
            }
            
            // UserInformation (player data after login)
            x if x == ServerPacketIds::UserInformation as u16 => {
                if let Ok(packet) = server::UserInformation::read_body(&mut cursor) {
                    // 🎯 推送UserInformation事件（SelectScene监听此事件切换到GameScene）
                    events.push(NetworkEvent::UserInformation {
                        location_x: packet.location_x,
                        location_y: packet.location_y,
                        hp: packet.hp,
                        mp: packet.mp,
                        gold: packet.gold,
                    });
                    
                    tracing::info!("📊 User information loaded for player at ({}, {})", 
                        packet.location_x, packet.location_y);
                }
            }
            
            _ => {
                tracing::debug!("⚠️ CharacterHandler: Unhandled opcode {:04X}", header.opcode);
                events.push(NetworkEvent::UnhandledPacket { opcode: header.opcode });
            }
        }
        
        events
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_character_handler_creation() {
        let handler = CharacterHandler;
        // Test with a dummy packet
        let events = handler.handle(&PacketHeader::new(4, 0x0010), &[]);
        assert_eq!(events.len(), 1);
    }
}
