// Item Handler - 物品相关数据包处理

use mir2_shared::packets::{PacketHeader, Packet, server};
use mir2_shared::enums::ServerPacketIds;
use crate::network::handlers::{GameEvent, PacketHandler};
use std::io::Cursor;

pub struct ItemHandler;

impl ItemHandler {
    pub fn new() -> Self {
        Self
    }
}

impl PacketHandler for ItemHandler {
    fn handle(&self, header: &PacketHeader, payload: &[u8]) -> Vec<GameEvent> {
        let mut events = Vec::new();
        let mut cursor = Cursor::new(payload);
        
        match header.opcode as u16 {
            // GainedItem
            x if x == ServerPacketIds::GainedItem as u16 => {
                if let Ok(packet) = server::GainedItem::read_body(&mut cursor) {
                    events.push(GameEvent::ItemGained {
                        item: packet.item.clone(),
                    });
                    tracing::debug!("📦 Item gained: {:?}", packet.item);
                }
            }
            
            // DeleteItem
            x if x == ServerPacketIds::DeleteItem as u16 => {
                if let Ok(packet) = server::DeleteItem::read_body(&mut cursor) {
                    events.push(GameEvent::ItemLost {
                        unique_id: packet.unique_id,
                    });
                    tracing::debug!("📦 Item lost: {}", packet.unique_id);
                }
            }
            
            // GainedGold
            x if x == ServerPacketIds::GainedGold as u16 => {
                if let Ok(packet) = server::GainedGold::read_body(&mut cursor) {
                    events.push(GameEvent::GoldChanged {
                        amount: packet.gold as u32,
                    });
                    tracing::debug!("💰 Gold gained: {}", packet.gold);
                }
            }
            
            _ => {
                events.push(GameEvent::UnhandledPacket { opcode: header.opcode });
            }
        }
        
        events
    }
}

impl Default for ItemHandler {
    fn default() -> Self {
        Self::new()
    }
}
