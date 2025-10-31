// Group Handler - 组队相关数据包处理

use mir2_shared::packets::{PacketHeader, Packet, server};
use mir2_shared::enums::ServerPacketIds;
use crate::network::handlers::{GameEvent, PacketHandler};
use std::io::Cursor;

pub struct GroupHandler;

impl PacketHandler for GroupHandler {
    fn handle(&self, header: &PacketHeader, payload: &[u8]) -> Vec<GameEvent> {
        let mut events = Vec::new();
        let mut cursor = Cursor::new(payload);
        
        match header.opcode as u16 {
            // GroupInvite
            x if x == ServerPacketIds::GroupInvite as u16 => {
                if let Ok(packet) = server::GroupInvite::read_body(&mut cursor) {
                    events.push(GameEvent::GroupInvite {
                        inviter: packet.name.clone(),
                    });
                    tracing::info!("👥 Group invite from: {}", packet.name);
                }
            }
            
            // AddMember
            x if x == ServerPacketIds::AddMember as u16 => {
                if let Ok(packet) = server::AddMember::read_body(&mut cursor) {
                    events.push(GameEvent::GroupMemberAdded {
                        name: packet.name.clone(),
                    });
                    tracing::info!("👥 Member added to group: {}", packet.name);
                }
            }
            
            // DeleteMember
            x if x == ServerPacketIds::DeleteMember as u16 => {
                if let Ok(packet) = server::DeleteMember::read_body(&mut cursor) {
                    events.push(GameEvent::GroupMemberRemoved {
                        name: packet.name.clone(),
                    });
                    tracing::info!("👥 Member removed from group: {}", packet.name);
                }
            }
            
            // DeleteGroup
            x if x == ServerPacketIds::DeleteGroup as u16 => {
                if let Ok(_packet) = server::DeleteGroup::read_body(&mut cursor) {
                    events.push(GameEvent::GroupDisbanded);
                    tracing::info!("👥 Group disbanded");
                }
            }
            
            _ => {
                events.push(GameEvent::UnhandledPacket { opcode: header.opcode });
            }
        }
        
        events
    }
}
