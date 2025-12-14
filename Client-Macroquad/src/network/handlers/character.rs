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
            
            // StartGame
            x if x == ServerPacketIds::StartGame as u16 => {
                if let Ok(packet) = server::StartGame::read_body(&mut cursor) {
                    tracing::info!("🎮 StartGame result: {}", packet.result);
                    events.push(NetworkEvent::StartGame { packet });
                }
            }

            // StartGameBanned
            x if x == ServerPacketIds::StartGameBanned as u16 => {
                if let Ok(packet) = server::StartGameBanned::read_body(&mut cursor) {
                    tracing::warn!(
                        "⛔ StartGame banned: {} (expiry {})",
                        packet.reason,
                        packet.expiry_date
                    );
                    events.push(NetworkEvent::StartGameBanned { packet });
                }
            }

            // StartGameDelay
            x if x == ServerPacketIds::StartGameDelay as u16 => {
                if let Ok(packet) = server::StartGameDelay::read_body(&mut cursor) {
                    tracing::info!("⏳ StartGameDelay: {}ms", packet.milliseconds);
                    events.push(NetworkEvent::StartGameDelay { packet });
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
                    tracing::info!(
                        "📊 UserInformation: {} at ({}, {})",
                        packet.name,
                        packet.location_x,
                        packet.location_y
                    );

                    // 🎯 推送UserInformation事件（进入游戏后的完整状态）
                    events.push(NetworkEvent::UserInformation { packet });
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
