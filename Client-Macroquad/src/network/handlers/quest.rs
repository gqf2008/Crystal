// Quest Handler - 任务相关数据包处理

use mir2_shared::packets::{PacketHeader, Packet, server};
use mir2_shared::enums::ServerPacketIds;
use crate::network::handlers::{NetworkEvent, PacketHandler};
use std::io::Cursor;

pub struct QuestHandler;

impl PacketHandler for QuestHandler {
    fn handle(&self, header: &PacketHeader, payload: &[u8]) -> Vec<NetworkEvent> {
        let mut events = Vec::new();
        let mut cursor = Cursor::new(payload);
        
        match header.opcode as u16 {
            x if x == ServerPacketIds::ChangeQuest as u16 => {
                if let Ok(packet) = server::ChangeQuest::read_body(&mut cursor) {
                    events.push(NetworkEvent::QuestChanged {
                        quest: packet.quest,
                    });
                    tracing::debug!("📜 Quest changed");
                }
            }
            x if x == ServerPacketIds::CompleteQuest as u16 => {
                if let Ok(packet) = server::CompleteQuest::read_body(&mut cursor) {
                    events.push(NetworkEvent::QuestCompleted {
                        quest_id: packet.quest_id as u32,
                    });
                    tracing::debug!("📜 Quest completed: {}", packet.quest_id);
                }
            }
            x if x == ServerPacketIds::GainedQuestItem as u16 => {
                if let Ok(packet) = server::GainedQuestItem::read_body(&mut cursor) {
                    events.push(NetworkEvent::QuestItemGained {
                        item_id: packet.item_id,
                    });
                    tracing::debug!("📜 Quest item gained: {}", packet.item_id);
                }
            }
            
            _ => {
                tracing::trace!("Quest packet: {:04X}", header.opcode);
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
    fn test_quest_handler_unhandled() {
        let handler = QuestHandler;
        let events = handler.handle(&PacketHeader::new(0, 9999), &[]);
        assert_eq!(events.len(), 1);
        assert!(matches!(events[0], NetworkEvent::UnhandledPacket { opcode: 9999 }));
    }

    #[test]
    fn test_quest_complete() {
        let handler = QuestHandler;
        let opcode = ServerPacketIds::CompleteQuest as i16;
        // CompleteQuest reads an i32 (quest_id)
        let payload = 42i32.to_le_bytes();
        let events = handler.handle(&PacketHeader::new(4, opcode), &payload);
        assert!(events.iter().any(|e| matches!(e, NetworkEvent::QuestCompleted { quest_id: 42 })));
    }
}
