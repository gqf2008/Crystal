// Player Handler - 玩家相关数据包处理（Inspect 等）

use crate::network::handlers::{NetworkEvent, PacketHandler};
use mir2_shared::enums::ServerPacketIds;
use mir2_shared::packets::{server, Packet, PacketHeader};
use std::io::Cursor;

pub struct PlayerHandler;

impl PacketHandler for PlayerHandler {
    fn handle(&self, header: &PacketHeader, payload: &[u8]) -> Vec<NetworkEvent> {
        let mut events = Vec::new();
        let mut cursor = Cursor::new(payload);

        match header.opcode as u16 {
            x if x == ServerPacketIds::PlayerInspect as u16 => {
                if let Ok(packet) = server::PlayerInspect::read_body(&mut cursor) {
                    events.push(NetworkEvent::PlayerInspect { packet });
                } else {
                    events.push(NetworkEvent::UnhandledPacket {
                        opcode: header.opcode,
                    });
                }
            }
            _ => {
                tracing::debug!("⚠️ PlayerHandler: Unknown opcode {:04X}", header.opcode);
                events.push(NetworkEvent::UnhandledPacket {
                    opcode: header.opcode,
                });
            }
        }

        events
    }
}
