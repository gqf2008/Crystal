// UI Events Handler - UI/表现层相关数据包处理
//
// 目前仅实现：PlaySound (266)

use crate::network::handlers::{NetworkEvent, PacketHandler};
use mir2_shared::enums::ServerPacketIds;
use mir2_shared::packets::{server, Packet, PacketHeader};
use std::io::Cursor;

/// UI / 表现层事件 handler
pub struct UiEventsHandler;

impl PacketHandler for UiEventsHandler {
    fn handle(&self, header: &PacketHeader, payload: &[u8]) -> Vec<NetworkEvent> {
        let mut events = Vec::new();
        let mut cursor = Cursor::new(payload);

        match header.opcode as u16 {
            x if x == ServerPacketIds::PlaySound as u16 => {
                if let Ok(packet) = server::PlaySound::read_body(&mut cursor) {
                    events.push(NetworkEvent::PlaySound {
                        sound_id: packet.sound_id,
                    });
                    tracing::debug!("🔊 PlaySound received id={}", packet.sound_id);
                } else {
                    events.push(NetworkEvent::UnhandledPacket { opcode: header.opcode });
                }
            }

            x if x == ServerPacketIds::MountUpdate as u16 => {
                if let Ok(packet) = server::MountUpdate::read_body(&mut cursor) {
                    events.push(NetworkEvent::MountUpdated {
                        object_id: packet.object_id,
                        mount_type: packet.mount_type,
                        riding_mount: packet.riding_mount,
                    });
                    tracing::debug!(
                        "🐎 MountUpdate received object_id={} mount_type={} riding={}",
                        packet.object_id,
                        packet.mount_type,
                        packet.riding_mount
                    );
                } else {
                    events.push(NetworkEvent::UnhandledPacket { opcode: header.opcode });
                }
            }

            // ====================================================================
            // Timer Events
            // ====================================================================

            // SetTimer
            x if x == ServerPacketIds::SetTimer as u16 => {
                if let Ok(packet) = server::SetTimer::read_body(&mut cursor) {
                    events.push(NetworkEvent::TimerSet {
                        timer_id: packet.timer_id as u8,
                        seconds: packet.seconds as u32,
                    });
                    tracing::debug!("⏱️ SetTimer: id={} seconds={}", packet.timer_id, packet.seconds);
                } else {
                    events.push(NetworkEvent::UnhandledPacket { opcode: header.opcode });
                }
            }

            // ExpireTimer
            x if x == ServerPacketIds::ExpireTimer as u16 => {
                if let Ok(packet) = server::ExpireTimer::read_body(&mut cursor) {
                    events.push(NetworkEvent::TimerExpired {
                        timer_id: packet.timer_id as u8,
                    });
                    tracing::debug!("⏰ TimerExpired: id={}", packet.timer_id);
                } else {
                    events.push(NetworkEvent::UnhandledPacket { opcode: header.opcode });
                }
            }

            // UpdateNotice
            x if x == ServerPacketIds::UpdateNotice as u16 => {
                if let Ok(packet) = server::UpdateNotice::read_body(&mut cursor) {
                    let notice = packet.notices.join("\n");
                    events.push(NetworkEvent::NoticeUpdated {
                        notice: notice.clone(),
                    });
                    tracing::debug!("📢 NoticeUpdated: {} notices", packet.notices.len());
                } else {
                    events.push(NetworkEvent::UnhandledPacket { opcode: header.opcode });
                }
            }

            // Roll
            x if x == ServerPacketIds::Roll as u16 => {
                if let Ok(packet) = server::Roll::read_body(&mut cursor) {
                    events.push(NetworkEvent::RollReceivedEvent {
                        value: packet.result as u32,
                    });
                    tracing::debug!("🎲 RollReceived: result={}", packet.result);
                } else {
                    events.push(NetworkEvent::UnhandledPacket { opcode: header.opcode });
                }
            }

            // SetCompass
            x if x == ServerPacketIds::SetCompass as u16 => {
                if let Ok(packet) = server::SetCompass::read_body(&mut cursor) {
                    events.push(NetworkEvent::CompassUpdated {
                        location: packet.location,
                    });
                    tracing::debug!("🧭 CompassUpdated: location={:?}", packet.location);
                } else {
                    events.push(NetworkEvent::UnhandledPacket { opcode: header.opcode });
                }
            }

            // OpenBrowser
            x if x == ServerPacketIds::OpenBrowser as u16 => {
                if let Ok(packet) = server::OpenBrowser::read_body(&mut cursor) {
                    events.push(NetworkEvent::BrowserOpened {
                        url: packet.url.clone(),
                    });
                    tracing::debug!("🌐 BrowserOpened: {}", packet.url);
                } else {
                    events.push(NetworkEvent::UnhandledPacket { opcode: header.opcode });
                }
            }

            // FishingUpdate
            x if x == ServerPacketIds::FishingUpdate as u16 => {
                if let Ok(packet) = server::FishingUpdate::read_body(&mut cursor) {
                    events.push(NetworkEvent::FishingStatusUpdated {
                        state: packet.fishing_progress as u8,
                    });
                    tracing::debug!("🎣 FishingStatusUpdated: progress={}", packet.fishing_progress);
                } else {
                    events.push(NetworkEvent::UnhandledPacket { opcode: header.opcode });
                }
            }

            // Rankings
            x if x == ServerPacketIds::Rankings as u16 => {
                if let Ok(_packet) = server::Rankings::read_body(&mut cursor) {
                    events.push(NetworkEvent::RankingsReceived);
                    tracing::debug!("🏆 RankingsReceived");
                } else {
                    events.push(NetworkEvent::UnhandledPacket { opcode: header.opcode });
                }
            }

            // GameShopInfo
            x if x == ServerPacketIds::GameShopInfo as u16 => {
                if let Ok(_packet) = server::GameShopInfo::read_body(&mut cursor) {
                    events.push(NetworkEvent::GameShopInfoReceived);
                    tracing::debug!("🛒 GameShopInfoReceived");
                } else {
                    events.push(NetworkEvent::UnhandledPacket { opcode: header.opcode });
                }
            }

            // GameShopStock
            x if x == ServerPacketIds::GameShopStock as u16 => {
                if let Ok(_packet) = server::GameShopStock::read_body(&mut cursor) {
                    events.push(NetworkEvent::GameShopStockReceived);
                    tracing::debug!("📦 GameShopStockReceived");
                } else {
                    events.push(NetworkEvent::UnhandledPacket { opcode: header.opcode });
                }
            }

            // NewRecipeInfo
            x if x == ServerPacketIds::NewRecipeInfo as u16 => {
                if let Ok(_packet) = server::NewRecipeInfo::read_body(&mut cursor) {
                    tracing::debug!("📜 NewRecipeInfo received");
                } else {
                    events.push(NetworkEvent::UnhandledPacket { opcode: header.opcode });
                }
            }

            _ => {
                events.push(NetworkEvent::UnhandledPacket { opcode: header.opcode });
            }
        }

        events
    }
}
