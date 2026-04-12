// Market Handler - 市场/寄售数据包处理

use mir2_shared::packets::{PacketHeader, Packet, server};
use mir2_shared::enums::ServerPacketIds;
use crate::network::handlers::{NetworkEvent, PacketHandler};
use std::io::Cursor;

pub struct MarketHandler;

impl PacketHandler for MarketHandler {
    fn handle(&self, header: &PacketHeader, payload: &[u8]) -> Vec<NetworkEvent> {
        let mut events = Vec::new();
        let mut cursor = Cursor::new(payload);

        match header.opcode as u16 {
            // NPCConsign
            x if x == ServerPacketIds::NPCConsign as u16 => {
                if let Ok(_packet) = server::NPCConsign::read_body(&mut cursor) {
                    events.push(NetworkEvent::NPCConsignEvent);
                    tracing::debug!("📋 NPC consign event");
                }
            }

            // NPCMarket
            x if x == ServerPacketIds::NPCMarket as u16 => {
                if let Ok(_packet) = server::NPCMarket::read_body(&mut cursor) {
                    events.push(NetworkEvent::NPCMarketEvent2);
                    tracing::debug!("🏪 NPC market event");
                }
            }

            // NPCMarketPage
            x if x == ServerPacketIds::NPCMarketPage as u16 => {
                if let Ok(_packet) = server::NPCMarketPage::read_body(&mut cursor) {
                    events.push(NetworkEvent::NPCMarketPageEvent2);
                    tracing::debug!("🏪 NPC market page event");
                }
            }

            // ConsignItem
            x if x == ServerPacketIds::ConsignItem as u16 => {
                if let Ok(_packet) = server::ConsignItem::read_body(&mut cursor) {
                    events.push(NetworkEvent::ConsignItemEvent);
                    tracing::debug!("📋 Consign item event");
                }
            }

            // MarketFail
            x if x == ServerPacketIds::MarketFail as u16 => {
                if let Ok(packet) = server::MarketFail::read_body(&mut cursor) {
                    events.push(NetworkEvent::MarketFailedEvent2 {
                        reason: format!("reason_code={}", packet.reason),
                    });
                    tracing::warn!("🏪 Market failed: reason={}", packet.reason);
                }
            }

            // MarketSuccess
            x if x == ServerPacketIds::MarketSuccess as u16 => {
                if let Ok(packet) = server::MarketSuccess::read_body(&mut cursor) {
                    events.push(NetworkEvent::MarketSuccessEvent2);
                    tracing::info!("🏪 Market success: {}", packet.message);
                }
            }

            _ => {
                tracing::debug!("⚠️ MarketHandler: Unknown opcode {:04X}", header.opcode);
                events.push(NetworkEvent::UnhandledPacket { opcode: header.opcode });
            }
        }

        events
    }
}
