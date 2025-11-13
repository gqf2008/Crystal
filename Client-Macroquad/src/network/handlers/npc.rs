// NPC Handler - NPC相关数据包处理

use mir2_shared::packets::{PacketHeader, Packet, server};
use mir2_shared::enums::ServerPacketIds;
use crate::network::handlers::{NetworkEvent, PacketHandler};
use std::io::Cursor;

pub struct NpcHandler;

impl PacketHandler for NpcHandler {
    fn handle(&self, header: &PacketHeader, payload: &[u8]) -> Vec<NetworkEvent> {
        let mut events = Vec::new();
        let mut cursor = Cursor::new(payload);
        
        match header.opcode as u16 {
            // NPCResponse (dialog)
            x if x == ServerPacketIds::NPCResponse as u16 => {
                if let Ok(packet) = server::NPCResponse::read_body(&mut cursor) {
                    let dialog = packet.page.join("\n");
                    events.push(NetworkEvent::NpcDialog {
                        npc_id: 0,  // NPCResponse只有page字段，没有object_id
                        dialog: dialog.clone(),
                    });
                    tracing::debug!("🗨️ NPC dialog: {}", dialog);
                }
            }
            
            _ => {
                events.push(NetworkEvent::UnhandledPacket { opcode: header.opcode });
            }
        }
        
        events
    }
}
