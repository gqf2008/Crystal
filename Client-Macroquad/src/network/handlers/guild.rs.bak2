// Guild Handler - 公会相关数据包处理

use mir2_shared::packets::{PacketHeader, Packet, server};
use mir2_shared::enums::ServerPacketIds;
use crate::network::handlers::{GameEvent, PacketHandler};
use std::io::Cursor;

pub struct GuildHandler;

impl PacketHandler for GuildHandler {
    fn handle(&self, header: &PacketHeader, payload: &[u8]) -> Vec<GameEvent> {
        let mut events = Vec::new();
        let mut cursor = Cursor::new(payload);
        
        match header.opcode as u16 {
            // GuildInvite
            x if x == ServerPacketIds::GuildInvite as u16 => {
                if let Ok(packet) = server::GuildInvite::read_body(&mut cursor) {
                    events.push(GameEvent::GuildInvite {
                        inviter: String::new(),  // GuildInvite只有guild_name字段
                        guild_name: packet.guild_name.clone(),
                    });
                    tracing::info!("🏛️ Guild invite to join {}", packet.guild_name);
                }
            }
            
            // GuildStatus (joined)
            x if x == ServerPacketIds::GuildStatus as u16 => {
                if let Ok(_packet) = server::GuildStatus::read_body(&mut cursor) {
                    // GuildStatus包结构需要进一步检查
                    events.push(GameEvent::GuildJoined {
                        guild_name: String::new(),
                    });
                    tracing::info!("🏛️ Guild status updated");
                }
            }
            
            _ => {
                events.push(GameEvent::UnhandledPacket { opcode: header.opcode });
            }
        }
        
        events
    }
}
