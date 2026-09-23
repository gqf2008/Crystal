// ============================================================================
// 对象状态表现层（#226）
// 网络驱动：ObjectHide/ObjectShow/ObjectSitDown/Pushed/ObjectPushed/
//           ObjectTeleportOut/ObjectTeleportIn → ServerEvent
// 绘制参考：Client-Macroquad/src（对象隐身/传送/击退表现）
// ============================================================================

use bevy::ecs::system::SystemParam;
use bevy::prelude::*;

use crate::actor::{
    ActorAnim, ActorAppearance, GhostLayer, LocalPlayer, MonsterName, MountState, NetObjectId,
    NpcAppearance, NpcName, Player, PlayerName, Sitting, SpriteLayer,
};
use crate::game::movement::tile_to_world;
use crate::game::sound::{play_sound_cached, SoundBank, SoundCache};
use crate::map_renderer::GameLibraries;
use crate::network::server_event::ServerEvent;
use crate::resources::libraries::LibraryName;
use crate::scenes::AppState;
use crate::ui::sprite_ui::{ui_image, UiImageCache};

/// #2892：`MapObject.Hidden` 标记（半透明档）——来源：`ObjectPlayer.hidden`（进视野）与
/// `S.ObjectHidden/ObjectShown`（状态变化）。透明度由 `apply_hidden_alpha` 落地。
#[derive(Component, Clone, Copy, PartialEq, Eq, Debug)]
pub struct HiddenObject {
    pub hidden: bool,
}

/// C# `MapObject.cs:5006` `DXManager.SetOpacity(0.5F)` / 恢复 1.0
pub fn hidden_alpha(hidden: bool) -> f32 {
    if hidden {
        0.5
    } else {
        1.0
    }
}

/// 把 `object_id` 对应的实体标记为隐藏/显形（找不到实体则跳过：进视野那份走 ObjectPlayer.hidden）
fn set_hidden_flag(
    commands: &mut Commands,
    ids: &Query<(Entity, &NetObjectId)>,
    object_id: u32,
    hidden: bool,
) {
    if let Some((e, _)) = ids.iter().find(|(_, id)| id.0 == object_id) {
        commands.entity(e).insert(HiddenObject { hidden });
    }
}

/// #2892：按 `HiddenObject` 设置**该对象自己**的 sprite 层透明度
/// （C# `MapObject.cs:5006`；含生成/事件两条来源，顺序无关）。
fn apply_hidden_alpha(
    mut objects: Query<(&HiddenObject, &Children, &mut Visibility), Changed<HiddenObject>>,
    mut layers: Query<&mut SpriteLayer>,
) {
    for (h, children, mut vis) in &mut objects {
        // `Hidden` 对象仍然可见（半透明）；「对他人完全消失」是 S.ObjectRemove 的职责
        if *vis != Visibility::Visible {
            *vis = Visibility::Visible;
        }
        let alpha = hidden_alpha(h.hidden);
        for child in children.iter() {
            if let Ok(mut layer) = layers.get_mut(child) {
                if layer.alpha != alpha {
                    layer.alpha = alpha;
                }
            }
        }
    }
}

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
                apply_level_up_fx_events,
                advance_level_up_fx,
                // #2892：半透明档落地（排在事件系统之后，`Commands` 已应用）
                apply_hidden_alpha,
            )
                .after(crate::network::network_system)
                .run_if(in_state(AppState::Game)),
        );
    }
}

/// `apply_object_state_events` 图层三件套打包（Bevy 系统 16 参数上限，
/// LESSON_Bevy系统参数上限16 已沉淀的同族处理）
#[derive(SystemParam)]
struct ActorLayerQueries<'w, 's> {
    pub children: Query<'w, 's, &'static Children>,
    pub layers: Query<'w, 's, &'static mut SpriteLayer>,
    pub ghost_layers: Query<'w, 's, &'static GhostLayer>,
}

/// 消费对象状态事件：隐藏/显形/坐下/击退/传送进出/坐骑上马下马
#[allow(clippy::too_many_arguments)]
fn apply_object_state_events(
    mut commands: Commands,
    mut events: MessageReader<ServerEvent>,
    // #2633 批次4 步7：本地判定改读 `NetObjectId`（HudState 已于步9 删除）；
    // 实体缺失视同非本地（原 hud.player_object_id=None 默认）
    local_q: Query<&NetObjectId, With<LocalPlayer>>,
    mut vis: Query<(&NetObjectId, &mut Visibility)>,
    // #2892：事件 → `HiddenObject` 标记（只读 id 查询；与 `vis` 只共享 `NetObjectId` 读访问）
    ids: Query<(Entity, &NetObjectId)>,
    mut anim: Query<(Entity, &NetObjectId, &mut ActorAnim)>,
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
    mut lq: ActorLayerQueries,
    mut effects: MessageWriter<crate::game::effects::PendingEffect>,
) {
    let pending: Vec<ServerEvent> = events.read().cloned().collect();
    if pending.is_empty() {
        return;
    }
    let local_id = local_q.single().ok().map(|id| id.0);
    for ev in pending {
        match ev {
            ServerEvent::ObjectHidden { object_id } => {
                // #2892：对齐 C# `MapObject.cs:5006` —— `Hidden` 对象**一律半透明绘制**
                //（`if (Hidden && !DXManager.Blending) DXManager.SetOpacity(0.5F);`）。
                // 只改本对象的 `HiddenObject` 标记，透明度由 `apply_hidden_alpha` 落到该对象的层上：
                // 旧实现遍历**全部** `SpriteLayer`，会把场上所有对象一起变半透明；
                // 且事件早于对象创建时会丢失（进视野那份由 `ObjectPlayer.hidden` 携带）。
                let is_local = local_id == Some(object_id);
                set_hidden_flag(&mut commands, &ids, object_id, true);
                tracing::debug!(
                    "[OBJSTATE] 隐藏 id={} local={} found={}",
                    object_id,
                    is_local,
                    vis.iter().any(|(id, _)| id.0 == object_id)
                );
            }
            ServerEvent::ObjectShown { object_id } => {
                // 取消隐藏 → 恢复不透明（C# `Hidden = false` 后不再 `SetOpacity(0.5F)`）
                set_hidden_flag(&mut commands, &ids, object_id, false);
            }
            ServerEvent::ObjectSitDown {
                object_id,
                direction,
                sitting,
            } => {
                let found = anim.iter().any(|(_, id, _)| id.0 == object_id);
                tracing::debug!(
                    "[OBJSTATE] 坐下 id={} dir={} sitting={} found={}",
                    object_id,
                    direction,
                    sitting,
                    found
                );
                // 有 SitDown 帧表才切动作，否则只更新朝向（避免动画冻结）
                let has_sit = crate::objects::frames::get_player_frame(
                    mir2_shared::enums::MirAction::SitDown,
                )
                .is_some();
                for (e, id, mut a) in &mut anim {
                    if id.0 == object_id {
                        a.direction = direction;
                        if sitting && has_sit {
                            a.action = mir2_shared::enums::MirAction::SitDown;
                            a.frame_index = 0;
                            // #573：坐下标记——演示驱动不再转向（C# 坐姿对象不自动转身）
                            commands.entity(e).insert(Sitting);
                        } else if !sitting {
                            // #1354：起身——恢复站立动作并移除坐下标记
                            // #3028：换图重建可能已 despawn 该实体 → 落地时复查（见 movement::safe_remove）
                            crate::game::movement::safe_remove::<Sitting>(&mut commands, e);
                            a.action = mir2_shared::enums::MirAction::Standing;
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
                for (_e, id, mut a) in &mut anim {
                    if id.0 == object_id {
                        a.direction = direction;
                        break;
                    }
                }
            }
            ServerEvent::ObjectTeleportOut { object_id, .. } => {
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
            ServerEvent::ObjectTeleportIn {
                object_id,
                location_x,
                location_y,
            } => {
                // 传送出现：瞬移到新位置 + 白紫色爆点 + 显示（C# Teleport 特效+位置）
                effects.write(crate::game::effects::PendingEffect::Burst {
                    target_id: object_id,
                    color: [0.8, 0.7, 1.0],
                });
                let to = tile_to_world(location_x as i32, location_y as i32);
                for (id, mut t) in &mut transforms {
                    if id.0 == object_id {
                        t.translation.x = to.x;
                        t.translation.y = to.y;
                        break;
                    }
                }
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
                    if let Ok(children_of) = lq.children.get(ent) {
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
                        if let Ok(children_of) = lq.children.get(ent) {
                            for c in children_of.iter() {
                                if let Ok(mut l) = lq.layers.get_mut(c) {
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
                        // 与 spawn.rs 初始生成路径同源：坐骑层 + 坐骑 ghost 残影层
                        // 一并移除（ghost 无 SpriteLayer，内联只 despawn is_mount
                        // 图层时 ghost 必泄漏——上马一次叠一个，遮挡时叠画）
                        crate::actor::detach_mount_layers(
                            &mut commands,
                            &lq.children,
                            &lq.layers,
                            &lq.ghost_layers,
                            ent,
                        );
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

/// #283：升级特效（C# Effect(Libraries.Magic2, 1180, 16, 2500, ob)）
#[derive(Component)]
struct LevelUpFx {
    t: f32,
    dur: f32,
    frames: u32,
    /// Magic2 起始帧（1180）
    base: usize,
}

/// #283：ObjectLeveled → 目标对象升级特效 + LevelUp 音效；本地玩家 LevelChanged 同样播放
fn apply_level_up_fx_events(
    mut commands: Commands,
    mut events: MessageReader<ServerEvent>,
    mut libs: ResMut<GameLibraries>,
    mut images: ResMut<Assets<Image>>,
    mut cache: ResMut<UiImageCache>,
    bank: Res<SoundBank>,
    mut sound_cache: ResMut<SoundCache>,
    mut assets: ResMut<Assets<AudioSource>>,
    actors: Query<(&NetObjectId, &Transform)>,
    local: Query<&Transform, (With<LocalPlayer>, With<Player>)>,
) {
    for ev in events.read() {
        match ev {
            ServerEvent::ObjectLeveled { object_id, .. } => {
                for (id, tf) in &actors {
                    if id.0 == *object_id {
                        spawn_level_up_fx(
                            &mut commands,
                            &mut libs,
                            &mut images,
                            &mut cache,
                            tf.translation,
                        );
                        play_sound_cached(
                            &mut commands,
                            &mut assets,
                            &bank,
                            &mut sound_cache,
                            10156,
                        );
                        tracing::info!("✨ 对象 {} 升级特效", object_id);
                    }
                }
            }
            ServerEvent::LevelChanged { .. } => {
                if let Ok(tf) = local.single() {
                    spawn_level_up_fx(
                        &mut commands,
                        &mut libs,
                        &mut images,
                        &mut cache,
                        tf.translation,
                    );
                    play_sound_cached(&mut commands, &mut assets, &bank, &mut sound_cache, 10156);
                    tracing::info!("✨ 本地玩家升级特效");
                }
            }
            _ => {}
        }
    }
}

/// 生成升级特效实体（Magic2[1180..1195] 16 帧 2.5s，跟随对象当前位置）
fn spawn_level_up_fx(
    commands: &mut Commands,
    libs: &mut GameLibraries,
    images: &mut Assets<Image>,
    cache: &mut UiImageCache,
    pos: Vec3,
) {
    let Some(handle) = ui_image(libs, images, cache, LibraryName::Magic2, 1180) else {
        return;
    };
    commands.spawn((
        LevelUpFx {
            t: 0.0,
            dur: 2.5,
            frames: 16,
            base: 1180,
        },
        Sprite {
            image: handle,
            ..default()
        },
        bevy::sprite::Anchor::CENTER,
        Transform::from_translation(pos),
    ));
}

/// 推进升级特效帧并到期销毁
fn advance_level_up_fx(
    time: Res<Time>,
    mut commands: Commands,
    mut libs: ResMut<GameLibraries>,
    mut images: ResMut<Assets<Image>>,
    mut cache: ResMut<UiImageCache>,
    mut q: Query<(Entity, &mut LevelUpFx, &mut Sprite)>,
) {
    for (e, mut fx, mut sprite) in &mut q {
        fx.t += time.delta_secs();
        if fx.t >= fx.dur {
            commands.entity(e).despawn();
            continue;
        }
        let idx = (fx.t / fx.dur * fx.frames as f32).floor() as usize;
        if let Some(h) = ui_image(
            &mut libs,
            &mut images,
            &mut cache,
            LibraryName::Magic2,
            fx.base + idx,
        ) {
            sprite.image = h;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::actor::SpriteLayer;
    use crate::resources::libraries::ArrayLibType;

    fn spawn_actor(app: &mut App, object_id: u32, layer_alphas: &[f32]) -> (Entity, Vec<Entity>) {
        let mut layers = Vec::new();
        let mut obj = app
            .world_mut()
            .spawn((NetObjectId(object_id), Visibility::Visible))
            .id();
        app.world_mut().entity_mut(obj).with_children(|parent| {
            for alpha in layer_alphas {
                layers.push(
                    parent
                        .spawn(SpriteLayer {
                            lib: ArrayLibType::CArmours,
                            slot: 0,
                            frame: 0,
                            is_effect: false,
                            is_mount: false,
                            alpha: *alpha,
                        })
                        .id(),
                );
            }
        });
        obj = app.world().entity(obj).id();
        (obj, layers)
    }

    fn alpha_of(app: &App, e: Entity) -> f32 {
        app.world().entity(e).get::<SpriteLayer>().unwrap().alpha
    }

    /// #2965 审查 P1：`MountUpdated{is_mounted:false}`（运行时下马主路径——
    /// 装备/NPC 脚本广播 MountUpdate，SetMountState 不重发 ObjectPlayer）必须连
    /// 坐骑 ghost 残影层一起移除；旧内联块只 despawn `is_mount` 的 SpriteLayer，
    /// ghost 无 SpriteLayer 必泄漏（再上马叠一个，遮挡时双份 alpha 叠画）。
    ///
    /// 阳性对照：把下马分支退回内联 `layers.get(c)` 循环 → 本测试 ghost 断言 FAILED。
    #[test]
    fn dismount_via_mount_updated_removes_mount_layer_and_ghost() {
        use bevy::ecs::message::Messages;
        use bevy::ecs::system::RunSystemOnce;

        let mut world = World::new();
        world.init_resource::<Messages<ServerEvent>>();
        world.init_resource::<Messages<crate::game::effects::PendingEffect>>();

        // 骑乘态角色：身体层 + 坐骑层 + 身体 ghost + 坐骑 ghost（同生产 spawn 结构）
        let root = world
            .spawn((
                NetObjectId(7),
                MountState { mount_type: 0 },
                Visibility::Visible,
            ))
            .id();
        let (body_layer, mount_layer, body_ghost, mount_ghost) = {
            let mut e = world.entity_mut(root);
            let mut ids = (
                Entity::PLACEHOLDER,
                Entity::PLACEHOLDER,
                Entity::PLACEHOLDER,
                Entity::PLACEHOLDER,
            );
            e.with_children(|p| {
                ids.0 = p
                    .spawn(SpriteLayer {
                        lib: ArrayLibType::CArmours,
                        slot: 0,
                        frame: 0,
                        is_effect: false,
                        is_mount: false,
                        alpha: 1.0,
                    })
                    .id();
                ids.1 = p
                    .spawn(SpriteLayer {
                        lib: ArrayLibType::Mounts,
                        slot: 0,
                        frame: 0,
                        is_effect: false,
                        is_mount: true,
                        alpha: 1.0,
                    })
                    .id();
                ids.2 = p
                    .spawn(GhostLayer {
                        lib: ArrayLibType::CArmours,
                    })
                    .id();
                ids.3 = p
                    .spawn(GhostLayer {
                        lib: ArrayLibType::Mounts,
                    })
                    .id();
            });
            ids
        };

        world.write_message(ServerEvent::MountUpdated {
            object_id: 7,
            mount_type: -1,
            is_mounted: false,
        });
        world
            .run_system_once(apply_object_state_events)
            .expect("对象状态系统应可运行");
        // run_system_once 走 System::run：立即 apply_deferred，命令当帧落地

        assert!(
            world.get::<MountState>(root).is_none(),
            "下马后 MountState 必须移除"
        );
        assert!(
            world.get::<SpriteLayer>(body_layer).is_some(),
            "身体层必须保留"
        );
        assert!(
            world.get::<GhostLayer>(body_ghost).is_some(),
            "身体 ghost 必须保留"
        );
        assert!(
            world.get::<SpriteLayer>(mount_layer).is_none(),
            "坐骑层必须随下马移除"
        );
        assert!(
            world.get::<GhostLayer>(mount_ghost).is_none(),
            "坐骑 ghost 必须随下马移除（P1 泄漏点）"
        );
    }

    /// #2892：`Hidden` 只把**该对象自己**的图层设成 50% 透明。
    ///
    /// C# 基准：`MapObject.cs:5006` `if (Hidden && !DXManager.Blending) DXManager.SetOpacity(0.5F);`
    /// （绘制前设一次全局 opacity，逐对象绘制）。
    /// 阳性对照：把 `apply_hidden_alpha` 改回旧写法（遍历全部 `SpriteLayer` 设 alpha）→
    /// 本测试对 bystander 的断言 FAILED（旁观者也会被设成 0.5）。
    #[test]
    fn hidden_alpha_only_affects_target_object() {
        let mut app = App::new();
        app.add_systems(Update, apply_hidden_alpha);
        let (target, target_layers) = spawn_actor(&mut app, 7, &[1.0, 1.0]);
        let (_bystander, bystander_layers) = spawn_actor(&mut app, 8, &[1.0]);

        app.world_mut()
            .entity_mut(target)
            .insert(HiddenObject { hidden: true });
        app.update();

        for e in &target_layers {
            assert_eq!(
                alpha_of(&app, *e),
                0.5,
                "目标对象应半透明（C# SetOpacity(0.5F)）"
            );
        }
        for e in &bystander_layers {
            assert_eq!(alpha_of(&app, *e), 1.0, "旁观对象不得被一起变半透明");
        }

        // 显形 → 恢复不透明
        app.world_mut()
            .entity_mut(target)
            .insert(HiddenObject { hidden: false });
        app.update();
        for e in &target_layers {
            assert_eq!(alpha_of(&app, *e), 1.0, "取消隐藏后应恢复不透明");
        }
        assert_eq!(hidden_alpha(true), 0.5);
        assert_eq!(hidden_alpha(false), 1.0);
    }
}
