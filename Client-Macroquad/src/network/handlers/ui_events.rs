// UI Events Handler - UI/表现层相关数据包处理
//
// 目前仅实现：PlaySound (266)

use crate::network::handlers::{NetworkEvent, PacketHandler};
use mir2_shared::enums::ServerPacketIds;
use mir2_shared::packets::{server, Packet, PacketHeader};
use std::io::Cursor;

/// UI / 表现层事件 handler
pub struct UiEventsHandler;

impl PacketHandler for UiEventsHandler {
    fn handle(&self, header: &PacketHeader, payload: &[u8]) -> Vec<NetworkEvent> {
        let mut events = Vec::new();
        let mut cursor = Cursor::new(payload);

        match header.opcode as u16 {
            x if x == ServerPacketIds::PlaySound as u16 => {
                if let Ok(packet) = server::PlaySound::read_body(&mut cursor) {
                    events.push(NetworkEvent::PlaySound {
                        sound_id: packet.sound_id,
                    });
                    tracing::debug!("🔊 PlaySound received id={}", packet.sound_id);
                } else {
                    events.push(NetworkEvent::UnhandledPacket { opcode: header.opcode });
                }
            }

            x if x == ServerPacketIds::MountUpdate as u16 => {
                if let Ok(packet) = server::MountUpdate::read_body(&mut cursor) {
                    events.push(NetworkEvent::MountUpdated {
                        object_id: packet.object_id,
                        mount_type: packet.mount_type,
                        riding_mount: packet.riding_mount,
                    });
                    tracing::debug!(
                        "🐎 MountUpdate received object_id={} mount_type={} riding={}",
                        packet.object_id,
                        packet.mount_type,
                        packet.riding_mount
                    );
                } else {
                    events.push(NetworkEvent::UnhandledPacket { opcode: header.opcode });
                }
            }

            _ => {
                events.push(NetworkEvent::UnhandledPacket { opcode: header.opcode });
            }
        }

        events
    }
}
