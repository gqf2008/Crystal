use bevy::prelude::*;
use mir2_shared::packets::base::{Packet, PacketHeader};
use crate::network::*;
use crate::ui::login::AuthFeedback;
use super::*;

// 网络包解码分派（#72 拆分；#1148 再按域拆分）：handle_world 处理服务端包 地图/对象/移动/战斗/聊天 分支。
// 由 packets.rs::handle_packet 调度器按 opcode 调用；返回 true 表示已处理。

#[allow(clippy::too_many_arguments, unused_variables)]
pub(crate) fn handle_world(    net: &mut NetConnection,
    session: &mut SessionState,
    auth: &mut AuthFeedback,
    game_data: &mut GameData,
    net_objects: &mut MessageWriter<NetObject>,
    net_removals: &mut MessageWriter<NetObjectRemoved>,
    motions: &mut MessageWriter<NetMotion>,
    combat_evt: &mut MessageWriter<CombatEvent>,
    effects: &mut MessageWriter<PendingEffect>,
    server_events: &mut MessageWriter<ServerEvent>,
    control: &mut ControlState,
    next: &mut NextState<AppState>,
    payload: &[u8],) -> bool {
    use mir2_shared::packets::server::*;

    let mut cur = std::io::Cursor::new(payload);
    let Ok(header) = PacketHeader::read_from(&mut cur) else {
        return false;
    };
    let opcode = header.opcode;
    const HANDLED: &[i16] = &[ServerPacketIds::MapChanged as i16, ServerPacketIds::NewMapInfo as i16, ServerPacketIds::Roll as i16, ServerPacketIds::ObjectPlayer as i16, ServerPacketIds::ObjectMonster as i16, ServerPacketIds::ObjectNpc as i16, ServerPacketIds::ObjectRemove as i16, ServerPacketIds::ObjectItem as i16, ServerPacketIds::ObjectGold as i16, ServerPacketIds::UserDashAttack as i16, ServerPacketIds::ObjectDashAttack as i16, ServerPacketIds::TeleportIn as i16, ServerPacketIds::SetConcentration as i16, ServerPacketIds::SetElemental as i16, ServerPacketIds::ColourChanged as i16, ServerPacketIds::ObjectGuildNameChanged as i16, ServerPacketIds::ObjectName as i16, ServerPacketIds::UserName as i16, ServerPacketIds::InTrapRock as i16, ServerPacketIds::TransformUpdate as i16, ServerPacketIds::ObjectSneaking as i16, ServerPacketIds::ObjectLevelEffects as i16, ServerPacketIds::ObjectDeco as i16, ServerPacketIds::NPCUpdate as i16, ServerPacketIds::NPCImageUpdate as i16, ServerPacketIds::ObjectHarvest as i16, ServerPacketIds::ObjectHarvested as i16, ServerPacketIds::TimeOfDay as i16, ServerPacketIds::ObjectTurn as i16, ServerPacketIds::ObjectWalk as i16, ServerPacketIds::ObjectRun as i16, ServerPacketIds::ObjectHide as i16, ServerPacketIds::ObjectShow as i16, ServerPacketIds::ObjectSitDown as i16, ServerPacketIds::Pushed as i16, ServerPacketIds::ObjectPushed as i16, ServerPacketIds::ObjectTeleportOut as i16, ServerPacketIds::ObjectTeleportIn as i16, ServerPacketIds::ObjectAttack as i16, ServerPacketIds::UserDash as i16, ServerPacketIds::ObjectDash as i16, ServerPacketIds::UserDashFail as i16, ServerPacketIds::ObjectDashFail as i16, ServerPacketIds::UserBackStep as i16, ServerPacketIds::ObjectBackStep as i16, ServerPacketIds::UserAttackMove as i16, ServerPacketIds::Poisoned as i16, ServerPacketIds::ObjectPoisoned as i16, ServerPacketIds::Chat as i16, ServerPacketIds::ObjectChat as i16];
    let handled = HANDLED.contains(&opcode);
    match opcode {
        x if x == ServerPacketIds::MapChanged as i16 => {
            if let Ok(p) = map::MapChanged::read_body(&mut cur) {
                tracing::info!(
                    "🗺️ MapChanged: {} ({},{})",
                    p.file_name,
                    p.location_x,
                    p.location_y
                );
                game_data.desired_map = Some(p.file_name);
                game_data.player_spawn =
                    Some((p.location_x as f32, p.location_y as f32, p.direction));
                server_events.write(ServerEvent::WeatherChanged { code: p.weather });
                next.set(AppState::Game);
            }
        }
        x if x == ServerPacketIds::NewMapInfo as i16 => {
            if let Ok(p) = map::NewMapInfo::read_body(&mut cur) {
                tracing::info!(
                    "🗺️ NewMapInfo: map={} title={} npcs={}",
                    p.map_index,
                    p.title,
                    p.npcs.len()
                );
                let npcs: Vec<crate::game::dialogs::big_map::NpcRow> = p
                    .npcs
                    .into_iter()
                    .map(|n| crate::game::dialogs::big_map::NpcRow {
                        object_id: n.object_id,
                        name: n.name,
                        x: n.location_x,
                        y: n.location_y,
                        icon: n.icon,
                        can_teleport_to: n.can_teleport_to,
                    })
                    .collect();
                server_events.write(ServerEvent::MapInfo {
                    map_index: p.map_index,
                    title: p.title.clone(),
                    npcs,
                });
            }
        }
        x if x == ServerPacketIds::Roll as i16 => {
            if let Ok(p) = mir2_shared::packets::server::ui_events::Roll::read_body(&mut cur) {
                tracing::info!(
                    "🎲 Roll: type={} result={} page={} auto={}",
                    p.r#type,
                    p.result,
                    p.page,
                    p.auto_roll
                );
                server_events.write(ServerEvent::Roll {
                    r#type: p.r#type,
                    page: p.page,
                    result: p.result,
                    auto_roll: p.auto_roll,
                    visible: true,
                    started_at: 0.0,
                    finished: false,
                });
            }
        }
        x if x == ServerPacketIds::ObjectPlayer as i16 => {
            match objects::ObjectPlayer::read_body(&mut cur) {
            Ok(p) => {
                net_objects.write(NetObject::Player {
                    object_id: p.object_id,
                    name: p.name,
                    class: p.class,
                    gender: p.gender,
                    location_x: p.location_x,
                    location_y: p.location_y,
                    direction: p.direction as u8,
                    hair: p.hair,
                    weapon: p.weapon,
                    weapon_effect: p.weapon_effect,
                    armour: p.armour,
                    wing_effect: p.wing_effect,
                    mount_type: p.mount_type,
                    is_mounted: p.riding_mount,
                });
            }
            Err(e) => {
                tracing::warn!("⚠️ ObjectPlayer 解析失败: {} (len={})", e, payload.len());
            }
            }
        }
        x if x == ServerPacketIds::ObjectMonster as i16 => {
            if let Ok(p) = objects::ObjectMonster::read_body(&mut cur) {
                net_objects.write(NetObject::Monster {
                    object_id: p.object_id,
                    name: p.name,
                    location_x: p.location_x,
                    location_y: p.location_y,
                    image: p.image,
                    direction: p.direction as u8,
                });
            }
        }
        x if x == ServerPacketIds::ObjectNpc as i16 => {
            if let Ok(p) = objects::ObjectNpc::read_body(&mut cur) {
                net_objects.write(NetObject::Npc {
                    object_id: p.object_id,
                    name: p.name,
                    image: p.image,
                    location_x: p.location_x,
                    location_y: p.location_y,
                    direction: p.direction as u8,
                });
            }
        }
        x if x == ServerPacketIds::ObjectRemove as i16 => {
            if let Ok(p) = objects::ObjectRemove::read_body(&mut cur) {
                tracing::debug!("🗑️ ObjectRemove id={}", p.object_id);
                net_removals.write(NetObjectRemoved(p.object_id));
            }
        }
        x if x == ServerPacketIds::ObjectItem as i16 => {
            match drops::ObjectItem::read_body(&mut cur) {
            Ok(p) => {
                let name = p
                    .item
                    .info
                    .as_ref()
                    .map(|i| i.name.clone())
                    .unwrap_or_else(|| format!("#{}", p.item.item_index));
                tracing::info!(
                    "📦 地面物品: {} (uid={}) @ ({},{})",
                    name,
                    p.item.unique_id,
                    p.location_x,
                    p.location_y
                );
                net_objects.write(NetObject::GroundItem {
                    object_id: p.object_id,
                    item: to_inv_item(&p.item),
                    location_x: p.location_x,
                    location_y: p.location_y,
                });
            }
            Err(e) => tracing::warn!("⚠️ ObjectItem 解析失败: {} (len={})", e, payload.len()),
            }
        }
        x if x == ServerPacketIds::ObjectGold as i16 => {
            // #244：地面金币（C# ItemObject.Load(S.ObjectGold)）
            if let Ok(p) = drops::ObjectGold::read_body(&mut cur) {
                tracing::info!(
                    "💰 地面金币: {} @ ({},{})",
                    p.gold,
                    p.location_x,
                    p.location_y
                );
                net_objects.write(NetObject::GroundGold {
                    object_id: p.object_id,
                    gold: p.gold,
                    location_x: p.location_x,
                    location_y: p.location_y,
                });
            }
        }
        x if x == ServerPacketIds::UserDashAttack as i16 => {
            if let Ok(p) = movement::UserDashAttack::read_body(&mut cur) {
                let pid = session.local_player_id.unwrap_or(100);
                combat_evt.write(CombatEvent::Attack {
                    object_id: pid,
                    direction: p.direction as u8,
                });
                server_events.write(ServerEvent::ObjectPushed {
                    object_id: pid,
                    x: p.location_x,
                    y: p.location_y,
                    direction: p.direction as u8,
                });
            }
        }
        x if x == ServerPacketIds::ObjectDashAttack as i16 => {
            if let Ok(p) = movement::ObjectDashAttack::read_body(&mut cur) {
                combat_evt.write(CombatEvent::Attack {
                    object_id: p.object_id,
                    direction: p.direction as u8,
                });
                server_events.write(ServerEvent::ObjectPushed {
                    object_id: p.object_id,
                    x: p.location_x,
                    y: p.location_y,
                    direction: p.direction as u8,
                });
                tracing::debug!("💨 对象冲刺攻击 id={}", p.object_id);
            }
        }
        x if x == ServerPacketIds::TeleportIn as i16 => {
            // 空包：本地玩家传送出现特效
            let pid = session.local_player_id.unwrap_or(100);
            effects.write(PendingEffect::Burst {
                target_id: pid,
                color: [1.0, 1.0, 1.0],
            });
            tracing::info!("🌀 本地传送出现");
        }
        x if x == ServerPacketIds::SetConcentration as i16 => {
            if let Ok(p) = movement::SetConcentration::read_body(&mut cur) {
                tracing::debug!("🧘 集中 id={} enabled={}", p.object_id, p.enabled);
            }
        }
        x if x == ServerPacketIds::SetElemental as i16 => {
            if let Ok(p) = movement::SetElemental::read_body(&mut cur) {
                tracing::debug!(
                    "⚗️ 元素 id={} el={} value={}",
                    p.object_id,
                    p.element,
                    p.value
                );
            }
        }
        x if x == ServerPacketIds::ColourChanged as i16 => {
            if let Ok(p) = buff::ColourChanged::read_body(&mut cur) {
                tracing::debug!("🎨 名字颜色 {:08x}", p.name_colour_argb);
            }
        }
        x if x == ServerPacketIds::ObjectGuildNameChanged as i16 => {
            if let Ok(p) = buff::ObjectGuildNameChanged::read_body(&mut cur) {
                tracing::info!("🏴 对象行会名 id={} -> {}", p.object_id, p.guild_name);
            }
        }
        x if x == ServerPacketIds::ObjectName as i16 => {
            if let Ok(p) = player::ObjectName::read_body(&mut cur) {
                server_events.write(ServerEvent::ObjectName {
                    object_id: p.object_id,
                    name: p.name,
                });
            }
        }
        x if x == ServerPacketIds::UserName as i16 => {
            if let Ok(p) = miscellaneous::UserName::read_body(&mut cur) {
                server_events.write(ServerEvent::PlayerNameUpdated { name: p.name });
            }
        }
        x if x == ServerPacketIds::InTrapRock as i16 => {
            if let Ok(p) = miscellaneous::InTrapRock::read_body(&mut cur) {
                tracing::debug!("🪤 陷阱状态 in_trap={}", p.in_trap);
            }
        }
        x if x == ServerPacketIds::TransformUpdate as i16 => {
            if let Ok(p) = social_system::TransformUpdate::read_body(&mut cur) {
                tracing::info!("🐉 变身 id={} type={}", p.object_id, p.transform_type);
            }
        }
        x if x == ServerPacketIds::ObjectSneaking as i16 => {
            if let Ok(p) = movement::ObjectSneaking::read_body(&mut cur) {
                if p.sneaking {
                    server_events.write(ServerEvent::ObjectHidden {
                        object_id: p.object_id,
                    });
                } else {
                    server_events.write(ServerEvent::ObjectShown {
                        object_id: p.object_id,
                    });
                }
                tracing::info!("🥷 对象潜行 id={} sneaking={}", p.object_id, p.sneaking);
            }
        }
        x if x == ServerPacketIds::ObjectLevelEffects as i16 => {
            if let Ok(p) = movement::ObjectLevelEffects::read_body(&mut cur) {
                effects.write(PendingEffect::Burst {
                    target_id: p.object_id,
                    color: [1.0, 0.9, 0.3],
                });
                tracing::info!(
                    "✨ 对象等级特效 id={} flags={:04x}",
                    p.object_id,
                    p.level_effects
                );
            }
        }
        x if x == ServerPacketIds::ObjectDeco as i16 => {
            if let Ok(p) = movement::ObjectDeco::read_body(&mut cur) {
                tracing::debug!(
                    "🎀 对象装饰 id={} deco={} remove={}",
                    p.object_id,
                    p.deco,
                    p.remove
                );
            }
        }
        x if x == ServerPacketIds::NPCUpdate as i16 => {
            if let Ok(p) = npc_interaction::NPCUpdate::read_body(&mut cur) {
                tracing::debug!("🧙 NPC 更新 id={}", p.npc_id);
            }
        }
        x if x == ServerPacketIds::NPCImageUpdate as i16 => {
            if let Ok(p) = npc_interaction::NPCImageUpdate::read_body(&mut cur) {
                server_events.write(ServerEvent::NpcImageUpdated {
                    npc_id: p.npc_id,
                    image: p.image,
                });
                tracing::info!("🧙 NPC 形象更新 id={} image={}", p.npc_id, p.image);
            }
        }
        x if x == ServerPacketIds::ObjectHarvest as i16 => {
            if let Ok(p) = objects::ObjectHarvest::read_body(&mut cur) {
                combat_evt.write(CombatEvent::Harvest {
                    object_id: p.object_id,
                    direction: p.direction as u8,
                });
                server_events.write(ServerEvent::ObjectPushed {
                    object_id: p.object_id,
                    x: p.location_x,
                    y: p.location_y,
                    direction: p.direction as u8,
                });
                tracing::debug!(
                    "🌾 对象采集 id={} ({},{})",
                    p.object_id,
                    p.location_x,
                    p.location_y
                );
            }
        }
        x if x == ServerPacketIds::ObjectHarvested as i16 => {
            if let Ok(p) = objects::ObjectHarvested::read_body(&mut cur) {
                combat_evt.write(CombatEvent::Harvest {
                    object_id: p.object_id,
                    direction: p.direction as u8,
                });
                server_events.write(ServerEvent::ObjectPushed {
                    object_id: p.object_id,
                    x: p.location_x,
                    y: p.location_y,
                    direction: p.direction as u8,
                });
                tracing::debug!(
                    "🌾 对象采集完成 id={} ({},{})",
                    p.object_id,
                    p.location_x,
                    p.location_y
                );
            }
        }
        x if x == ServerPacketIds::TimeOfDay as i16 => {
            // C# S.TimeOfDay.Lights（SharedRust LightSetting 值 3..7）
            if let Ok(p) = TimeOfDay::read_body(&mut cur) {
                if let Ok(light) = mir2_shared::enums::LightSetting::try_from(p.lights) {
                    server_events.write(ServerEvent::TimeOfDay { light });
                    tracing::info!("🌗 服务端昼夜: {:?}", light);
                }
            }
        }
        x if x == ServerPacketIds::ObjectTurn as i16 => {
            if let Ok(p) = objects::ObjectTurn::read_body(&mut cur) {
                motions.write(NetMotion::Turn {
                    object_id: p.object_id,
                    x: p.location_x,
                    y: p.location_y,
                    dir: p.direction as u8,
                });
            }
        }
        x if x == ServerPacketIds::ObjectWalk as i16 => {
            if let Ok(p) = objects::ObjectWalk::read_body(&mut cur) {
                tracing::debug!("🚶 ObjectWalk id={} -> ({},{})", p.object_id, p.location_x, p.location_y);
                motions.write(NetMotion::Walk {
                    object_id: p.object_id,
                    x: p.location_x,
                    y: p.location_y,
                    dir: p.direction as u8,
                });
            }
        }
        x if x == ServerPacketIds::ObjectRun as i16 => {
            if let Ok(p) = objects::ObjectRun::read_body(&mut cur) {
                motions.write(NetMotion::Run {
                    object_id: p.object_id,
                    x: p.location_x,
                    y: p.location_y,
                    dir: p.direction as u8,
                });
            }
        }
        x if x == ServerPacketIds::ObjectHide as i16 => {
            if let Ok(p) = map::ObjectHide::read_body(&mut cur) {
                server_events.write(ServerEvent::ObjectHidden {
                    object_id: p.object_id,
                });
                tracing::debug!("🙈 对象隐藏 id={}", p.object_id);
            }
        }
        x if x == ServerPacketIds::ObjectShow as i16 => {
            if let Ok(p) = map::ObjectShow::read_body(&mut cur) {
                server_events.write(ServerEvent::ObjectShown {
                    object_id: p.object_id,
                });
                tracing::debug!("🙉 对象显形 id={}", p.object_id);
            }
        }
        x if x == ServerPacketIds::ObjectSitDown as i16 => {
            if let Ok(p) = miscellaneous::ObjectSitDown::read_body(&mut cur) {
                server_events.write(ServerEvent::ObjectSitDown {
                    object_id: p.object_id,
                    direction: p.direction,
                    sitting: p.sitting,
                });
                tracing::debug!("🪑 对象坐下 id={} sitting={}", p.object_id, p.sitting);
            }
        }
        x if x == ServerPacketIds::Pushed as i16 => {
            if let Ok(p) = combat::Pushed::read_body(&mut cur) {
                let pid = session.local_player_id.unwrap_or(100);
                server_events.write(ServerEvent::ObjectPushed {
                    object_id: pid,
                    x: p.location_x as i32,
                    y: p.location_y as i32,
                    direction: p.direction,
                });
                tracing::debug!("💨 玩家被击退 ({},{})", p.location_x, p.location_y);
            }
        }
        x if x == ServerPacketIds::ObjectPushed as i16 => {
            if let Ok(p) = combat::ObjectPushed::read_body(&mut cur) {
                server_events.write(ServerEvent::ObjectPushed {
                    object_id: p.object_id,
                    x: p.location_x as i32,
                    y: p.location_y as i32,
                    direction: p.direction,
                });
                tracing::debug!(
                    "💨 对象被击退 id={} ({},{})",
                    p.object_id,
                    p.location_x,
                    p.location_y
                );
            }
        }
        x if x == ServerPacketIds::ObjectTeleportOut as i16 => {
            if let Ok(p) = map::ObjectTeleportOut::read_body(&mut cur) {
                server_events.write(ServerEvent::ObjectTeleportOut {
                    object_id: p.object_id,
                });
                tracing::debug!("🌀 对象传送消失 id={}", p.object_id);
            }
        }
        x if x == ServerPacketIds::ObjectTeleportIn as i16 => {
            if let Ok(p) = map::ObjectTeleportIn::read_body(&mut cur) {
                server_events.write(ServerEvent::ObjectTeleportIn {
                    object_id: p.object_id,
                });
                tracing::debug!("🌀 对象传送出现 id={}", p.object_id);
            }
        }
        x if x == ServerPacketIds::ObjectAttack as i16 => {
            if let Ok(p) = combat::ObjectAttack::read_body(&mut cur) {
                combat_evt.write(CombatEvent::Attack {
                    object_id: p.object_id,
                    direction: p.direction,
                });
                tracing::debug!("⚔️ 对象攻击 id={} dir={}", p.object_id, p.direction);
            }
        }
        x if x == ServerPacketIds::UserDash as i16 => {
            if let Ok(p) = combat::UserDash::read_body(&mut cur) {
                let pid = session.local_player_id.unwrap_or(100);
                server_events.write(ServerEvent::ObjectPushed {
                    object_id: pid,
                    x: p.location_x as i32,
                    y: p.location_y as i32,
                    direction: p.direction,
                });
                tracing::debug!("💨 玩家冲刺 ({},{})", p.location_x, p.location_y);
            }
        }
        x if x == ServerPacketIds::ObjectDash as i16 => {
            if let Ok(p) = combat::ObjectDash::read_body(&mut cur) {
                server_events.write(ServerEvent::ObjectPushed {
                    object_id: p.object_id,
                    x: p.location_x as i32,
                    y: p.location_y as i32,
                    direction: p.direction,
                });
                tracing::debug!(
                    "💨 对象冲刺 id={} ({},{})",
                    p.object_id,
                    p.location_x,
                    p.location_y
                );
            }
        }
        x if x == ServerPacketIds::UserDashFail as i16 => {
            if let Ok(p) = combat::UserDashFail::read_body(&mut cur) {
                tracing::debug!("🚫 玩家冲刺失败 ({},{})", p.location_x, p.location_y);
            }
        }
        x if x == ServerPacketIds::ObjectDashFail as i16 => {
            if let Ok(p) = combat::ObjectDashFail::read_body(&mut cur) {
                tracing::debug!("🚫 对象冲刺失败 id={}", p.object_id);
            }
        }
        x if x == ServerPacketIds::UserBackStep as i16 => {
            if let Ok(p) = movement::UserBackStep::read_body(&mut cur) {
                let pid = session.local_player_id.unwrap_or(100);
                server_events.write(ServerEvent::ObjectPushed {
                    object_id: pid,
                    x: p.location_x,
                    y: p.location_y,
                    direction: p.direction as u8,
                });
                tracing::debug!("↩️ 玩家后跳 ({},{})", p.location_x, p.location_y);
            }
        }
        x if x == ServerPacketIds::ObjectBackStep as i16 => {
            if let Ok(p) = movement::ObjectBackStep::read_body(&mut cur) {
                server_events.write(ServerEvent::ObjectPushed {
                    object_id: p.object_id,
                    x: p.location_x,
                    y: p.location_y,
                    direction: p.direction as u8,
                });
                tracing::debug!(
                    "↩️ 对象后跳 id={} ({},{})",
                    p.object_id,
                    p.location_x,
                    p.location_y
                );
            }
        }
        x if x == ServerPacketIds::UserAttackMove as i16 => {
            if let Ok(p) = movement::UserAttackMove::read_body(&mut cur) {
                let pid = session.local_player_id.unwrap_or(100);
                server_events.write(ServerEvent::ObjectPushed {
                    object_id: pid,
                    x: p.location_x,
                    y: p.location_y,
                    direction: p.direction as u8,
                });
                tracing::debug!("🏃 玩家攻击移动 ({},{})", p.location_x, p.location_y);
            }
        }
        x if x == ServerPacketIds::Poisoned as i16 => {
            if let Ok(p) = buff::Poisoned::read_body(&mut cur) {
                let pid = session.local_player_id.unwrap_or(100);
                server_events.write(ServerEvent::ObjectPoisoned {
                    object_id: pid,
                    poisoned: !p.poison.is_empty(),
                });
                tracing::debug!("☠️ 玩家中毒: {:?}", p.poison);
            }
        }
        x if x == ServerPacketIds::ObjectPoisoned as i16 => {
            if let Ok(p) = buff::ObjectPoisoned::read_body(&mut cur) {
                server_events.write(ServerEvent::ObjectPoisoned {
                    object_id: p.object_id,
                    poisoned: !p.poison.is_empty(),
                });
                tracing::debug!("☠️ 对象中毒 id={}: {:?}", p.object_id, p.poison);
            }
        }
        x if x == ServerPacketIds::Chat as i16 => {
            if let Ok(p) = chat::Chat::read_body(&mut cur) {
                server_events.write(crate::network::server_event::from_packet::chat(&p));
            }
        }
        x if x == ServerPacketIds::ObjectChat as i16 => {
            if let Ok(p) = chat::ObjectChat::read_body(&mut cur) {
                server_events.write(crate::network::server_event::from_packet::object_chat(&p));
            }
        }
        _ => {}
    }
    handled
}
