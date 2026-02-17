// Spell Handler - 魔法/技能相关数据包处理

use mir2_shared::packets::{PacketHeader, Packet, server};
use mir2_shared::enums::ServerPacketIds;
use crate::network::handlers::{NetworkEvent, PacketHandler};
use std::io::Cursor;

pub struct SpellHandler;

impl PacketHandler for SpellHandler {
    fn handle(&self, header: &PacketHeader, payload: &[u8]) -> Vec<NetworkEvent> {
        let mut events = Vec::new();
        let mut cursor = Cursor::new(payload);
        
        match header.opcode as u16 {
            x if x == ServerPacketIds::NewMagic as u16 => {
                if let Ok(packet) = server::NewMagic::read_body(&mut cursor) {
                    events.push(NetworkEvent::NewMagicReceived {
                        magic: packet.magic,
                        hero: packet.hero,
                    });
                    tracing::debug!("🔮 New magic learned (hero={})", packet.hero);
                }
            }
            x if x == ServerPacketIds::MagicLeveled as u16 => {
                if let Ok(packet) = server::MagicLeveled::read_body(&mut cursor) {
                    events.push(NetworkEvent::MagicLeveledUp {
                        spell: packet.spell,
                        level: packet.level,
                        hero: packet.hero,
                    });
                    tracing::debug!("🔮 Magic leveled: {:?} -> lv{}", packet.spell, packet.level);
                }
            }
            x if x == ServerPacketIds::SpellToggle as u16 => {
                if let Ok(packet) = server::SpellToggle::read_body(&mut cursor) {
                    events.push(NetworkEvent::SpellToggled {
                        spell: packet.spell,
                        can_use: packet.can_use,
                        hero: packet.hero,
                    });
                    tracing::debug!("🔮 Spell toggle: {:?} can_use={}", packet.spell, packet.can_use);
                }
            }
            
            _ => {
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
    fn test_spell_handler_unhandled() {
        let handler = SpellHandler;
        let events = handler.handle(&PacketHeader::new(0, 9999), &[]);
        assert_eq!(events.len(), 1);
        assert!(matches!(events[0], NetworkEvent::UnhandledPacket { opcode: 9999 }));
    }

    #[test]
    fn test_spell_handler_new_magic_empty() {
        let handler = SpellHandler;
        let opcode = ServerPacketIds::NewMagic as i16;
        // Empty payload will fail read_body, so no event is pushed
        let events = handler.handle(&PacketHeader::new(0, opcode), &[]);
        assert!(events.is_empty());
    }
}
