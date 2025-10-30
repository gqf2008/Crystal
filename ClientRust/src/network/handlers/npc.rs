// NPC Handler - NPC相关数据包处理

use mir2_shared::packets::{PacketHeader, Packet, server};
use mir2_shared::enums::ServerPacketIds;
use crate::network::handlers::{GameEvent, PacketHandler};
use std::io::Cursor;

pub struct NpcHandler;

impl NpcHandler {
    pub fn new() -> Self {
        Self
    }
}

impl PacketHandler for NpcHandler {
    fn handle(&self, header: &PacketHeader, payload: &[u8]) -> Vec<GameEvent> {
        let mut events = Vec::new();
        let mut cursor = Cursor::new(payload);
        
        match header.opcode as u16 {
            // NPCResponse (dialog)
            x if x == ServerPacketIds::NPCResponse as u16 => {
                if let Ok(packet) = server::NPCResponse::read_body(&mut cursor) {
                    let dialog = packet.page.join("\n");
                    events.push(GameEvent::NpcDialog {
                        npc_id: 0,  // NPCResponse只有page字段，没有object_id
                        dialog: dialog.clone(),
                    });
                    tracing::debug!("🗨️ NPC dialog: {}", dialog);
                }
            }
            
            _ => {
                events.push(GameEvent::UnhandledPacket { opcode: header.opcode });
            }
        }
        
        events
    }
}

impl Default for NpcHandler {
    fn default() -> Self {
        Self::new()
    }
}
