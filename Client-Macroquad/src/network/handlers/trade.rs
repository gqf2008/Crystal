// Trade Handler - 交易相关数据包处理

use mir2_shared::packets::{PacketHeader, Packet, server};
use mir2_shared::enums::ServerPacketIds;
use crate::network::handlers::{NetworkEvent, PacketHandler};
use std::io::Cursor;

pub struct TradeHandler;

impl PacketHandler for TradeHandler {
    fn handle(&self, header: &PacketHeader, payload: &[u8]) -> Vec<NetworkEvent> {
        let mut events = Vec::new();
        let mut cursor = Cursor::new(payload);

        match header.opcode as u16 {
            // TradeRequest - 收到交易请求
            x if x == ServerPacketIds::TradeRequest as u16 => {
                if let Ok(packet) = server::TradeRequest::read_body(&mut cursor) {
                    events.push(NetworkEvent::TradeRequested {
                        requester: packet.name.clone(),
                    });
                    tracing::debug!("💱 Trade request from: {}", packet.name);
                }
            }

            // TradeAccept - 对方接受了交易
            x if x == ServerPacketIds::TradeAccept as u16 => {
                if let Ok(packet) = server::TradeAccept::read_body(&mut cursor) {
                    events.push(NetworkEvent::TradeStarted {
                        partner: packet.name.clone(),
                    });
                    tracing::debug!("💱 Trade accepted by: {}", packet.name);
                }
            }

            // TradeGold - 交易中添加的金币
            x if x == ServerPacketIds::TradeGold as u16 => {
                if let Ok(packet) = server::TradeGold::read_body(&mut cursor) {
                    events.push(NetworkEvent::TradeGoldAdded {
                        amount: packet.amount,
                    });
                    tracing::debug!("💱 Trade gold added: {}", packet.amount);
                }
            }

            // TradeItem - 交易中添加的物品
            x if x == ServerPacketIds::TradeItem as u16 => {
                if let Ok(_packet) = server::TradeItem::read_body(&mut cursor) {
                    events.push(NetworkEvent::TradeItemAdded);
                    tracing::debug!("💱 Trade item added");
                }
            }

            // TradeConfirm - 对方确认交易
            x if x == ServerPacketIds::TradeConfirm as u16 => {
                if let Ok(_packet) = server::TradeConfirm::read_body(&mut cursor) {
                    events.push(NetworkEvent::TradeConfirmedEvent { locked: false });
                    tracing::debug!("💱 Trade confirmed");
                }
            }

            // TradeCancel - 对方取消交易/解锁
            x if x == ServerPacketIds::TradeCancel as u16 => {
                if let Ok(_packet) = server::TradeCancel::read_body(&mut cursor) {
                    events.push(NetworkEvent::TradeCancelledEvent);
                    tracing::debug!("💱 Trade cancelled/unlocked");
                }
            }

            _ => {
                tracing::debug!("⚠️ TradeHandler: Unknown opcode {:04X}", header.opcode);
                events.push(NetworkEvent::UnhandledPacket { opcode: header.opcode });
            }
        }

        events
    }
}
