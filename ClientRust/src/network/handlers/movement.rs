// Movement Handler - 移动相关数据包处理

use mir2_shared::packets::{PacketHeader, Packet, server};
use mir2_shared::enums::ServerPacketIds;
use crate::network::handlers::{GameEvent, PacketHandler};
use std::io::Cursor;

pub struct MovementHandler;

impl MovementHandler {
    pub fn new() -> Self {
        Self
    }
}

impl PacketHandler for MovementHandler {
    fn handle(&self, header: &PacketHeader, payload: &[u8]) -> Vec<GameEvent> {
        let mut events = Vec::new();
        let mut cursor = Cursor::new(payload);
        
        match header.opcode as u16 {
            // UserLocation
            x if x == ServerPacketIds::UserLocation as u16 => {
                if let Ok(packet) = server::UserLocation::read_body(&mut cursor) {
                    events.push(GameEvent::PlayerLocationChanged {
                        x: packet.location_x,
                        y: packet.location_y,
                    });
                    tracing::trace!("📍 User location updated: ({}, {})", 
                        packet.location_x, packet.location_y);
                }
            }
            
            _ => {
                events.push(GameEvent::UnhandledPacket { opcode: header.opcode });
            }
        }
        
        events
    }
}

impl Default for MovementHandler {
    fn default() -> Self {
        Self::new()
    }
}
