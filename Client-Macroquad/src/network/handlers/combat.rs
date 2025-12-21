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

            // DamageIndicator - damage number & target id
            x if x == ServerPacketIds::DamageIndicator as u16 => {
                if let Ok(packet) = server::DamageIndicator::read_body(&mut cursor) {
                    events.push(NetworkEvent::DamageIndicator {
                        object_id: packet.object_id,
                        damage: packet.damage,
                        damage_type: packet.damage_type,
                    });
                    tracing::trace!(
                        "💥 DamageIndicator: object={} dmg={} type={}",
                        packet.object_id,
                        packet.damage,
                        packet.damage_type
                    );
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

            // HealthChanged - local player hp/mp updated
            x if x == ServerPacketIds::HealthChanged as u16 => {
                if let Ok(packet) = server::HealthChanged::read_body(&mut cursor) {
                    // 协议只携带 hp/mp 当前值；max 由客户端已有状态决定。
                    // 用 max=0 作为“未知/不要覆盖 max”的标记，由落地层处理。
                    events.push(NetworkEvent::HealthChanged {
                        current: packet.hp,
                        max: 0,
                    });
                    events.push(NetworkEvent::ManaChanged {
                        current: packet.mp,
                        max: 0,
                    });
                    tracing::debug!("❤️ HealthChanged hp={} mp={}", packet.hp, packet.mp);
                }
            }

            // HeroHealthChanged - ignore for now (no hero ECS yet), but keep parser to avoid Unhandled
            x if x == ServerPacketIds::HeroHealthChanged as u16 => {
                if let Ok(packet) = server::HeroHealthChanged::read_body(&mut cursor) {
                    tracing::trace!("🧡 HeroHealthChanged hp={} mp={}", packet.hp, packet.mp);
                }
            }

            // ObjectHealth - percent based health update
            x if x == ServerPacketIds::ObjectHealth as u16 => {
                if let Ok(packet) = server::ObjectHealth::read_body(&mut cursor) {
                    events.push(NetworkEvent::ObjectHealthPercent {
                        object_id: packet.object_id,
                        percent: packet.percent,
                        expire: packet.expire,
                    });
                    tracing::trace!(
                        "🩸 ObjectHealthPercent object={} {}% expire={}",
                        packet.object_id,
                        packet.percent,
                        packet.expire
                    );
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
