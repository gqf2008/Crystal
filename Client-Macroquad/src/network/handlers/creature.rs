// Creature Handler - 智能宠物数据包处理

use mir2_shared::packets::{PacketHeader, Packet, server};
use mir2_shared::enums::ServerPacketIds;
use crate::network::handlers::{NetworkEvent, PacketHandler};
use std::io::Cursor;

pub struct CreatureHandler;

impl PacketHandler for CreatureHandler {
    fn handle(&self, header: &PacketHeader, payload: &[u8]) -> Vec<NetworkEvent> {
        let mut events = Vec::new();
        let mut cursor = Cursor::new(payload);

        match header.opcode as u16 {
            // NewIntelligentCreature
            x if x == ServerPacketIds::NewIntelligentCreature as u16 => {
                if let Ok(packet) = server::NewIntelligentCreature::read_body(&mut cursor) {
                    events.push(NetworkEvent::NewIntelligentCreatureReceived {
                        creature_type: packet.creature_type as u8,
                    });
                    tracing::debug!("🐾 New intelligent creature received: {:?}", packet.creature_type);
                }
            }

            // UpdateIntelligentCreatureList
            x if x == ServerPacketIds::UpdateIntelligentCreatureList as u16 => {
                if let Ok(packet) = server::UpdateIntelligentCreatureList::read_body(&mut cursor) {
                    let count = packet.creatures.len();
                    events.push(NetworkEvent::IntelligentCreatureListUpdated { creatures: packet.creatures });
                    tracing::debug!("🐾 Intelligent creature list updated: {} creatures", count);
                }
            }

            // IntelligentCreatureEnableRename
            x if x == ServerPacketIds::IntelligentCreatureEnableRename as u16 => {
                if let Ok(packet) = server::IntelligentCreatureEnableRename::read_body(&mut cursor) {
                    events.push(NetworkEvent::IntelligentCreatureRenameEnabled { can_rename: packet.can_rename });
                    tracing::debug!("🐾 Intelligent creature rename enabled: {}", packet.can_rename);
                }
            }

            // IntelligentCreaturePickup
            x if x == ServerPacketIds::IntelligentCreaturePickup as u16 => {
                if let Ok(packet) = server::IntelligentCreaturePickup::read_body(&mut cursor) {
                    events.push(NetworkEvent::IntelligentCreaturePickupReceived { enabled: packet.enabled });
                    tracing::debug!("🐾 Intelligent creature pickup: {}", packet.enabled);
                }
            }

            _ => {
                tracing::debug!("⚠️ CreatureHandler: Unknown opcode {:04X}", header.opcode);
                events.push(NetworkEvent::UnhandledPacket { opcode: header.opcode });
            }
        }

        events
    }
}
