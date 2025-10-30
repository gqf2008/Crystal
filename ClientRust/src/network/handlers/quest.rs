// Quest Handler - 任务相关数据包处理

use mir2_shared::packets::PacketHeader;
use crate::network::handlers::{GameEvent, PacketHandler};

pub struct QuestHandler;

impl QuestHandler {
    pub fn new() -> Self {
        Self
    }
}

impl PacketHandler for QuestHandler {
    fn handle(&self, header: &PacketHeader, _payload: &[u8]) -> Vec<GameEvent> {
        let mut events = Vec::new();
        
        match header.opcode as u16 {
            // Quest related opcodes - to be implemented when quest packets are defined
            _ => {
                tracing::trace!("Quest packet: {:04X}", header.opcode);
                events.push(GameEvent::UnhandledPacket { opcode: header.opcode });
            }
        }
        
        events
    }
}

impl Default for QuestHandler {
    fn default() -> Self {
        Self::new()
    }
}
