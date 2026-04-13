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
                if let Ok(_packet) = server::HeroCreateRequest::read_body(&mut cursor) {
                    events.push(NetworkEvent::HeroCreateRequested);
                    tracing::debug!("🦸 Hero create requested");
                }
            }

            // NewHero
            x if x == ServerPacketIds::NewHero as u16 => {
                if let Ok(_packet) = server::NewHero::read_body(&mut cursor) {
                    events.push(NetworkEvent::NewHeroCreated);
                    tracing::debug!("🦸 New hero created");
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
                if let Ok(_packet) = server::UnlockHeroAutoPot::read_body(&mut cursor) {
                    events.push(NetworkEvent::HeroAutoPotUnlocked);
                    tracing::debug!("🦸 Hero auto-pot unlocked");
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
                        item_id: packet.item_id as u32,
                    });
                    tracing::debug!("🦸 Hero auto-pot item set: item_id={}", packet.item_id);
                }
            }

            // SetHeroBehaviour
            x if x == ServerPacketIds::SetHeroBehaviour as u16 => {
                if let Ok(packet) = server::SetHeroBehaviour::read_body(&mut cursor) {
                    events.push(NetworkEvent::HeroBehaviourSet {
                        behaviour: packet.attack_mode as u8,
                    });
                    tracing::debug!("🦸 Hero behaviour set: attack_mode={:?}, pet_mode={:?}", packet.attack_mode, packet.pet_mode);
                }
            }

            // ManageHeroes
            x if x == ServerPacketIds::ManageHeroes as u16 => {
                if let Ok(_packet) = server::ManageHeroes::read_body(&mut cursor) {
                    events.push(NetworkEvent::HeroManageReceived);
                    tracing::debug!("🦸 Hero manage received");
                }
            }

            // ChangeHero
            x if x == ServerPacketIds::ChangeHero as u16 => {
                if let Ok(_packet) = server::ChangeHero::read_body(&mut cursor) {
                    events.push(NetworkEvent::HeroChanged);
                    tracing::debug!("🦸 Hero changed");
                }
            }

            // HeroBaseStatsInfo
            x if x == ServerPacketIds::HeroBaseStatsInfo as u16 => {
                if let Ok(_packet) = server::HeroBaseStatsInfo::read_body(&mut cursor) {
                    events.push(NetworkEvent::HeroBaseStatsReceived);
                    tracing::debug!("🦸 Hero base stats received");
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
