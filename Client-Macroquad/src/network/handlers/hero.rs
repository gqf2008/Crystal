// Hero Handler - 英雄相关数据包处理

use mir2_shared::packets::{PacketHeader, Packet, server};
use mir2_shared::enums::ServerPacketIds;
use crate::network::handlers::{NetworkEvent, PacketHandler};
use std::io::Cursor;

pub struct HeroHandler;

impl PacketHandler for HeroHandler {
    fn handle(&self, header: &PacketHeader, payload: &[u8]) -> Vec<NetworkEvent> {
        let mut events = Vec::new();
        let mut cursor = Cursor::new(payload);

        match header.opcode as u16 {
            // HeroCreateRequest
            x if x == ServerPacketIds::HeroCreateRequest as u16 => {
                if let Ok(packet) = server::HeroCreateRequest::read_body(&mut cursor) {
                    let count = packet.can_create_class.iter().filter(|&&c| c).count();
                    events.push(NetworkEvent::HeroCreateRequested {
                        can_create_class: packet.can_create_class,
                    });
                    tracing::debug!("🦸 Hero create requested: {} classes available", count);
                }
            }

            // NewHero
            x if x == ServerPacketIds::NewHero as u16 => {
                if let Ok(packet) = server::NewHero::read_body(&mut cursor) {
                    events.push(NetworkEvent::NewHeroCreated { result: packet.result });
                    tracing::debug!("🦸 New hero result: {}", packet.result);
                }
            }

            // HeroInformation
            x if x == ServerPacketIds::HeroInformation as u16 => {
                if let Ok(packet) = server::HeroInformation::read_body(&mut cursor) {
                    events.push(NetworkEvent::HeroInfoReceived { hero_id: packet.hero_id });
                    tracing::debug!("🦸 Hero information: hero_id={}", packet.hero_id);
                }
            }

            // UpdateHeroSpawnState
            x if x == ServerPacketIds::UpdateHeroSpawnState as u16 => {
                if let Ok(packet) = server::UpdateHeroSpawnState::read_body(&mut cursor) {
                    events.push(NetworkEvent::HeroSpawnStateUpdated {
                        state: packet.state as u8,
                    });
                    tracing::debug!("🦸 Hero spawn state updated: {:?}", packet.state);
                }
            }

            // UnlockHeroAutoPot
            x if x == ServerPacketIds::UnlockHeroAutoPot as u16 => {
                if let Ok(packet) = server::UnlockHeroAutoPot::read_body(&mut cursor) {
                    events.push(NetworkEvent::HeroAutoPotUnlocked { unlocked: packet.unlocked });
                    tracing::debug!("🦸 Hero auto-pot unlock: {}", packet.unlocked);
                }
            }

            // SetAutoPotValue
            x if x == ServerPacketIds::SetAutoPotValue as u16 => {
                if let Ok(packet) = server::SetAutoPotValue::read_body(&mut cursor) {
                    events.push(NetworkEvent::HeroAutoPotSet {
                        pot_type: packet.stat,
                        value: packet.value,
                    });
                    tracing::debug!("🦸 Hero auto-pot set: stat={}, value={}", packet.stat, packet.value);
                }
            }

            // SetAutoPotItem
            x if x == ServerPacketIds::SetAutoPotItem as u16 => {
                if let Ok(packet) = server::SetAutoPotItem::read_body(&mut cursor) {
                    events.push(NetworkEvent::HeroAutoPotItemSet {
                        slot: packet.grid as i32,
                        item_id: packet.item_index as u32,
                    });
                    tracing::debug!("🦸 Hero auto-pot item set: item_id={}", packet.item_index);
                }
            }

            // SetHeroBehaviour
            x if x == ServerPacketIds::SetHeroBehaviour as u16 => {
                if let Ok(packet) = server::SetHeroBehaviour::read_body(&mut cursor) {
                    events.push(NetworkEvent::HeroBehaviourSet {
                        behaviour: packet.behaviour as u8,
                    });
                    tracing::debug!("🦸 Hero behaviour set: {:?}", packet.behaviour);
                }
            }

            // ManageHeroes
            x if x == ServerPacketIds::ManageHeroes as u16 => {
                if let Ok(packet) = server::ManageHeroes::read_body(&mut cursor) {
                    let count = packet.heroes.len();
                    events.push(NetworkEvent::HeroManageReceived { heroes: packet.heroes });
                    tracing::debug!("🦸 Hero manage received: {} heroes", count);
                }
            }

            // ChangeHero
            x if x == ServerPacketIds::ChangeHero as u16 => {
                if let Ok(packet) = server::ChangeHero::read_body(&mut cursor) {
                    events.push(NetworkEvent::HeroChanged { success: packet.success });
                    tracing::debug!("🦸 Hero changed: success={}", packet.success);
                }
            }

            // HeroBaseStatsInfo
            x if x == ServerPacketIds::HeroBaseStatsInfo as u16 => {
                if let Ok(packet) = server::HeroBaseStatsInfo::read_body(&mut cursor) {
                    let count = packet.stats.len();
                    events.push(NetworkEvent::HeroBaseStatsReceived { stats: packet.stats });
                    tracing::debug!("🦸 Hero base stats received: {} values", count);
                }
            }

            _ => {
                tracing::debug!("⚠️ HeroHandler: Unknown opcode {:04X}", header.opcode);
                events.push(NetworkEvent::UnhandledPacket { opcode: header.opcode });
            }
        }

        events
    }
}
