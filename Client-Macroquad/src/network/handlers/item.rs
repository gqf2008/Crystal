// Item Handler - 物品相关数据包处理

use mir2_shared::packets::{PacketHeader, Packet, server};
use mir2_shared::enums::ServerPacketIds;
use crate::network::handlers::{NetworkEvent, PacketHandler};
use std::io::Cursor;

pub struct ItemHandler;

impl PacketHandler for ItemHandler {
    fn handle(&self, header: &PacketHeader, payload: &[u8]) -> Vec<NetworkEvent> {
        let mut events = Vec::new();
        let mut cursor = Cursor::new(payload);
        
        match header.opcode as u16 {
            // GainedItem
            x if x == ServerPacketIds::GainedItem as u16 => {
                if let Ok(packet) = server::GainedItem::read_body(&mut cursor) {
                    events.push(NetworkEvent::ItemGained {
                        item: packet.item.clone(),
                    });
                    tracing::debug!("📦 Item gained: {:?}", packet.item);
                }
            }
            
            // DeleteItem
            x if x == ServerPacketIds::DeleteItem as u16 => {
                if let Ok(packet) = server::DeleteItem::read_body(&mut cursor) {
                    events.push(NetworkEvent::ItemLost {
                        unique_id: packet.unique_id,
                    });
                    tracing::debug!("📦 Item lost: {}", packet.unique_id);
                }
            }
            
            // GainedGold
            x if x == ServerPacketIds::GainedGold as u16 => {
                if let Ok(packet) = server::GainedGold::read_body(&mut cursor) {
                    events.push(NetworkEvent::GoldChanged {
                        delta: packet.gold as i32,
                    });
                    tracing::debug!("💰 Gold gained: {}", packet.gold);
                }
            }

            // LoseGold
            x if x == ServerPacketIds::LoseGold as u16 => {
                if let Ok(packet) = server::LoseGold::read_body(&mut cursor) {
                    events.push(NetworkEvent::GoldChanged {
                        delta: -(packet.gold as i32),
                    });
                    tracing::debug!("💸 Gold lost: {}", packet.gold);
                }
            }

            // DuraChanged
            x if x == ServerPacketIds::DuraChanged as u16 => {
                if let Ok(packet) = server::DuraChanged::read_body(&mut cursor) {
                    events.push(NetworkEvent::DuraChanged {
                        unique_id: packet.unique_id,
                        current_dura: packet.current_dura,
                    });
                    tracing::debug!("🔧 Durability changed: id={} dura={}", packet.unique_id, packet.current_dura);
                }
            }

            // CombineItem
            x if x == ServerPacketIds::CombineItem as u16 => {
                if let Ok(packet) = server::CombineItem::read_body(&mut cursor) {
                    events.push(NetworkEvent::ItemCombined {
                        grid: packet.grid,
                        id_from: packet.id_from,
                        id_to: packet.id_to,
                        success: packet.success,
                        destroy: packet.destroy,
                    });
                    tracing::debug!("📦 Item combined: from={} to={} success={}", packet.id_from, packet.id_to, packet.success);
                }
            }

            // ItemUpgraded
            x if x == ServerPacketIds::ItemUpgraded as u16 => {
                if let Ok(packet) = server::ItemUpgraded::read_body(&mut cursor) {
                    events.push(NetworkEvent::ItemUpgraded {
                        item: packet.item.clone(),
                    });
                    tracing::debug!("📦 Item upgraded");
                }
            }
            
            _ => {
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
    fn test_item_handler_unhandled() {
        let handler = ItemHandler;
        let events = handler.handle(&PacketHeader::new(0, 9999), &[]);
        assert_eq!(events.len(), 1);
        assert!(matches!(events[0], NetworkEvent::UnhandledPacket { opcode: 9999 }));
    }

    #[test]
    fn test_dura_changed() {
        let handler = ItemHandler;
        let opcode = ServerPacketIds::DuraChanged as i16;
        // DuraChanged reads u64 (unique_id) + u16 (current_dura)
        let mut payload = Vec::new();
        payload.extend_from_slice(&100u64.to_le_bytes());
        payload.extend_from_slice(&50u16.to_le_bytes());
        let events = handler.handle(&PacketHeader::new(10, opcode), &payload);
        assert!(events.iter().any(|e| matches!(e, NetworkEvent::DuraChanged { unique_id: 100, current_dura: 50 })));
    }
}
