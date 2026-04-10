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
                if let Ok(_packet) = server::NewIntelligentCreature::read_body(&mut cursor) {
                    events.push(NetworkEvent::NewIntelligentCreatureReceived);
                    tracing::debug!("🐾 New intelligent creature received");
                }
            }

            // UpdateIntelligentCreatureList
            x if x == ServerPacketIds::UpdateIntelligentCreatureList as u16 => {
                if let Ok(_packet) = server::UpdateIntelligentCreatureList::read_body(&mut cursor) {
                    events.push(NetworkEvent::IntelligentCreatureListUpdated);
                    tracing::debug!("🐾 Intelligent creature list updated");
                }
            }

            // IntelligentCreatureEnableRename
            x if x == ServerPacketIds::IntelligentCreatureEnableRename as u16 => {
                if let Ok(_packet) = server::IntelligentCreatureEnableRename::read_body(&mut cursor) {
                    events.push(NetworkEvent::IntelligentCreatureRenameEnabled);
                    tracing::debug!("🐾 Intelligent creature rename enabled");
                }
            }

            // IntelligentCreaturePickup
            x if x == ServerPacketIds::IntelligentCreaturePickup as u16 => {
                if let Ok(_packet) = server::IntelligentCreaturePickup::read_body(&mut cursor) {
                    events.push(NetworkEvent::IntelligentCreaturePickupReceived);
                    tracing::debug!("🐾 Intelligent creature pickup received");
                }
            }

            _ => {
                events.push(NetworkEvent::UnhandledPacket { opcode: header.opcode });
            }
        }

        events
    }
}
