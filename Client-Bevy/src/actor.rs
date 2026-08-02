// ============================================================================
// ActorPlugin - 角色/NPC/怪物精灵渲染与帧动画（里程碑 2）
// ============================================================================
//
// 对应 Client-Macroquad:
// - objects/frames.rs（帧表，原样复用）
// - systems/presentation/animation_system.rs（帧号计算）
// - systems/rendering/sprite_system/character.rs（分层绘制）
//
// 帧号公式（与 C#/macroquad 一致）:
//   DrawFrame     = Frame.Start + Direction * (Count+Skip) + FrameIndex
//   EffectFrame   = Frame.EffectStart + Direction * (EffectCount+EffectSkip) + FrameIndex
//
// 渲染方式: 每个角色实体挂多层子实体（身体/发型/武器/特效），
// 每帧按帧号从对应 .Lib 取图并缓存为 Bevy Image 资产。

use bevy::prelude::*;
use mir2_shared::{MirAction, MirClass, MirDirection, MirGender};

use std::collections::HashMap;

use crate::map_renderer::{make_image, GameData, GameLibraries, TILE_HEIGHT, TILE_WIDTH};
use crate::network::{NetObject, NetObjects};
use crate::objects::frames::{get_default_npc_frame, get_monster_frame, get_player_frame, Frame};
use crate::resources::libraries::ArrayLibType;

pub struct ActorPlugin;

impl Plugin for ActorPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<ActorImageCache>();
        app.add_systems(
            Update,
            (
                spawn_demo_actors_when_ready,
                spawn_net_objects_when_ready,
                despawn_removed_objects,
                advance_actor_animations,
                actor_sprite_render,
                update_local_ghost,
                dump_depth_debug,
            )
                .chain()
                .run_if(in_state(crate::scenes::AppState::Game)),
        );
        app.add_systems(
            Update,
            (demo_drive, sync_actor_depth, log_player_walk)
                .run_if(in_state(crate::scenes::AppState::Game)),
        );
    }
}

// ============================================================================
// 组件
// ============================================================================

/// 角色外观（决定用哪套库）
#[derive(Component, Clone)]
pub enum ActorAppearance {
    Player {
        class: MirClass,
        gender: MirGender,
        armour: u16,
        hair: u8,
        weapon: i16,
        weapon_effect: i16,
        wing_effect: u8,
    },
    Monster {
        monster_type: u16,
        stage: u8,
    },
    Npc {
        npc_index: u16,
    },
}

/// 本地玩家标记（用于遮挡 ghost 效果）
#[derive(Component)]
pub struct LocalPlayer;

/// 服务器对象 ID（ObjectRemove 用它删除实体）
#[derive(Component, Clone, Copy)]
pub struct NetObjectId(pub u32);

/// 动画状态（动作/朝向/当前帧）
#[derive(Component)]
pub struct ActorAnim {
    pub action: MirAction,
    pub direction: u8,
    pub frame_index: i32,
    pub elapsed_ms: f32,
}

impl Default for ActorAnim {
    fn default() -> Self {
        Self {
            action: MirAction::Standing,
            direction: 0,
            frame_index: 0,
            elapsed_ms: 0.0,
        }
    }
}

/// 角色身上的单个渲染层（子实体）
#[derive(Component)]
pub struct SpriteLayer {
    /// 使用哪个数组库
    pub lib: ArrayLibType,
    /// 库槽位（护甲索引/怪物类型/NPC 索引等）
    pub slot: u32,
    /// 当前绘制帧号（由动画系统写入）
    pub frame: i32,
    /// true = 特效层（用 effect 帧段，如翅膀）
    pub is_effect: bool,
}

/// 演示行为（统一枚举，挂在演示角色上）
#[derive(Component)]
pub enum DemoBehavior {
    /// 玩家：绕方块行走（平滑插值，一格 0.6s）
    Walk {
        side_len: i32,
        side_progress: i32,
        direction: u8,
        step_progress: f32,
        from_x: f32,
        from_y: f32,
        to_x: f32,
        to_y: f32,
        started: bool,
    },
    /// 原地待机并缓慢转向
    Idle { timer: f32, interval: f32 },
    /// 周期性攻击
    Attack {
        timer: f32,
        interval: f32,
        attacking: bool,
        attack_timer: f32,
    },
}

// ============================================================================
// 帧表 + 精灵图缓存
// ============================================================================

/// 取角色当前动作对应的帧定义
fn actor_frame(app: &ActorAppearance, anim: &ActorAnim) -> Option<&'static Frame> {
    match app {
        ActorAppearance::Player { .. } => get_player_frame(anim.action),
        ActorAppearance::Monster {
            monster_type,
            stage,
        } => {
            let dir = MirDirection::try_from(anim.direction).unwrap_or(MirDirection::Up);
            get_monster_frame(*monster_type, anim.action, dir, *stage)
        }
        ActorAppearance::Npc { .. } => get_default_npc_frame(anim.action),
    }
}

/// 缓存的精灵图（Bevy Image 句柄 + 绘制元数据）
struct CachedSprite {
    handle: Handle<Image>,
    width: u32,
    height: u32,
    offset_x: i32,
    offset_y: i32,
}

#[derive(Resource, Default)]
pub struct ActorImageCache {
    map: HashMap<(u8, u32, u32), CachedSprite>,
}

// ============================================================================
// 系统
// ============================================================================

/// 动画推进：按帧表 interval 推进 frame_index，并把各层的绘制帧号写回
fn advance_actor_animations(
    time: Res<Time>,
    mut actors: Query<(&ActorAppearance, &mut ActorAnim, &Children)>,
    mut layers: Query<&mut SpriteLayer>,
) {
    let dt_ms = time.delta_secs() * 1000.0;
    for (app, mut anim, children) in &mut actors {
        let Some(frame) = actor_frame(app, &anim) else {
            continue;
        };
        let draw_frame = frame.start + (anim.direction as i32) * frame.offset() + anim.frame_index;
        let effect_frame =
            frame.effect_start + (anim.direction as i32) * frame.effect_offset() + anim.frame_index;

        anim.elapsed_ms += dt_ms;
        let interval = frame.interval.max(1) as f32;
        let count = frame.count.max(1);
        while anim.elapsed_ms >= interval {
            anim.elapsed_ms -= interval;
            anim.frame_index = (anim.frame_index + 1) % count;
        }

        for child in children.iter() {
            if let Ok(mut layer) = layers.get_mut(child) {
                layer.frame = if layer.is_effect {
                    effect_frame
                } else {
                    draw_frame
                };
            }
        }
    }
}

/// 渲染：按 SpriteLayer 帧号取图（带缓存），更新 Sprite 与相对位置
fn actor_sprite_render(
    mut libs: ResMut<GameLibraries>,
    mut images: ResMut<Assets<Image>>,
    mut cache: ResMut<ActorImageCache>,
    mut q: Query<(&mut Sprite, &mut Transform, &SpriteLayer)>,
) {
    for (mut sprite, mut transform, layer) in &mut q {
        let idx = layer.frame.max(0) as u32;
        let key = (layer.lib as u8, layer.slot, idx);

        let cached = match cache.map.get(&key) {
            Some(c) => Some(CachedSprite {
                handle: c.handle.clone(),
                width: c.width,
                height: c.height,
                offset_x: c.offset_x,
                offset_y: c.offset_y,
            }),
            None => match libs
                .0
                .get_array_image(layer.lib, layer.slot as usize, idx as usize)
            {
                Some(info) => {
                    if let Some(rgba) = info.rgba.clone() {
                        let handle = images.add(make_image(
                            rgba,
                            info.width.max(0) as u32,
                            info.height.max(0) as u32,
                        ));
                        let c = CachedSprite {
                            handle,
                            width: info.width.max(0) as u32,
                            height: info.height.max(0) as u32,
                            offset_x: info.offset_x as i32,
                            offset_y: info.offset_y as i32,
                        };
                        cache.map.insert(key, c);
                        Some(CachedSprite {
                            handle: cache.map[&key].handle.clone(),
                            width: cache.map[&key].width,
                            height: cache.map[&key].height,
                            offset_x: cache.map[&key].offset_x,
                            offset_y: cache.map[&key].offset_y,
                        })
                    } else {
                        None
                    }
                }
                None => None,
            },
        };

        match cached {
            Some(c) => {
                sprite.image = c.handle;
                // 相对父实体（演员脚点）的本地坐标：macroquad 里图左上角在
                // (pos.x + offset_x, pos.y + offset_y)，Bevy 以中心为锚且 y 向上
                transform.translation = Vec3::new(
                    c.offset_x as f32 + c.width as f32 / 2.0,
                    -(c.offset_y as f32 + c.height as f32 / 2.0),
                    0.0,
                );
            }
            None => {
                sprite.image = Handle::default();
            }
        }
    }
}

// ============================================================================
// 演示内容
// ============================================================================

/// 等待地图加载后生成网络对象（MapChanged 后服务器发的 ObjectPlayer/Monster/Npc）
fn spawn_net_objects_when_ready(
    mut commands: Commands,
    data: Res<GameData>,
    mut net_objects: ResMut<NetObjects>,
    net: Res<crate::network::NetworkContext>,
) {
    if data.map.is_none() {
        return;
    }
    let pending: Vec<NetObject> = net_objects.pending.drain(..).collect();
    if pending.is_empty() {
        return;
    }
    // mock 模式没有 UserInformation → local_player_id=None，第一个 ObjectPlayer 视为本地
    let mut local_spawned = net.local_player_id.is_some();
    for obj in &pending {
        let is_local = match obj {
            NetObject::Player { object_id, .. } => {
                if net.local_player_id == Some(*object_id) {
                    true
                } else if net.local_player_id.is_none() && !local_spawned {
                    local_spawned = true;
                    true
                } else {
                    false
                }
            }
            _ => false,
        };
        spawn_net_object_entity(&mut commands, obj, is_local);
    }
    tracing::info!("🌐 网络对象生成完成: {} 个", pending.len());
}

/// 按网络对象生成实体；is_local_player 时生成受控本地玩家（无 DemoBehavior）
fn spawn_net_object_entity(commands: &mut Commands, obj: &NetObject, is_local_player: bool) {
    // 瓦片坐标 → 世界像素（脚点）
    let wx = |tx: i32| tx as f32 * TILE_WIDTH + TILE_WIDTH / 2.0;
    let wy = |ty: i32| ty as f32 * TILE_HEIGHT + TILE_HEIGHT;

    match obj {
        NetObject::Player {
            object_id,
            name: _,
            class,
            gender,
            location_x,
            location_y,
            direction: _,
            hair,
            weapon,
            weapon_effect,
            armour,
            wing_effect,
        } => {
            let e = if is_local_player {
                spawn_local_player_with(
                    commands,
                    wx(*location_x),
                    wy(*location_y),
                    *class,
                    *gender,
                    *armour,
                    *hair,
                    *weapon,
                    *weapon_effect,
                    *wing_effect,
                    *object_id,
                )
            } else {
                spawn_remote_player_with(
                    commands,
                    wx(*location_x),
                    wy(*location_y),
                    *class,
                    *gender,
                    *armour,
                    *hair,
                    *weapon,
                    *weapon_effect,
                    *wing_effect,
                    *object_id,
                )
            };
            let _ = e;
        }
        NetObject::Monster {
            object_id,
            name: _,
            location_x,
            location_y,
            image,
            direction,
        } => {
            let e = spawn_monster(
                commands,
                *image,
                wx(*location_x),
                wy(*location_y),
                *direction,
            );
            commands.entity(e).insert(NetObjectId(*object_id));
        }
        NetObject::Npc {
            object_id,
            name: _,
            image,
            location_x,
            location_y,
            direction,
        } => {
            let e = spawn_npc(
                commands,
                *image,
                wx(*location_x),
                wy(*location_y),
                *direction,
            );
            commands.entity(e).insert(NetObjectId(*object_id));
        }
    }
}

/// 处理 ObjectRemove：按 NetObjectId 删除对应实体
fn despawn_removed_objects(
    mut commands: Commands,
    mut net_objects: ResMut<NetObjects>,
    query: Query<(Entity, &NetObjectId)>,
) {
    let to_remove: Vec<u32> = net_objects.to_remove.drain(..).collect();
    if to_remove.is_empty() {
        return;
    }
    for (e, id) in &query {
        if to_remove.contains(&id.0) {
            tracing::debug!("🗑️ 移除对象实体 id={}", id.0);
            commands.entity(e).despawn();
        }
    }
}

/// 等待地图加载完成后生成演示角色（只跑一次）
fn spawn_demo_actors_when_ready(
    mut commands: Commands,
    data: Res<GameData>,
    mut done: Local<bool>,
) {
    // 演示角色只在 --demo 模式下生成（默认走网络 mock 对象）
    if !std::env::args().any(|a| a == "--demo") {
        return;
    }
    if *done {
        return;
    }
    let Some(map) = data.map.as_ref() else {
        return;
    };
    *done = true;

    let cx = (map.width as f32 * TILE_WIDTH) / 2.0;
    let cy = (map.height as f32 * TILE_HEIGHT) / 2.0;

    spawn_player(&mut commands, cx, cy - 4.0 * TILE_HEIGHT);
    spawn_monster(
        &mut commands,
        1,
        cx - 4.0 * TILE_WIDTH,
        cy + 2.0 * TILE_HEIGHT,
        0,
    );
    spawn_monster(
        &mut commands,
        5,
        cx + 4.0 * TILE_WIDTH,
        cy - 3.0 * TILE_HEIGHT,
        0,
    );
    spawn_monster(
        &mut commands,
        9,
        cx - 3.0 * TILE_WIDTH,
        cy + 4.0 * TILE_HEIGHT,
        0,
    );
    spawn_npc(
        &mut commands,
        0,
        cx + 3.0 * TILE_WIDTH,
        cy + 3.0 * TILE_HEIGHT,
        0,
    );
}

/// 世界 y（屏幕向下）→ Bevy 深度 z（与 front 瓦片共用同一深度函数，
/// 实现角色与建筑/树的经典交错遮挡）
pub fn depth_z(world_y: f32) -> f32 {
    crate::map_renderer::depth_y(world_y)
}

fn spawn_player(commands: &mut Commands, x: f32, y: f32) {
    spawn_player_with(
        commands,
        x,
        y,
        MirClass::Warrior,
        MirGender::Male,
        0,
        0,
        0,
        0,
        0,
    );
}

/// 按外观生成本地玩家实体
#[allow(clippy::too_many_arguments)]
fn spawn_player_with(
    commands: &mut Commands,
    x: f32,
    y: f32,
    class: MirClass,
    gender: MirGender,
    armour: i16,
    hair: u8,
    weapon: i16,
    weapon_effect: i16,
    wing_effect: u8,
) -> Entity {
    let z = depth_z(y);
    let root = commands
        .spawn((
            LocalPlayer,
            ActorAppearance::Player {
                class,
                gender,
                armour: armour.max(0) as u16,
                hair,
                weapon,
                weapon_effect,
                wing_effect,
            },
            ActorAnim::default(),
            DemoBehavior::Walk {
                side_len: 6,
                side_progress: 0,
                direction: 0,
                step_progress: 0.0,
                from_x: x,
                from_y: y,
                to_x: x,
                to_y: y,
                started: false,
            },
            Transform::from_xyz(x, -y, z),
            Visibility::default(),
        ))
        .id();
    commands.entity(root).with_children(|p| {
        p.spawn((
            Sprite::default(),
            Transform::default(),
            SpriteLayer {
                lib: ArrayLibType::CArmours,
                slot: armour.max(0) as u32,
                frame: 0,
                is_effect: false,
            },
        ));
        p.spawn((
            Sprite::default(),
            Transform::default(),
            SpriteLayer {
                lib: ArrayLibType::CHair,
                slot: hair as u32,
                frame: 0,
                is_effect: false,
            },
        ));
        p.spawn((
            Sprite::default(),
            Transform::default(),
            SpriteLayer {
                lib: ArrayLibType::CWeapons,
                slot: weapon.max(0) as u32,
                frame: 0,
                is_effect: false,
            },
        ));
        // ghost 残影层（遮挡时显示，镜像对应图层）
        for lib in [
            ArrayLibType::CArmours,
            ArrayLibType::CHair,
            ArrayLibType::CWeapons,
        ] {
            p.spawn((
                Sprite::default(),
                Transform::from_xyz(0.0, 0.0, 0.5),
                Visibility::Hidden,
                GhostLayer { lib },
            ));
        }
    });
    root
}

/// 生成本地受控玩家（真实网络；无 DemoBehavior，由玩家控制系统驱动）
#[allow(clippy::too_many_arguments)]
fn spawn_local_player_with(
    commands: &mut Commands,
    x: f32,
    y: f32,
    class: MirClass,
    gender: MirGender,
    armour: i16,
    hair: u8,
    weapon: i16,
    weapon_effect: i16,
    wing_effect: u8,
    object_id: u32,
) -> Entity {
    let z = depth_z(y);
    let root = commands
        .spawn((
            LocalPlayer,
            NetObjectId(object_id),
            ActorAppearance::Player {
                class,
                gender,
                armour: armour.max(0) as u16,
                hair,
                weapon,
                weapon_effect,
                wing_effect,
            },
            ActorAnim::default(),
            Transform::from_xyz(x, y, z),
            Visibility::default(),
        ))
        .id();
    attach_player_layers(commands, root, armour, hair, weapon);
    root
}

/// 生成远端玩家（其他玩家；无 LocalPlayer、无 DemoBehavior）
#[allow(clippy::too_many_arguments)]
fn spawn_remote_player_with(
    commands: &mut Commands,
    x: f32,
    y: f32,
    class: MirClass,
    gender: MirGender,
    armour: i16,
    hair: u8,
    weapon: i16,
    weapon_effect: i16,
    wing_effect: u8,
    object_id: u32,
) -> Entity {
    let z = depth_z(y);
    let root = commands
        .spawn((
            NetObjectId(object_id),
            ActorAppearance::Player {
                class,
                gender,
                armour: armour.max(0) as u16,
                hair,
                weapon,
                weapon_effect,
                wing_effect,
            },
            ActorAnim::default(),
            Transform::from_xyz(x, y, z),
            Visibility::default(),
        ))
        .id();
    attach_player_layers(commands, root, armour, hair, weapon);
    root
}

/// 玩家分层子精灵（护甲/发型/武器 + ghost 层）
fn attach_player_layers(
    commands: &mut Commands,
    root: Entity,
    armour: i16,
    hair: u8,
    weapon: i16,
) {
    commands.entity(root).with_children(|p| {
        p.spawn((
            Sprite::default(),
            Transform::default(),
            SpriteLayer {
                lib: ArrayLibType::CArmours,
                slot: armour.max(0) as u32,
                frame: 0,
                is_effect: false,
            },
        ));
        p.spawn((
            Sprite::default(),
            Transform::default(),
            SpriteLayer {
                lib: ArrayLibType::CHair,
                slot: hair as u32,
                frame: 0,
                is_effect: false,
            },
        ));
        p.spawn((
            Sprite::default(),
            Transform::default(),
            SpriteLayer {
                lib: ArrayLibType::CWeapons,
                slot: weapon.max(0) as u32,
                frame: 0,
                is_effect: false,
            },
        ));
        for lib in [
            ArrayLibType::CArmours,
            ArrayLibType::CHair,
            ArrayLibType::CWeapons,
        ] {
            p.spawn((
                Sprite::default(),
                Transform::from_xyz(0.0, 0.0, 0.5),
                Visibility::Hidden,
                GhostLayer { lib },
            ));
        }
    });
}

fn spawn_monster(commands: &mut Commands, monster_type: u16, x: f32, y: f32, direction: u8) -> Entity {
    let z = depth_z(y);
    let root = commands
        .spawn((
            ActorAppearance::Monster {
                monster_type,
                stage: 0,
            },
            ActorAnim {
                action: MirAction::Standing,
                direction,
                frame_index: 0,
                elapsed_ms: 0.0,
            },
            if monster_type.is_multiple_of(3) {
                DemoBehavior::Attack {
                    timer: 0.0,
                    interval: 4.0,
                    attacking: false,
                    attack_timer: 0.0,
                }
            } else {
                DemoBehavior::Idle {
                    timer: 0.0,
                    interval: 1.5,
                }
            },
            Transform::from_xyz(x, -y, z),
            Visibility::default(),
        ))
        .id();
    commands.entity(root).with_children(|p| {
        p.spawn((
            Sprite::default(),
            Transform::default(),
            SpriteLayer {
                lib: ArrayLibType::Monsters,
                slot: monster_type as u32,
                frame: 0,
                is_effect: false,
            },
        ));
    });
    root
}

fn spawn_npc(commands: &mut Commands, npc_index: u16, x: f32, y: f32, direction: u8) -> Entity {
    let z = depth_z(y);
    let root = commands
        .spawn((
            ActorAppearance::Npc { npc_index },
            ActorAnim {
                action: MirAction::Standing,
                direction,
                frame_index: 0,
                elapsed_ms: 0.0,
            },
            DemoBehavior::Idle {
                timer: 0.0,
                interval: 3.0,
            },
            Transform::from_xyz(x, -y, z),
            Visibility::default(),
        ))
        .id();
    commands.entity(root).with_children(|p| {
        p.spawn((
            Sprite::default(),
            Transform::default(),
            SpriteLayer {
                lib: ArrayLibType::Npcs,
                slot: npc_index as u32,
                frame: 0,
                is_effect: false,
            },
        ));
    });
    root
}

/// 调试：输出角色 z、玩家平滑移动进度与 ghost 遮挡瓦片数（帧 30 一次）
fn dump_depth_debug(
    actors: Query<(&ActorAppearance, &Transform)>,
    front: Query<(&Transform, &crate::map_renderer::FrontTile)>,
    ghosts: Query<&Visibility, With<GhostLayer>>,
    local: Query<&Transform, (With<LocalPlayer>, Without<GhostLayer>)>,
    q_layer: Query<&SpriteLayer>,
    mut frames: Local<u32>,
) {
    if std::env::var_os("CRYSTAL_DEPTH_DEBUG").is_none() {
        return;
    }
    *frames += 1;
    if *frames != 30 {
        return;
    }
    let mut lines = String::from("depth debug\n");
    for (app, tf) in &actors {
        let label = match app {
            ActorAppearance::Player { .. } => "player",
            ActorAppearance::Monster { .. } => "monster",
            ActorAppearance::Npc { .. } => "npc",
        };
        lines.push_str(&format!(
            "  actor {} at world_y={:.0} z={:.4}\n",
            label, -tf.translation.y, tf.translation.z
        ));
    }
    // 角色附近（中心 1280x800 视野内）的 front 瓦片 z
    let mut fz: Vec<(f32, f32)> = front
        .iter()
        .filter(|(_, ft)| ft.base_y > 10800.0 && ft.base_y < 11600.0)
        .map(|(tf, _)| (tf.translation.z, tf.translation.x))
        .collect();
    fz.sort_by(|a, b| a.0.partial_cmp(&b.0).unwrap());
    lines.push_str(&format!(
        "  front tiles in view(base_y 10800..11600): {}  z range {:.4}..{:.4}\n",
        fz.len(),
        fz.first().map(|x| x.0).unwrap_or(0.0),
        fz.last().map(|x| x.0).unwrap_or(0.0)
    ));
    // 角色纹理诊断：统计角色实体上 Sprite 图像是否有效
    {
        let layers = q_layer.iter().count();
        lines.push_str(&format!("  sprite_layers={}\n", layers));
    }

    // ghost 统计：玩家前方遮挡瓦片数（本地玩家）
    let player_y = actors
        .iter()
        .filter(|(app, _)| matches!(app, ActorAppearance::Player { .. }))
        .map(|(_, tf)| -tf.translation.y)
        .next();
    if let Some(foot_y) = player_y {
        let occluding = front
            .iter()
            .filter(|(tf, ft)| {
                ft.bottom > foot_y
                    && (tf.translation.x - 16600.0).abs() < 300.0
                    && ft.base_y < foot_y + 500.0
            })
            .count();
        lines.push_str(&format!(
            "  player foot_y={:.0} occluding_front_tiles={}\n",
            foot_y, occluding
        ));
    }
    // ghost 状态：本地玩家是否被遮挡、残影是否可见
    if let Ok(tf) = local.single() {
        let foot_x = tf.translation.x;
        let foot_y = -tf.translation.y;
        let occluded = front.iter().any(|(_, ft)| {
            ft.bottom > foot_y
                && ft.left < foot_x + 22.0
                && ft.right > foot_x - 22.0
                && ft.top < foot_y + 2.0
                && ft.bottom > foot_y - 92.0
        });
        let visible_count = ghosts.iter().filter(|v| **v == Visibility::Visible).count();
        lines.push_str(&format!(
            "  ghost: occluded={} visible_layers={}/{}\n",
            occluded,
            visible_count,
            ghosts.iter().count()
        ));
    }
    let _ = std::fs::write("E:/tmp/depth_debug.txt", lines);
}

/// Ghost 残影层标记：镜像本地玩家的对应图层（按库匹配）
#[derive(Component, Clone, Copy)]
pub struct GhostLayer {
    pub lib: crate::resources::libraries::ArrayLibType,
}

/// 本地玩家遮挡 ghost（对齐 macroquad PostFront 实现）：
/// 被 front 瓦片（建筑/树）遮挡时，在 front 层之上再画一层半透明玩家残影。
/// 建筑本身保持不透明，避免"周边建筑块全变半透明"。
#[allow(clippy::type_complexity)]
fn update_local_ghost(
    mut ghosts: Query<
        (&mut Sprite, &mut Transform, &mut Visibility, &GhostLayer),
        (Without<SpriteLayer>, Without<LocalPlayer>),
    >,
    local: Query<(&Transform, &Children), (With<LocalPlayer>, Without<GhostLayer>)>,
    layers: Query<(&Sprite, &Transform, &SpriteLayer), Without<GhostLayer>>,
    front: Query<&crate::map_renderer::FrontTile>,
) {
    let Ok((root_tf, children)) = local.single() else {
        return;
    };
    let foot_x = root_tf.translation.x;
    let foot_y = -root_tf.translation.y;
    // 玩家身体包围盒（覆盖身体/武器/翅膀）
    let (bl, bt, br, bb) = (foot_x - 22.0, foot_y - 92.0, foot_x + 22.0, foot_y + 2.0);
    let occluded = front.iter().any(|ft| {
        ft.bottom > foot_y && ft.left < br && ft.right > bl && ft.top < bb && ft.bottom > bt
    });
    const GHOST_ALPHA: f32 = 0.55; // 与 macroquad PLAYER_GHOST_ALPHA 一致
    const GHOST_LOCAL_Z: f32 = 0.5; // 本地 z 偏移：保证世界 z 高于所有 front 瓦片

    for (mut gs, mut gt, mut gv, gl) in &mut ghosts {
        let mut matched = None;
        for child in children.iter() {
            if let Ok((ls, lt, ll)) = layers.get(child) {
                if ll.lib == gl.lib {
                    matched = Some((ls, lt));
                    break;
                }
            }
        }
        match matched {
            Some((ls, lt)) if occluded => {
                gs.image = ls.image.clone();
                gs.color = Color::srgba(1.0, 1.0, 1.0, GHOST_ALPHA);
                gt.translation = Vec3::new(lt.translation.x, lt.translation.y, GHOST_LOCAL_Z);
                *gv = Visibility::Visible;
            }
            _ => {
                *gv = Visibility::Hidden;
            }
        }
    }
}

/// 调试：记录玩家位置采样，验证移动平滑（每 6 帧一次，前 90 帧，CRYSTAL_DEPTH_DEBUG 开启）
fn log_player_walk(local: Query<&Transform, With<LocalPlayer>>, mut frames: Local<u32>) {
    if std::env::var_os("CRYSTAL_DEPTH_DEBUG").is_none() {
        return;
    }
    *frames += 1;
    if *frames > 90 || !(*frames).is_multiple_of(6) {
        return;
    }
    if let Ok(tf) = local.single() {
        use std::io::Write;
        if let Ok(mut f) = std::fs::OpenOptions::new()
            .create(true)
            .append(true)
            .open("E:/tmp/player_walk.txt")
        {
            let _ = writeln!(
                f,
                "frame={} x={:.1} y={:.1}",
                *frames, tf.translation.x, tf.translation.y
            );
        }
    }
}

/// 角色 z 与脚底世界 Y 保持同步（移动/转向时深度正确）
fn sync_actor_depth(mut actors: Query<&mut Transform, With<ActorAppearance>>) {
    for mut tf in &mut actors {
        // translation.y = -世界Y（Bevy y 向上）
        tf.translation.z = depth_z(-tf.translation.y);
    }
}

/// 演示驱动：玩家绕方块行走；怪物/NPC 原地转向；部分怪物周期性攻击
fn demo_drive(
    time: Res<Time>,
    mut actors: Query<(&mut ActorAnim, &mut Transform, &mut DemoBehavior)>,
) {
    let dt = time.delta_secs();

    for (mut anim, mut tf, mut behavior) in &mut actors {
        match behavior.as_mut() {
            DemoBehavior::Walk {
                side_len,
                side_progress,
                direction,
                step_progress,
                from_x,
                from_y,
                to_x,
                to_y,
                started,
            } => {
                anim.action = MirAction::Walking;
                anim.direction = *direction;
                let step_time = 0.6; // 与 Walking 帧间隔同步（6帧 * 100ms）
                if !*started {
                    // 首次：从当前位置初始化目标
                    *from_x = tf.translation.x;
                    *from_y = -tf.translation.y;
                    let (dx, dy) = dir_vec(*direction);
                    *to_x = *from_x + dx * TILE_WIDTH;
                    *to_y = *from_y + dy * TILE_HEIGHT;
                    *started = true;
                    *step_progress = 0.0;
                }
                *step_progress += dt / step_time;
                // 低帧率时可能一次跨多步：逐格完成
                while *step_progress >= 1.0 {
                    *step_progress -= 1.0;
                    *from_x = *to_x;
                    *from_y = *to_y;
                    *side_progress += 1;
                    if *side_progress >= *side_len {
                        *side_progress = 0;
                        *direction = (*direction + 2) % 8;
                        anim.direction = *direction;
                    }
                    let (dx, dy) = dir_vec(*direction);
                    *to_x = *from_x + dx * TILE_WIDTH;
                    *to_y = *from_y + dy * TILE_HEIGHT;
                }
                // 平滑插值：从当前格到目标格
                let t = (*step_progress).clamp(0.0, 1.0);
                tf.translation.x = *from_x + (*to_x - *from_x) * t;
                tf.translation.y = -(*from_y + (*to_y - *from_y) * t);
            }
            DemoBehavior::Idle { timer, interval } => {
                *timer += dt;
                if *timer >= *interval {
                    *timer = 0.0;
                    anim.direction = (anim.direction + 1) % 8;
                }
                anim.action = MirAction::Standing;
            }
            DemoBehavior::Attack {
                timer,
                interval,
                attacking,
                attack_timer,
            } => {
                *timer += dt;
                if *attacking {
                    *attack_timer += dt;
                    anim.action = MirAction::Attack1;
                    if *attack_timer >= 0.7 {
                        *attacking = false;
                        *attack_timer = 0.0;
                        anim.frame_index = 0;
                        anim.elapsed_ms = 0.0;
                    }
                } else {
                    anim.action = MirAction::Standing;
                    if *timer >= *interval {
                        *timer = 0.0;
                        *attacking = true;
                        anim.frame_index = 0;
                        anim.elapsed_ms = 0.0;
                    }
                }
            }
        }
    }
}

/// MirDirection: 0=Up 1=UpRight 2=Right 3=DownRight 4=Down 5=DownLeft 6=Left 7=UpLeft
fn dir_vec(d: u8) -> (f32, f32) {
    match d % 8 {
        0 => (0.0, -1.0),
        1 => (1.0, -1.0),
        2 => (1.0, 0.0),
        3 => (1.0, 1.0),
        4 => (0.0, 1.0),
        5 => (-1.0, 1.0),
        6 => (-1.0, 0.0),
        _ => (-1.0, -1.0),
    }
}
