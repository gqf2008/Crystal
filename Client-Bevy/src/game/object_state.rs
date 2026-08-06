// ============================================================================
// 对象状态表现层（#226）
// 网络驱动：ObjectHide/ObjectShow/ObjectSitDown/Pushed/ObjectPushed/
//           ObjectTeleportOut/ObjectTeleportIn → ServerEvent
// 绘制参考：Client-Macroquad/src（对象隐身/传送/击退表现）
// ============================================================================

use bevy::prelude::*;

use crate::actor::{
    ActorAnim, ActorAppearance, MonsterName, MountState, NpcAppearance, NpcName, NetObjectId,
    PlayerName, SpriteLayer,
};
use crate::game::movement::tile_to_world;
use crate::network::server_event::ServerEvent;
use crate::scenes::AppState;

pub struct ObjectStatePlugin;

/// #279：服务端怪物/NPC 信息缓存（NewMonsterInfo / NewNPCInfo）
#[derive(Resource, Default)]
pub struct InfoCache {
    pub monsters: std::collections::HashMap<i32, mir2_shared::data::client_data::ClientMonsterInfo>,
    pub npcs: std::collections::HashMap<u32, mir2_shared::data::client_data::ClientNPCInfo>,
}

impl Plugin for ObjectStatePlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<InfoCache>();
        app.add_systems(
            Update,
            (
                apply_object_state_events,
                apply_player_update_events,
                apply_info_cache_events,
            )
                .after(crate::network::network_system)
                .run_if(in_state(AppState::Game)),
        );
    }
}

/// 消费对象状态事件：隐藏/显形/坐下/击退/传送进出/坐骑上马下马
#[allow(clippy::too_many_arguments)]
fn apply_object_state_events(
    mut commands: Commands,
    mut events: MessageReader<ServerEvent>,
    mut vis: Query<(&NetObjectId, &mut Visibility)>,
    mut anim: Query<(&NetObjectId, &mut ActorAnim)>,
    mut transforms: Query<(&NetObjectId, &mut Transform)>,
    mounts: Query<(Entity, &NetObjectId, Option<&MountState>)>,
    poisons: Query<(Entity, &NetObjectId, Option<&crate::actor::PoisonTint>)>,
    mut npcs: Query<(Entity, &NetObjectId, &mut NpcAppearance)>,
    mut name_actors: Query<
        (
            Entity,
            &NetObjectId,
            Option<&mut PlayerName>,
            Option<&mut MonsterName>,
            Option<&mut NpcName>,
        ),
        Without<crate::actor::LocalPlayer>,
    >,
    mut local_names: Query<(Entity, Option<&mut PlayerName>), With<crate::actor::LocalPlayer>>,
    mut name_labels: Query<&mut Text2d, With<crate::actor::ActorNameLabel>>,
    children: Query<&Children>,
    mut layers: Query<&mut SpriteLayer>,
    mut effects: MessageWriter<crate::game::effects::PendingEffect>,
) {
    let pending: Vec<ServerEvent> = events.read().cloned().collect();
    if pending.is_empty() {
        return;
    }
    for ev in pending {
        match ev {
            ServerEvent::ObjectHidden { object_id } => {
                let found = vis.iter().any(|(id, _)| id.0 == object_id);
                tracing::debug!("[OBJSTATE] 隐藏 id={} found={}", object_id, found);
                for (id, mut v) in &mut vis {
                    if id.0 == object_id {
                        *v = Visibility::Hidden;
                        break;
                    }
                }
            }
            ServerEvent::ObjectShown { object_id } => {
                for (id, mut v) in &mut vis {
                    if id.0 == object_id {
                        *v = Visibility::Visible;
                        break;
                    }
                }
            }
            ServerEvent::ObjectSitDown {
                object_id,
                direction,
            } => {
                let found = anim.iter().any(|(id, _)| id.0 == object_id);
                tracing::debug!(
                    "[OBJSTATE] 坐下 id={} dir={} found={}",
                    object_id,
                    direction,
                    found
                );
                // 有 SitDown 帧表才切动作，否则只更新朝向（避免动画冻结）
                let has_sit = crate::objects::frames::get_player_frame(
                    mir2_shared::enums::MirAction::SitDown,
                )
                .is_some();
                for (id, mut a) in &mut anim {
                    if id.0 == object_id {
                        a.direction = direction;
                        if has_sit {
                            a.action = mir2_shared::enums::MirAction::SitDown;
                            a.frame_index = 0;
                        }
                        break;
                    }
                }
            }
            ServerEvent::ObjectPushed {
                object_id,
                x,
                y,
                direction,
            } => {
                let to = tile_to_world(x, y);
                for (id, mut tf) in &mut transforms {
                    if id.0 == object_id {
                        tf.translation.x = to.x;
                        tf.translation.y = to.y;
                        break;
                    }
                }
                for (id, mut a) in &mut anim {
                    if id.0 == object_id {
                        a.direction = direction;
                        break;
                    }
                }
            }
            ServerEvent::ObjectTeleportOut { object_id } => {
                // 传送消失：白紫色爆点 + 隐藏
                effects.write(crate::game::effects::PendingEffect::Burst {
                    target_id: object_id,
                    color: [0.8, 0.7, 1.0],
                });
                for (id, mut v) in &mut vis {
                    if id.0 == object_id {
                        *v = Visibility::Hidden;
                        break;
                    }
                }
            }
            ServerEvent::ObjectTeleportIn { object_id } => {
                effects.write(crate::game::effects::PendingEffect::Burst {
                    target_id: object_id,
                    color: [0.8, 0.7, 1.0],
                });
                for (id, mut v) in &mut vis {
                    if id.0 == object_id {
                        *v = Visibility::Visible;
                        break;
                    }
                }
            }
            ServerEvent::ObjectName { object_id, name } => {
                // #264：对象改名 → 更新名字组件 + 头顶标签文本
                let mut label_entity = None;
                for (ent, id, mut p, mut m, mut n) in &mut name_actors {
                    if id.0 == object_id {
                        if let Some(p) = p.as_mut() {
                            p.0 = name.clone();
                        } else if let Some(m) = m.as_mut() {
                            m.0 = name.clone();
                        } else if let Some(n) = n.as_mut() {
                            n.0 = name.clone();
                        }
                        label_entity = Some(ent);
                        tracing::info!("🏷️ 对象改名 id={} -> {}", object_id, name);
                        break;
                    }
                }
                if let Some(ent) = label_entity {
                    if let Ok(children_of) = children.get(ent) {
                        for c in children_of.iter() {
                            if let Ok(mut t) = name_labels.get_mut(c) {
                                t.0 = name.clone();
                            }
                        }
                    }
                }
            }
            ServerEvent::PlayerNameUpdated { name } => {
                // #264：本地玩家改名（HudState.name 由 hud 更新；这里同步名字组件）
                for (_ent, mut p) in &mut local_names {
                    if let Some(p) = p.as_mut() {
                        p.0 = name.clone();
                    }
                }
            }
            ServerEvent::NpcImageUpdated { npc_id, image } => {
                // #248：NPC 形象更新 → NpcAppearance + 子层 slot（Npcs 库帧号）
                for (ent, id, mut app) in &mut npcs {
                    if id.0 == npc_id {
                        app.npc_index = image;
                        if let Ok(children_of) = children.get(ent) {
                            for c in children_of.iter() {
                                if let Ok(mut l) = layers.get_mut(c) {
                                    if l.lib == crate::resources::libraries::ArrayLibType::Npcs {
                                        l.slot = image as u32;
                                    }
                                }
                            }
                        }
                        tracing::info!("🧙 NPC 形象更新 id={} image={}", npc_id, image);
                        break;
                    }
                }
            }
            ServerEvent::ObjectPoisoned {
                object_id,
                poisoned,
            } => {
                // #236：中毒 → 挂 PoisonTint（渲染染绿）；清除 → 移除
                for (ent, id, tint) in &poisons {
                    if id.0 == object_id {
                        if poisoned && tint.is_none() {
                            commands.entity(ent).insert(crate::actor::PoisonTint);
                            tracing::info!("☠️ 对象 {} 中毒（绿色染层）", object_id);
                        } else if !poisoned && tint.is_some() {
                            commands.entity(ent).remove::<crate::actor::PoisonTint>();
                            tracing::info!("💚 对象 {} 毒解", object_id);
                        }
                        break;
                    }
                }
            }
            ServerEvent::MountUpdated {
                object_id,
                mount_type,
                is_mounted,
            } => {
                // #232：上马插入 MountState + 坐骑层；下马移除
                if is_mounted && mount_type >= 0 {
                    let target = mounts
                        .iter()
                        .find(|(_, id, m)| id.0 == object_id && m.is_none())
                        .map(|(e, _, _)| e);
                    if let Some(ent) = target {
                        commands.entity(ent).insert(MountState { mount_type });
                        crate::actor::attach_mount_layer(&mut commands, ent, mount_type);
                        tracing::info!("🐴 对象 {} 上马 type={}", object_id, mount_type);
                    }
                } else {
                    let target = mounts
                        .iter()
                        .find(|(_, id, m)| id.0 == object_id && m.is_some())
                        .map(|(e, _, _)| e);
                    if let Some(ent) = target {
                        commands.entity(ent).remove::<MountState>();
                        if let Ok(children_of) = children.get(ent) {
                            for c in children_of.iter() {
                                if let Ok(l) = layers.get(c) {
                                    if l.is_mount {
                                        commands.entity(c).despawn();
                                    }
                                }
                            }
                        }
                        tracing::info!("🐴 对象 {} 下马", object_id);
                    }
                }
            }
            _ => {}
        }
    }
}

/// #279：PlayerUpdate → 更新 ActorAppearance（换装/光照；本地玩家外观由 sync_player_equipment 处理）
fn apply_player_update_events(
    mut events: MessageReader<ServerEvent>,
    mut actors: Query<(&NetObjectId, &mut ActorAppearance)>,
) {
    for ev in events.read() {
        if let ServerEvent::PlayerUpdate {
            object_id,
            weapon,
            weapon_effect,
            armor,
            wings_effect,
            ..
        } = ev
        {
            for (id, mut app) in &mut actors {
                if id.0 == *object_id {
                    app.weapon = *weapon;
                    app.weapon_effect = *weapon_effect;
                    app.armour = (*armor).max(0) as u16;
                    app.wing_effect = *wings_effect;
                    tracing::info!(
                        "🧍 外观更新 id={} weapon={} armor={}",
                        object_id,
                        weapon,
                        armor
                    );
                }
            }
        }
    }
}

/// #279：NewMonsterInfo / NewNPCInfo → 信息缓存（供渲染/查询）
fn apply_info_cache_events(mut events: MessageReader<ServerEvent>, mut cache: ResMut<InfoCache>) {
    for ev in events.read() {
        match ev {
            ServerEvent::MonsterInfo { info } => {
                tracing::info!("👹 怪物信息 #{} {}", info.index, info.name);
                cache.monsters.insert(info.index, info.clone());
            }
            ServerEvent::NpcInfo { info } => {
                tracing::info!("🧙 NPC 信息 id={} {}", info.object_id, info.name);
                cache.npcs.insert(info.object_id, info.clone());
            }
            _ => {}
        }
    }
}
