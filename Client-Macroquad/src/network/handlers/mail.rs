// Mail Handler - 邮件系统相关数据包处理

use mir2_shared::packets::{PacketHeader, Packet, server};
use mir2_shared::enums::ServerPacketIds;
use crate::network::handlers::{NetworkEvent, PacketHandler};
use std::io::Cursor;

pub struct MailHandler;

impl PacketHandler for MailHandler {
    fn handle(&self, header: &PacketHeader, payload: &[u8]) -> Vec<NetworkEvent> {
        let mut events = Vec::new();
        let mut cursor = Cursor::new(payload);
        
        match header.opcode as u16 {
            x if x == ServerPacketIds::ReceiveMail as u16 => {
                if let Ok(packet) = server::ReceiveMail::read_body(&mut cursor) {
                    let count = packet.mail_list.len();
                    events.push(NetworkEvent::MailListReceived {
                        mail_list: packet.mail_list,
                    });
                    tracing::debug!("📧 Mail list received: {} mails", count);
                }
            }
            x if x == ServerPacketIds::MailSendRequest as u16 => {
                if let Ok(packet) = server::MailSendRequest::read_body(&mut cursor) {
                    events.push(NetworkEvent::MailSendRequested {
                        mail_id: packet.mail_id,
                    });
                    tracing::debug!("📧 Mail send requested: id={}", packet.mail_id);
                }
            }
            x if x == ServerPacketIds::MailSent as u16 => {
                if let Ok(packet) = server::MailSent::read_body(&mut cursor) {
                    events.push(NetworkEvent::MailSentResult {
                        mail_id: packet.mail_id,
                        result: packet.result,
                    });
                    tracing::debug!("📧 Mail sent: id={} result={}", packet.mail_id, packet.result);
                }
            }
            x if x == ServerPacketIds::ParcelCollected as u16 => {
                if let Ok(packet) = server::ParcelCollected::read_body(&mut cursor) {
                    events.push(NetworkEvent::MailCollected {
                        mail_id: packet.mail_id,
                        success: packet.success,
                    });
                    tracing::debug!("📧 Parcel collected: id={} success={}", packet.mail_id, packet.success);
                }
            }
            x if x == ServerPacketIds::MailCost as u16 => {
                if let Ok(packet) = server::MailCost::read_body(&mut cursor) {
                    tracing::debug!("📧 Mail cost: {}", packet.cost);
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
    fn test_mail_handler_unhandled() {
        let handler = MailHandler;
        let events = handler.handle(&PacketHeader::new(0, 9999), &[]);
        assert_eq!(events.len(), 1);
        assert!(matches!(events[0], NetworkEvent::UnhandledPacket { opcode: 9999 }));
    }

    #[test]
    fn test_mail_sent() {
        let handler = MailHandler;
        let opcode = ServerPacketIds::MailSent as i16;
        // MailSent reads u64 (mail_id) + u8 (result)
        let mut payload = Vec::new();
        payload.extend_from_slice(&7u64.to_le_bytes());
        payload.push(1u8);
        let events = handler.handle(&PacketHeader::new(9, opcode), &payload);
        assert!(events.iter().any(|e| matches!(e, NetworkEvent::MailSentResult { mail_id: 7, result: 1 })));
    }
}
