// Social Handler - 婚姻/师徒/社交数据包处理

use mir2_shared::packets::{PacketHeader, Packet, server};
use mir2_shared::enums::ServerPacketIds;
use crate::network::handlers::{NetworkEvent, PacketHandler};
use std::io::Cursor;

pub struct SocialHandler;

impl PacketHandler for SocialHandler {
    fn handle(&self, header: &PacketHeader, payload: &[u8]) -> Vec<NetworkEvent> {
        let mut events = Vec::new();
        let mut cursor = Cursor::new(payload);

        match header.opcode as u16 {
            // MarriageRequest
            x if x == ServerPacketIds::MarriageRequest as u16 => {
                if let Ok(packet) = server::MarriageRequest::read_body(&mut cursor) {
                    events.push(NetworkEvent::MarriageRequested2 {
                        requester: packet.lover_name.clone(),
                    });
                    tracing::debug!("💍 Marriage requested by: {}", packet.lover_name);
                }
            }

            // DivorceRequest
            x if x == ServerPacketIds::DivorceRequest as u16 => {
                if let Ok(_packet) = server::DivorceRequest::read_body(&mut cursor) {
                    events.push(NetworkEvent::DivorceRequested2);
                    tracing::debug!("💔 Divorce requested");
                }
            }

            // MentorRequest
            x if x == ServerPacketIds::MentorRequest as u16 => {
                if let Ok(_packet) = server::MentorRequest::read_body(&mut cursor) {
                    events.push(NetworkEvent::MentorRequested2);
                    tracing::debug!("🎓 Mentor requested");
                }
            }

            // LoverUpdate
            x if x == ServerPacketIds::LoverUpdate as u16 => {
                if let Ok(packet) = server::LoverUpdate::read_body(&mut cursor) {
                    events.push(NetworkEvent::LoverUpdated {
                        lover_name: packet.lover_name.clone(),
                        date: packet.date,
                    });
                    tracing::debug!("💕 Lover updated: {} (date: {})", packet.lover_name, packet.date);
                }
            }

            // MentorUpdate
            x if x == ServerPacketIds::MentorUpdate as u16 => {
                if let Ok(packet) = server::MentorUpdate::read_body(&mut cursor) {
                    events.push(NetworkEvent::MentorUpdated {
                        mentor_name: packet.mentor_name.clone(),
                        mentor_level: packet.mentor_level,
                        mentor_online: packet.mentor_online,
                    });
                    tracing::debug!("🎓 Mentor updated: {} (Lv.{})", packet.mentor_name, packet.mentor_level);
                }
            }

            _ => {
                events.push(NetworkEvent::UnhandledPacket { opcode: header.opcode });
            }
        }

        events
    }
}
