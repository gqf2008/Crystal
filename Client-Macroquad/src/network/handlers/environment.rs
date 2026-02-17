// Environment Handler - 环境相关数据包处理 (时间、天气等)

use mir2_shared::packets::{PacketHeader, Packet, server};
use mir2_shared::enums::ServerPacketIds;
use crate::network::handlers::{NetworkEvent, PacketHandler};
use std::io::Cursor;

pub struct EnvironmentHandler;

impl PacketHandler for EnvironmentHandler {
    fn handle(&self, header: &PacketHeader, payload: &[u8]) -> Vec<NetworkEvent> {
        let mut events = Vec::new();
        let mut cursor = Cursor::new(payload);
        
        match header.opcode as u16 {
            x if x == ServerPacketIds::TimeOfDay as u16 => {
                if let Ok(packet) = server::TimeOfDay::read_body(&mut cursor) {
                    events.push(NetworkEvent::TimeOfDayChanged {
                        lights: packet.lights,
                    });
                    tracing::debug!("🌅 Time of day: lights={}", packet.lights);
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
    fn test_environment_handler_unhandled() {
        let handler = EnvironmentHandler;
        let events = handler.handle(&PacketHeader::new(0, 9999), &[]);
        assert_eq!(events.len(), 1);
        assert!(matches!(events[0], NetworkEvent::UnhandledPacket { opcode: 9999 }));
    }

    #[test]
    fn test_time_of_day() {
        let handler = EnvironmentHandler;
        let opcode = ServerPacketIds::TimeOfDay as i16;
        // TimeOfDay reads u8 (lights)
        let payload = [3u8];
        let events = handler.handle(&PacketHeader::new(1, opcode), &payload);
        assert!(events.iter().any(|e| matches!(e, NetworkEvent::TimeOfDayChanged { lights: 3 })));
    }
}
