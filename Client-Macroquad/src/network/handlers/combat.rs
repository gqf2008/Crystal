// Combat Handler - 战斗相关数据包处理

use mir2_shared::packets::{PacketHeader, Packet, server};
use mir2_shared::enums::ServerPacketIds;
use crate::network::handlers::{NetworkEvent, PacketHandler};
use std::io::Cursor;

pub struct CombatHandler;

impl PacketHandler for CombatHandler {
    fn handle(&self, header: &PacketHeader, payload: &[u8]) -> Vec<NetworkEvent> {
        let mut events = Vec::new();
        let mut cursor = Cursor::new(payload);
        
        match header.opcode as u16 {
            // ObjectAttack - another object attacks
            x if x == ServerPacketIds::ObjectAttack as u16 => {
                if let Ok(packet) = server::ObjectAttack::read_body(&mut cursor) {
                    events.push(NetworkEvent::ObjectAttack { packet });
                    tracing::trace!("⚔️ ObjectAttack received");
                }
            }

            // Struck - player was hit
            x if x == ServerPacketIds::Struck as u16 => {
                if let Ok(packet) = server::Struck::read_body(&mut cursor) {
                    events.push(NetworkEvent::PlayerStruck {
                        attacker_id: packet.attacker_id,
                        damage: 0,  // Struck包没有damage字段
                    });
                    tracing::debug!("⚔️ Player struck by {}", packet.attacker_id);
                }
            }
            
            // Death - player died
            x if x == ServerPacketIds::Death as u16 => {
                if let Ok(_packet) = server::Death::read_body(&mut cursor) {
                    events.push(NetworkEvent::PlayerDied);
                    tracing::warn!("💀 Player died");
                }
            }
            
            // ObjectStruck - another object was hit
            x if x == ServerPacketIds::ObjectStruck as u16 => {
                if let Ok(packet) = server::ObjectStruck::read_body(&mut cursor) {
                    events.push(NetworkEvent::ObjectStruck {
                        object_id: packet.object_id,
                        attacker_id: packet.attacker_id,
                        damage: 0,  // ObjectStruck包没有damage字段
                    });
                    tracing::trace!("⚔️ Object {} struck by {}", 
                        packet.object_id, packet.attacker_id);
                }
            }
            
            // ObjectDied - another object died
            x if x == ServerPacketIds::ObjectDied as u16 => {
                if let Ok(packet) = server::ObjectDied::read_body(&mut cursor) {
                    events.push(NetworkEvent::ObjectDied {
                        object_id: packet.object_id,
                    });
                    tracing::trace!("💀 Object {} died", packet.object_id);
                }
            }
            
            // GainExperience
            x if x == ServerPacketIds::GainExperience as u16 => {
                if let Ok(packet) = server::GainExperience::read_body(&mut cursor) {
                    events.push(NetworkEvent::ExperienceGained {
                        amount: packet.amount as i64,  // u32→i64
                    });
                    tracing::debug!("✨ Experience gained: {}", packet.amount);
                }
            }
            
            // LevelChanged
            x if x == ServerPacketIds::LevelChanged as u16 => {
                if let Ok(packet) = server::LevelChanged::read_body(&mut cursor) {
                    events.push(NetworkEvent::LevelUp {
                        new_level: packet.level,
                    });
                    tracing::info!("🎉 Level up to {}!", packet.level);
                }
            }
            
            _ => {
                events.push(NetworkEvent::UnhandledPacket { opcode: header.opcode });
            }
        }
        
        events
    }
}
