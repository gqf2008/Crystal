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
            x if x == ServerPacketIds::TradeRequest as u16 => {
                if let Ok(_packet) = server::TradeRequest::read_body(&mut cursor) {
                    events.push(NetworkEvent::TradeRequested {
                        requester: String::new(),
                    });
                    tracing::debug!("💱 Trade request received");
                }
            }
            x if x == ServerPacketIds::TradeAccept as u16 => {
                if let Ok(_packet) = server::TradeAccept::read_body(&mut cursor) {
                    events.push(NetworkEvent::TradeStarted {
                        partner: String::new(),
                    });
                    tracing::debug!("💱 Trade accepted");
                }
            }
            x if x == ServerPacketIds::TradeItem as u16 => {
                if let Ok(packet) = server::TradeItem::read_body(&mut cursor) {
                    events.push(NetworkEvent::TradeItemReceived {
                        trade_items: packet.trade_items,
                    });
                    tracing::debug!("💱 Trade items updated");
                }
            }
            x if x == ServerPacketIds::TradeGold as u16 => {
                if let Ok(packet) = server::TradeGold::read_body(&mut cursor) {
                    events.push(NetworkEvent::TradeGoldReceived {
                        amount: packet.amount,
                    });
                    tracing::debug!("💱 Trade gold: {}", packet.amount);
                }
            }
            x if x == ServerPacketIds::TradeConfirm as u16 => {
                if let Ok(_packet) = server::TradeConfirm::read_body(&mut cursor) {
                    events.push(NetworkEvent::TradeConfirmed);
                    tracing::debug!("💱 Trade confirmed");
                }
            }
            x if x == ServerPacketIds::TradeCancel as u16 => {
                if let Ok(packet) = server::TradeCancel::read_body(&mut cursor) {
                    events.push(NetworkEvent::TradeCancelled);
                    tracing::debug!("💱 Trade cancelled (unlock={})", packet.unlock);
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
    fn test_trade_handler_unhandled() {
        let handler = TradeHandler;
        let events = handler.handle(&PacketHeader::new(0, 9999), &[]);
        assert_eq!(events.len(), 1);
        assert!(matches!(events[0], NetworkEvent::UnhandledPacket { opcode: 9999 }));
    }

    #[test]
    fn test_trade_handler_confirm() {
        let handler = TradeHandler;
        let opcode = ServerPacketIds::TradeConfirm as i16;
        let events = handler.handle(&PacketHeader::new(0, opcode), &[]);
        assert!(events.iter().any(|e| matches!(e, NetworkEvent::TradeConfirmed)));
    }
}
