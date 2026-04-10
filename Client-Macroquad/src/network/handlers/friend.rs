// Friend Handler - 好友协议 → NetworkEvent
// 负责将服务器发来的好友协议包转换为 NetworkEvent

use mir2_shared::packets::{PacketHeader, Packet, server};
use mir2_shared::enums::ServerPacketIds;
use crate::network::handlers::{NetworkEvent, PacketHandler};
use std::io::Cursor;

pub struct FriendHandler;

impl PacketHandler for FriendHandler {
    fn handle(&self, header: &PacketHeader, payload: &[u8]) -> Vec<NetworkEvent> {
        let mut events = Vec::new();
        let mut cursor = Cursor::new(payload);

        match header.opcode as u16 {
            // FriendUpdate - 好友列表更新
            x if x == ServerPacketIds::FriendUpdate as u16 => {
                if let Ok(packet) = server::FriendUpdate::read_body(&mut cursor) {
                    events.push(NetworkEvent::FriendUpdated);
                    tracing::debug!("👥 好友列表同步: {} 个好友", packet.friends.len());
                }
            }

            _ => {
                events.push(NetworkEvent::UnhandledPacket { opcode: header.opcode });
            }
        }

        events
    }
}
