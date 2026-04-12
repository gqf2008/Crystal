// Movement Handler - 移动相关数据包处理

use mir2_shared::packets::{PacketHeader, Packet, server};
use mir2_shared::enums::ServerPacketIds;
use crate::network::handlers::{NetworkEvent, PacketHandler};
use std::io::Cursor;

pub struct MovementHandler;

impl PacketHandler for MovementHandler {
    fn handle(&self, header: &PacketHeader, payload: &[u8]) -> Vec<NetworkEvent> {
        let mut events = Vec::new();
        let mut cursor = Cursor::new(payload);
        
        match header.opcode as u16 {
            // MapInformation
            x if x == ServerPacketIds::MapInformation as u16 => {
                if let Ok(packet) = server::MapInformation::read_body(&mut cursor) {
                    tracing::info!(
                        "🗺️ MapInformation: idx={} file={} title={}",
                        packet.map_index,
                        packet.file_name,
                        packet.title
                    );
                    events.push(NetworkEvent::MapInformation { packet });
                }
            }

            // MapChanged
            x if x == ServerPacketIds::MapChanged as u16 => {
                if let Ok(packet) = server::MapChanged::read_body(&mut cursor) {
                    tracing::info!(
                        "🗺️ MapChanged: idx={} file={} title={} loc=({}, {})",
                        packet.map_index,
                        packet.file_name,
                        packet.title,
                        packet.location_x,
                        packet.location_y
                    );
                    events.push(NetworkEvent::MapChanged { packet });
                }
            }

            // UserLocation
            x if x == ServerPacketIds::UserLocation as u16 => {
                if let Ok(packet) = server::UserLocation::read_body(&mut cursor) {
                    events.push(NetworkEvent::PlayerLocationChanged {
                        x: packet.location_x,
                        y: packet.location_y,
                    });
                    tracing::trace!("📍 User location updated: ({}, {})", 
                        packet.location_x, packet.location_y);
                }
            }

            // ===== Object spawns =====
            x if x == ServerPacketIds::ObjectPlayer as u16 => {
                if let Ok(packet) = server::ObjectPlayer::read_body(&mut cursor) {
                    tracing::trace!(
                        "🧑 ObjectPlayer: id={} name={} armour={} weapon={} wing={} loc=({}, {})",
                        packet.object_id,
                        packet.name,
                        packet.armour,
                        packet.weapon,
                        packet.wing_effect,
                        packet.location_x,
                        packet.location_y
                    );
                    events.push(NetworkEvent::ObjectPlayer { packet });
                }
            }
            x if x == ServerPacketIds::ObjectMonster as u16 => {
                if let Ok(packet) = server::ObjectMonster::read_body(&mut cursor) {
                    tracing::trace!(
                        "👾 ObjectMonster: id={} image={} loc=({}, {})",
                        packet.object_id,
                        packet.image,
                        packet.location_x,
                        packet.location_y
                    );
                    events.push(NetworkEvent::ObjectMonster { packet });
                }
            }
            x if x == ServerPacketIds::ObjectNpc as u16 => {
                if let Ok(packet) = server::ObjectNpc::read_body(&mut cursor) {
                    tracing::trace!(
                        "🧑‍💼 ObjectNpc: id={} image={} loc=({}, {})",
                        packet.object_id,
                        packet.image,
                        packet.location_x,
                        packet.location_y
                    );
                    events.push(NetworkEvent::ObjectNpc { packet });
                }
            }

            // ===== Object lifecycle =====
            x if x == ServerPacketIds::ObjectRemove as u16 => {
                if let Ok(packet) = server::ObjectRemove::read_body(&mut cursor) {
                    events.push(NetworkEvent::ObjectRemove { packet });
                }
            }

            // ===== Object movement =====
            x if x == ServerPacketIds::ObjectTurn as u16 => {
                if let Ok(packet) = server::ObjectTurn::read_body(&mut cursor) {
                    events.push(NetworkEvent::ObjectTurn { packet });
                }
            }
            x if x == ServerPacketIds::ObjectWalk as u16 => {
                if let Ok(packet) = server::ObjectWalk::read_body(&mut cursor) {
                    events.push(NetworkEvent::ObjectWalk { packet });
                }
            }
            x if x == ServerPacketIds::ObjectRun as u16 => {
                if let Ok(packet) = server::ObjectRun::read_body(&mut cursor) {
                    events.push(NetworkEvent::ObjectRun { packet });
                }
            }

            // ===== Object Hero =====
            x if x == ServerPacketIds::ObjectHero as u16 => {
                if let Ok(packet) = server::ObjectHero::read_body(&mut cursor) {
                    tracing::trace!(
                        "🦸 ObjectHero: owner={} id={} name={} loc=({}, {})",
                        packet.owner_name,
                        packet.player.object_id,
                        packet.player.name,
                        packet.player.location_x,
                        packet.player.location_y
                    );
                    events.push(NetworkEvent::ObjectHeroSpawned);
                }
            }

            // ===== Object visibility =====
            x if x == ServerPacketIds::ObjectHide as u16 => {
                if let Ok(packet) = server::ObjectHide::read_body(&mut cursor) {
                    tracing::trace!("👻 ObjectHidden: id={}", packet.object_id);
                    events.push(NetworkEvent::ObjectHidden { object_id: packet.object_id });
                }
            }
            x if x == ServerPacketIds::ObjectShow as u16 => {
                if let Ok(packet) = server::ObjectShow::read_body(&mut cursor) {
                    tracing::trace!("👁️ ObjectShown: id={}", packet.object_id);
                    events.push(NetworkEvent::ObjectShown { object_id: packet.object_id });
                }
            }

            // ===== Teleportation =====
            x if x == ServerPacketIds::ObjectTeleportOut as u16 => {
                if let Ok(packet) = server::ObjectTeleportOut::read_body(&mut cursor) {
                    tracing::trace!(
                        "✨ ObjectTeleportingOut: id={} type={}",
                        packet.object_id, packet.teleport_type
                    );
                    events.push(NetworkEvent::ObjectTeleportingOut { object_id: packet.object_id });
                }
            }
            x if x == ServerPacketIds::ObjectTeleportIn as u16 => {
                if let Ok(packet) = server::ObjectTeleportIn::read_body(&mut cursor) {
                    tracing::trace!(
                        "✨ ObjectTeleportingIn: id={} type={}",
                        packet.object_id, packet.teleport_type
                    );
                    events.push(NetworkEvent::ObjectTeleportingIn);
                }
            }
            x if x == ServerPacketIds::TeleportIn as u16 => {
                // TeleportIn has an empty body
                let _ = server::TeleportIn::read_body(&mut cursor);
                tracing::trace!("🌀 PlayerTeleportedIn");
                events.push(NetworkEvent::PlayerTeleportedIn);
            }

            // ===== BackStep =====
            x if x == ServerPacketIds::UserBackStep as u16 => {
                if let Ok(packet) = server::UserBackStep::read_body(&mut cursor) {
                    tracing::trace!("🔙 PlayerBackStepped: loc=({}, {})", packet.location_x, packet.location_y);
                    events.push(NetworkEvent::PlayerBackStepped {
                        x: packet.location_x,
                        y: packet.location_y,
                    });
                }
            }
            x if x == ServerPacketIds::ObjectBackStep as u16 => {
                if let Ok(packet) = server::ObjectBackStep::read_body(&mut cursor) {
                    tracing::trace!(
                        "🔙 ObjectBackStepped: id={} loc=({}, {}) dist={}",
                        packet.object_id, packet.location_x, packet.location_y, packet.distance
                    );
                    events.push(NetworkEvent::ObjectBackStepped);
                }
            }

            // ===== Dash =====
            x if x == ServerPacketIds::UserDash as u16 => {
                if let Ok(packet) = server::UserDash::read_body(&mut cursor) {
                    tracing::trace!("💨 PlayerDashing: loc=({}, {})", packet.location_x, packet.location_y);
                    events.push(NetworkEvent::PlayerDashing {
                        x: packet.location_x as i32,
                        y: packet.location_y as i32,
                    });
                }
            }
            x if x == ServerPacketIds::ObjectDash as u16 => {
                if let Ok(packet) = server::ObjectDash::read_body(&mut cursor) {
                    tracing::trace!(
                        "💨 ObjectDashing: id={} loc=({}, {})",
                        packet.object_id, packet.location_x, packet.location_y
                    );
                    events.push(NetworkEvent::ObjectDashing);
                }
            }
            x if x == ServerPacketIds::UserDashFail as u16 => {
                if let Ok(packet) = server::UserDashFail::read_body(&mut cursor) {
                    tracing::trace!(
                        "❌ PlayerDashFailed: loc=({}, {})",
                        packet.location_x, packet.location_y
                    );
                    events.push(NetworkEvent::PlayerDashFailed);
                }
            }
            x if x == ServerPacketIds::ObjectDashFail as u16 => {
                if let Ok(packet) = server::ObjectDashFail::read_body(&mut cursor) {
                    tracing::trace!(
                        "❌ ObjectDashFailed: id={} loc=({}, {})",
                        packet.object_id, packet.location_x, packet.location_y
                    );
                    events.push(NetworkEvent::ObjectDashFailed { object_id: packet.object_id });
                }
            }

            // ===== Sit Down =====
            x if x == ServerPacketIds::ObjectSitDown as u16 => {
                if let Ok(packet) = server::ObjectSitDown::read_body(&mut cursor) {
                    tracing::trace!(
                        "🪑 ObjectSatDown: id={} loc=({:?}) dir={}",
                        packet.object_id, packet.location, packet.direction
                    );
                    events.push(NetworkEvent::ObjectSatDown { object_id: packet.object_id });
                }
            }

            // ===== Map & World (extended) =====
            x if x == ServerPacketIds::NewMapInfo as u16 => {
                if let Ok(packet) = server::NewMapInfo::read_body(&mut cursor) {
                    tracing::info!(
                        "🗺️ NewMapInfo: idx={} title={} size={}x{} movements={} npcs={}",
                        packet.map_index, packet.title, packet.width, packet.height,
                        packet.movements.len(), packet.npcs.len()
                    );
                    events.push(NetworkEvent::NewMapInfoReceived);
                }
            }
            x if x == ServerPacketIds::WorldMapSetup as u16 => {
                if let Ok(packet) = server::WorldMapSetupInfo::read_body(&mut cursor) {
                    tracing::info!(
                        "🌍 WorldMapSetupReceived: {} map icons",
                        packet.world_maps.len()
                    );
                    events.push(NetworkEvent::WorldMapSetupReceived);
                }
            }
            x if x == ServerPacketIds::SearchMapResult as u16 => {
                if let Ok(packet) = server::SearchMapResult::read_body(&mut cursor) {
                    tracing::info!(
                        "🔍 SearchMapResultReceived: map={} loc=({}, {})",
                        packet.map_index, packet.location_x, packet.location_y
                    );
                    events.push(NetworkEvent::SearchMapResultReceived);
                }
            }
            x if x == ServerPacketIds::TimeOfDay as u16 => {
                if let Ok(packet) = server::TimeOfDay::read_body(&mut cursor) {
                    tracing::trace!("🌗 TimeOfDayChanged: lights={}", packet.lights);
                    events.push(NetworkEvent::TimeOfDayChanged { time_of_day: packet.lights });
                }
            }

            _ => {
                tracing::debug!("⚠️ MovementHandler: Unknown opcode {:04X}", header.opcode);
                events.push(NetworkEvent::UnhandledPacket { opcode: header.opcode });
            }
        }
        
        events
    }
}
