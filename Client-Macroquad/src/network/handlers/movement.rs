// Movement Handler - 移动相关数据包处理

use mir2_shared::packets::{PacketHeader, Packet, server};
use mir2_shared::enums::ServerPacketIds;
use crate::network::handlers::{NetworkEvent, PacketHandler};
use std::io::Cursor;

pub struct MovementHandler;

impl PacketHandler for MovementHandler {
    fn handle(&self, header: &PacketHeader, payload: &[u8]) -> Vec<NetworkEvent> {
        let mut events = Vec::new();
        let mut cursor = Cursor::new(payload);
        
        match header.opcode as u16 {
            // MapInformation
            x if x == ServerPacketIds::MapInformation as u16 => {
                if let Ok(packet) = server::MapInformation::read_body(&mut cursor) {
                    tracing::info!(
                        "🗺️ MapInformation: idx={} file={} title={}",
                        packet.map_index,
                        packet.file_name,
                        packet.title
                    );
                    events.push(NetworkEvent::MapInformation { packet });
                }
            }

            // MapChanged
            x if x == ServerPacketIds::MapChanged as u16 => {
                if let Ok(packet) = server::MapChanged::read_body(&mut cursor) {
                    tracing::info!(
                        "🗺️ MapChanged: idx={} file={} title={} loc=({}, {})",
                        packet.map_index,
                        packet.file_name,
                        packet.title,
                        packet.location_x,
                        packet.location_y
                    );
                    events.push(NetworkEvent::MapChanged { packet });
                }
            }

            // UserLocation
            x if x == ServerPacketIds::UserLocation as u16 => {
                if let Ok(packet) = server::UserLocation::read_body(&mut cursor) {
                    events.push(NetworkEvent::PlayerLocationChanged {
                        x: packet.location_x,
                        y: packet.location_y,
                    });
                    tracing::trace!("📍 User location updated: ({}, {})", 
                        packet.location_x, packet.location_y);
                }
            }

            // ===== Object spawns =====
            x if x == ServerPacketIds::ObjectMonster as u16 => {
                if let Ok(packet) = server::ObjectMonster::read_body(&mut cursor) {
                    tracing::trace!(
                        "👾 ObjectMonster: id={} image={} loc=({}, {})",
                        packet.object_id,
                        packet.image,
                        packet.location_x,
                        packet.location_y
                    );
                    events.push(NetworkEvent::ObjectMonster { packet });
                }
            }
            x if x == ServerPacketIds::ObjectNpc as u16 => {
                if let Ok(packet) = server::ObjectNpc::read_body(&mut cursor) {
                    tracing::trace!(
                        "🧑‍💼 ObjectNpc: id={} image={} loc=({}, {})",
                        packet.object_id,
                        packet.image,
                        packet.location_x,
                        packet.location_y
                    );
                    events.push(NetworkEvent::ObjectNpc { packet });
                }
            }

            // ===== Object lifecycle =====
            x if x == ServerPacketIds::ObjectRemove as u16 => {
                if let Ok(packet) = server::ObjectRemove::read_body(&mut cursor) {
                    events.push(NetworkEvent::ObjectRemove { packet });
                }
            }

            // ===== Object movement =====
            x if x == ServerPacketIds::ObjectTurn as u16 => {
                if let Ok(packet) = server::ObjectTurn::read_body(&mut cursor) {
                    events.push(NetworkEvent::ObjectTurn { packet });
                }
            }
            x if x == ServerPacketIds::ObjectWalk as u16 => {
                if let Ok(packet) = server::ObjectWalk::read_body(&mut cursor) {
                    events.push(NetworkEvent::ObjectWalk { packet });
                }
            }
            x if x == ServerPacketIds::ObjectRun as u16 => {
                if let Ok(packet) = server::ObjectRun::read_body(&mut cursor) {
                    events.push(NetworkEvent::ObjectRun { packet });
                }
            }
            
            _ => {
                events.push(NetworkEvent::UnhandledPacket { opcode: header.opcode });
            }
        }
        
        events
    }
}
