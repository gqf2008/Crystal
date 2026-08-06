// Group Handler - 组队相关数据包处理

use crate::network::handlers::{NetworkEvent, PacketHandler};
use mir2_shared::enums::ServerPacketIds;
use mir2_shared::packets::{server, Packet, PacketHeader};
use std::io::Cursor;

pub struct GroupHandler;

impl PacketHandler for GroupHandler {
    fn handle(&self, header: &PacketHeader, payload: &[u8]) -> Vec<NetworkEvent> {
        let mut events = Vec::new();
        let mut cursor = Cursor::new(payload);

        match header.opcode as u16 {
            // GroupInvite
            x if x == ServerPacketIds::GroupInvite as u16 => {
                if let Ok(packet) = server::GroupInvite::read_body(&mut cursor) {
                    events.push(NetworkEvent::GroupInvite {
                        inviter: packet.name.clone(),
                    });
                    tracing::info!("👥 Group invite from: {}", packet.name);
                }
            }

            // AddMember
            x if x == ServerPacketIds::AddMember as u16 => {
                if let Ok(packet) = server::AddMember::read_body(&mut cursor) {
                    events.push(NetworkEvent::GroupMemberAdded {
                        name: packet.name.clone(),
                    });
                    tracing::info!("👥 Member added to group: {}", packet.name);
                }
            }

            // DeleteMember
            x if x == ServerPacketIds::DeleteMember as u16 => {
                if let Ok(packet) = server::DeleteMember::read_body(&mut cursor) {
                    events.push(NetworkEvent::GroupMemberRemoved {
                        name: packet.name.clone(),
                    });
                    tracing::info!("👥 Member removed from group: {}", packet.name);
                }
            }

            // DeleteGroup
            x if x == ServerPacketIds::DeleteGroup as u16 => {
                if let Ok(_packet) = server::DeleteGroup::read_body(&mut cursor) {
                    events.push(NetworkEvent::GroupDisbanded);
                    tracing::info!("👥 Group disbanded");
                }
            }

            // SwitchGroup
            x if x == ServerPacketIds::SwitchGroup as u16 => {
                if let Ok(packet) = server::SwitchGroup::read_body(&mut cursor) {
                    events.push(NetworkEvent::GroupModeChanged {
                        allow_group: if packet.allow_group { 1 } else { 0 },
                    });
                    tracing::info!("👥 Group mode switched: allow_group={}", packet.allow_group);
                }
            }

            // GroupMembersMap
            x if x == ServerPacketIds::GroupMembersMap as u16 => {
                if let Ok(packet) = server::GroupMembersMap::read_body(&mut cursor) {
                    events.push(NetworkEvent::GroupMembersMapUpdated {
                        player_name: packet.player_name.clone(),
                        player_map: packet.player_map.clone(),
                    });
                    tracing::info!(
                        "👥 Group member '{}' on map '{}'",
                        packet.player_name,
                        packet.player_map
                    );
                }
            }

            // SendMemberLocation
            x if x == ServerPacketIds::SendMemberLocation as u16 => {
                if let Ok(packet) = server::SendMemberLocation::read_body(&mut cursor) {
                    events.push(NetworkEvent::GroupMemberLocationUpdated {
                        name: packet.member_name.clone(),
                        x: packet.location.x,
                        y: packet.location.y,
                    });
                    tracing::debug!(
                        "👥 Group member {} at ({}, {})",
                        packet.member_name,
                        packet.location.x,
                        packet.location.y
                    );
                }
            }

            _ => {
                tracing::warn!("⚠️ GroupHandler: Unknown opcode {:04X}", header.opcode);
                events.push(NetworkEvent::UnhandledPacket {
                    opcode: header.opcode,
                });
            }
        }

        events
    }
}
