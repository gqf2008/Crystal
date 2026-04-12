// Guild Handler - 公会相关数据包处理

use mir2_shared::packets::{PacketHeader, Packet, server};
use mir2_shared::enums::ServerPacketIds;
use crate::network::handlers::{NetworkEvent, PacketHandler};
use std::io::Cursor;

pub struct GuildHandler;

impl PacketHandler for GuildHandler {
    fn handle(&self, header: &PacketHeader, payload: &[u8]) -> Vec<NetworkEvent> {
        let mut events = Vec::new();
        let mut cursor = Cursor::new(payload);

        match header.opcode as u16 {
            // GuildInvite
            x if x == ServerPacketIds::GuildInvite as u16 => {
                if let Ok(packet) = server::GuildInvite::read_body(&mut cursor) {
                    events.push(NetworkEvent::GuildInvite {
                        inviter: String::new(),  // GuildInvite只有guild_name字段
                        guild_name: packet.guild_name.clone(),
                    });
                    tracing::info!("🏛️ Guild invite to join {}", packet.guild_name);
                }
            }

            // GuildStatus (joined)
            x if x == ServerPacketIds::GuildStatus as u16 => {
                if let Ok(packet) = server::GuildStatus::read_body(&mut cursor) {
                    events.push(NetworkEvent::GuildJoined {
                        guild_name: packet.guild_name.clone(),
                    });
                    tracing::info!("🏛️ Guild status updated: {} ({})", packet.guild_name, packet.rank_name);
                }
            }

            // GuildNoticeChange
            x if x == ServerPacketIds::GuildNoticeChange as u16 => {
                if let Ok(packet) = server::GuildNoticeChange::read_body(&mut cursor) {
                    let notice = packet.notice.join("\n");
                    events.push(NetworkEvent::GuildNoticeUpdated { notice });
                    tracing::info!("🏛️ Guild notice updated");
                }
            }

            // GuildMemberChange
            x if x == ServerPacketIds::GuildMemberChange as u16 => {
                if let Ok(packet) = server::GuildMemberChange::read_body(&mut cursor) {
                    let online = packet.status <= 5; // status > 5 means offline/left
                    events.push(NetworkEvent::GuildMemberUpdated {
                        name: packet.name.clone(),
                        rank: packet.rank_index,
                        online,
                    });
                    tracing::info!("🏛️ Guild member updated: {} (rank={}, online={})", packet.name, packet.rank_index, online);
                }
            }

            // GuildExpGain
            x if x == ServerPacketIds::GuildExpGain as u16 => {
                if let Ok(packet) = server::GuildExpGain::read_body(&mut cursor) {
                    events.push(NetworkEvent::GuildExpGained {
                        amount: packet.amount as i64,
                    });
                    tracing::info!("🏛️ Guild exp gained: {}", packet.amount);
                }
            }

            // GuildNameRequest
            x if x == ServerPacketIds::GuildNameRequest as u16 => {
                if let Ok(_packet) = server::GuildNameRequest::read_body(&mut cursor) {
                    events.push(NetworkEvent::GuildNameReceived {
                        name: String::new(),
                    });
                    tracing::info!("🏛️ Guild name received");
                }
            }

            // GuildStorageGoldChange
            x if x == ServerPacketIds::GuildStorageGoldChange as u16 => {
                if let Ok(packet) = server::GuildStorageGoldChange::read_body(&mut cursor) {
                    events.push(NetworkEvent::GuildStorageGoldChanged {
                        delta: packet.change as i64,
                    });
                    tracing::info!("🏛️ Guild storage gold changed: {}", packet.change);
                }
            }

            // GuildStorageItemChange
            x if x == ServerPacketIds::GuildStorageItemChange as u16 => {
                if let Ok(packet) = server::GuildStorageItemChange::read_body(&mut cursor) {
                    events.push(NetworkEvent::GuildStorageItemChanged {
                        change_type: packet.change_type,
                        slot: packet.slot,
                    });
                    tracing::info!("🏛️ Guild storage item changed: type={} slot={}", packet.change_type, packet.slot);
                }
            }

            // GuildStorageList
            x if x == ServerPacketIds::GuildStorageList as u16 => {
                if let Ok(_packet) = server::GuildStorageList::read_body(&mut cursor) {
                    events.push(NetworkEvent::GuildStorageListReceived);
                    tracing::info!("🏛️ Guild storage list received");
                }
            }

            // GuildRequestWar
            x if x == ServerPacketIds::GuildRequestWar as u16 => {
                if let Ok(packet) = server::GuildRequestWar::read_body(&mut cursor) {
                    tracing::warn!("🏛️ Guild war requested by: {}", packet.guild_name);
                    events.push(NetworkEvent::GuildWarRequested);
                }
            }

            // GuildBuffList
            x if x == ServerPacketIds::GuildBuffList as u16 => {
                if let Ok(packet) = server::GuildBuffList::read_body(&mut cursor) {
                    events.push(NetworkEvent::GuildBuffListReceived { buff_ids: packet.active_buffs.clone() });
                    tracing::info!("🏛️ Guild buff list received: {} buffs", packet.active_buffs.len());
                }
            }

            // GuildTerritoryPage
            x if x == ServerPacketIds::GuildTerritoryPage as u16 => {
                if let Ok(_packet) = server::GuildTerritoryPage::read_body(&mut cursor) {
                    events.push(NetworkEvent::GuildTerritoryPageReceived);
                    tracing::info!("🏛️ Guild territory page received");
                }
            }

            // PurchaseGuildTerritory
            x if x == ServerPacketIds::PurchaseGuildTerritory as u16 => {
                if let Ok(packet) = server::PurchaseGuildTerritory::read_body(&mut cursor) {
                    tracing::info!("🏛️ Guild territory purchase: {}", if packet.success { "success" } else { "failed" });
                    events.push(NetworkEvent::GuildTerritoryPurchased);
                }
            }

            _ => {
                tracing::debug!("⚠️ GuildHandler: Unknown opcode {:04X}", header.opcode);
                events.push(NetworkEvent::UnhandledPacket { opcode: header.opcode });
            }
        }

        events
    }
}
