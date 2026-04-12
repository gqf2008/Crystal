// Mail Handler - 邮件系统数据包处理

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
            // ReceiveMail
            x if x == ServerPacketIds::ReceiveMail as u16 => {
                if let Ok(packet) = server::ReceiveMail::read_body(&mut cursor) {
                    events.push(NetworkEvent::MailReceived {
                        mails: packet.mail_list.clone(),
                    });
                    tracing::debug!("📬 Mail received: {} mails", packet.mail_list.len());
                }
            }

            // MailLockedItem
            x if x == ServerPacketIds::MailLockedItem as u16 => {
                if let Ok(_packet) = server::MailLockedItem::read_body(&mut cursor) {
                    events.push(NetworkEvent::MailLockedItemReceived);
                    tracing::debug!("🔒 Mail locked item received");
                }
            }

            // MailSendRequest
            x if x == ServerPacketIds::MailSendRequest as u16 => {
                if let Ok(_packet) = server::MailSendRequest::read_body(&mut cursor) {
                    events.push(NetworkEvent::MailSendRequestReceived);
                    tracing::debug!("📤 Mail send request received");
                }
            }

            // MailSent
            x if x == ServerPacketIds::MailSent as u16 => {
                if let Ok(_packet) = server::MailSent::read_body(&mut cursor) {
                    events.push(NetworkEvent::MailSentEvent);
                    tracing::debug!("📬 Mail sent event");
                }
            }

            // ParcelCollected
            x if x == ServerPacketIds::ParcelCollected as u16 => {
                if let Ok(_packet) = server::ParcelCollected::read_body(&mut cursor) {
                    events.push(NetworkEvent::ParcelCollectedEvent);
                    tracing::debug!("📦 Parcel collected event");
                }
            }

            // MailCost
            x if x == ServerPacketIds::MailCost as u16 => {
                if let Ok(packet) = server::MailCost::read_body(&mut cursor) {
                    events.push(NetworkEvent::MailCostReceived {
                        cost: packet.cost,
                    });
                    tracing::debug!("💰 Mail cost received: {}", packet.cost);
                }
            }

            _ => {
                tracing::debug!("⚠️ MailHandler: Unknown opcode {:04X}", header.opcode);
                events.push(NetworkEvent::UnhandledPacket { opcode: header.opcode });
            }
        }

        events
    }
}
