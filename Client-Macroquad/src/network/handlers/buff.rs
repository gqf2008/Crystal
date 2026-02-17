// Buff Handler - Buff/Debuff 相关数据包处理

use mir2_shared::packets::{PacketHeader, Packet, server};
use mir2_shared::enums::ServerPacketIds;
use crate::network::handlers::{NetworkEvent, PacketHandler};
use std::io::Cursor;

pub struct BuffHandler;

impl PacketHandler for BuffHandler {
    fn handle(&self, header: &PacketHeader, payload: &[u8]) -> Vec<NetworkEvent> {
        let mut events = Vec::new();
        let mut cursor = Cursor::new(payload);
        
        match header.opcode as u16 {
            x if x == ServerPacketIds::AddBuff as u16 => {
                if let Ok(packet) = server::AddBuff::read_body(&mut cursor) {
                    events.push(NetworkEvent::BuffAdded {
                        buff: packet.buff,
                    });
                    tracing::debug!("✨ Buff added");
                }
            }
            x if x == ServerPacketIds::RemoveBuff as u16 => {
                if let Ok(packet) = server::RemoveBuff::read_body(&mut cursor) {
                    events.push(NetworkEvent::BuffRemoved {
                        buff_type: packet.buff_type,
                        object_id: packet.object_id,
                    });
                    tracing::debug!("✨ Buff removed: object={}", packet.object_id);
                }
            }
            x if x == ServerPacketIds::PauseBuff as u16 => {
                if let Ok(packet) = server::PauseBuff::read_body(&mut cursor) {
                    events.push(NetworkEvent::BuffPaused {
                        buff_type: packet.buff_type,
                        object_id: packet.object_id,
                        paused: packet.paused,
                    });
                    tracing::debug!("✨ Buff paused={}: object={}", packet.paused, packet.object_id);
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
    fn test_buff_handler_unhandled() {
        let handler = BuffHandler;
        let events = handler.handle(&PacketHeader::new(0, 9999), &[]);
        assert_eq!(events.len(), 1);
        assert!(matches!(events[0], NetworkEvent::UnhandledPacket { opcode: 9999 }));
    }

    #[test]
    fn test_buff_handler_add_empty_payload() {
        let handler = BuffHandler;
        let opcode = ServerPacketIds::AddBuff as i16;
        // Empty payload will fail read_body, so no event is pushed
        let events = handler.handle(&PacketHeader::new(0, opcode), &[]);
        assert!(events.is_empty());
    }
}
