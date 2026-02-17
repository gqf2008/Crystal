// Guild Handler - 公会相关数据包处理

use mir2_shared::packets::{PacketHeader, Packet, server};
use mir2_shared::enums::ServerPacketIds;
use crate::network::handlers::{NetworkEvent, PacketHandler};
use std::io::Cursor;

pub struct GuildHandler;

impl PacketHandler for GuildHandler {
    fn handle(&self, header: &PacketHeader, payload: &[u8]) -> Vec<NetworkEvent> {
        let mut events = Vec::new();
        let mut cursor = Cursor::new(payload);
        
        match header.opcode as u16 {
            // GuildInvite
            x if x == ServerPacketIds::GuildInvite as u16 => {
                if let Ok(packet) = server::GuildInvite::read_body(&mut cursor) {
                    events.push(NetworkEvent::GuildInvite {
                        inviter: String::new(),
                        guild_name: packet.guild_name.clone(),
                    });
                    tracing::info!("🏛️ Guild invite to join {}", packet.guild_name);
                }
            }
            
            // GuildStatus
            x if x == ServerPacketIds::GuildStatus as u16 => {
                if let Ok(packet) = server::GuildStatus::read_body(&mut cursor) {
                    events.push(NetworkEvent::GuildStatusReceived {
                        guild_name: packet.guild_name.clone(),
                        rank_name: packet.rank_name.clone(),
                    });
                    tracing::info!("🏛️ Guild status: {} rank={}", packet.guild_name, packet.rank_name);
                }
            }

            // GuildNoticeChange
            x if x == ServerPacketIds::GuildNoticeChange as u16 => {
                if let Ok(packet) = server::GuildNoticeChange::read_body(&mut cursor) {
                    events.push(NetworkEvent::GuildNoticeReceived {
                        notice: packet.notice.clone(),
                    });
                    tracing::info!("🏛️ Guild notice updated");
                }
            }

            // GuildMemberChange
            x if x == ServerPacketIds::GuildMemberChange as u16 => {
                if let Ok(packet) = server::GuildMemberChange::read_body(&mut cursor) {
                    events.push(NetworkEvent::GuildMemberListReceived {
                        name: packet.name.clone(),
                        rank_index: packet.rank_index,
                        status: packet.status,
                        ranks: packet.ranks.clone(),
                    });
                    tracing::info!("🏛️ Guild member change: {}", packet.name);
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
    fn test_guild_handler_unhandled() {
        let handler = GuildHandler;
        let events = handler.handle(&PacketHeader::new(0, 9999), &[]);
        assert_eq!(events.len(), 1);
        assert!(matches!(events[0], NetworkEvent::UnhandledPacket { opcode: 9999 }));
    }

    #[test]
    fn test_guild_status() {
        let handler = GuildHandler;
        let opcode = ServerPacketIds::GuildStatus as i16;
        // GuildStatus has guild_name and rank_name (dotnet strings)
        // Empty payload will fail read_body, so we just verify no panic
        let events = handler.handle(&PacketHeader::new(0, opcode), &[]);
        // read_body fails on empty payload, so no event is pushed
        assert!(events.is_empty());
    }
}
