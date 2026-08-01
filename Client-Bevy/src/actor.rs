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
use crate::objects::frames::{
    get_default_npc_frame, get_monster_frame, get_player_frame, Frame,
};
use crate::resources::libraries::ArrayLibType;

pub struct ActorPlugin;

impl Plugin for ActorPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<ActorImageCache>();
        app.add_systems(
            Update,
            (spawn_demo_actors_when_ready, advance_actor_animations, actor_sprite_render).chain(),
        );
        app.add_systems(Update, demo_drive);
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
    /// 玩家：绕方块行走
    Walk {
        side_len: i32,
        side_progress: i32,
        direction: u8,
        move_timer: f32,
    },
    /// 原地待机并缓慢转向
    Idle {
        timer: f32,
        interval: f32,
    },
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
        let draw_frame =
            frame.start + (anim.direction as i32) * frame.offset() + anim.frame_index;
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

/// 等待地图加载完成后生成演示角色（只跑一次）
fn spawn_demo_actors_when_ready(
    mut commands: Commands,
    data: Res<GameData>,
    mut done: Local<bool>,
) {
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
    spawn_monster(&mut commands, 1, cx - 4.0 * TILE_WIDTH, cy + 2.0 * TILE_HEIGHT);
    spawn_monster(&mut commands, 5, cx + 4.0 * TILE_WIDTH, cy - 3.0 * TILE_HEIGHT);
    spawn_monster(&mut commands, 9, cx - 3.0 * TILE_WIDTH, cy + 4.0 * TILE_HEIGHT);
    spawn_npc(&mut commands, 0, cx + 3.0 * TILE_WIDTH, cy + 3.0 * TILE_HEIGHT);
}

/// 世界 y（屏幕向下）→ Bevy 深度 z：越靠下（y 越大）越靠前
fn depth_z(world_y: f32) -> f32 {
    0.5 + world_y * 0.00001
}

fn spawn_player(commands: &mut Commands, x: f32, y: f32) {
    let z = depth_z(y);
    let root = commands
        .spawn((
            ActorAppearance::Player {
                class: MirClass::Warrior,
                gender: MirGender::Male,
                armour: 0,
                hair: 0,
                weapon: 0,
                weapon_effect: 0,
                wing_effect: 0,
            },
            ActorAnim::default(),
            DemoBehavior::Walk {
                side_len: 6,
                side_progress: 0,
                direction: 0,
                move_timer: 0.0,
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
                slot: 0,
                frame: 0,
                is_effect: false,
            },
        ));
        p.spawn((
            Sprite::default(),
            Transform::default(),
            SpriteLayer {
                lib: ArrayLibType::CHair,
                slot: 0,
                frame: 0,
                is_effect: false,
            },
        ));
        p.spawn((
            Sprite::default(),
            Transform::default(),
            SpriteLayer {
                lib: ArrayLibType::CWeapons,
                slot: 0,
                frame: 0,
                is_effect: false,
            },
        ));
    });
}

fn spawn_monster(commands: &mut Commands, monster_type: u16, x: f32, y: f32) {
    let z = depth_z(y);
    let root = commands
        .spawn((
            ActorAppearance::Monster {
                monster_type,
                stage: 0,
            },
            ActorAnim::default(),
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
}

fn spawn_npc(commands: &mut Commands, npc_index: u16, x: f32, y: f32) {
    let z = depth_z(y);
    let root = commands
        .spawn((
            ActorAppearance::Npc { npc_index },
            ActorAnim::default(),
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
                move_timer,
            } => {
                let step_time = 0.6; // 与 Walking 帧间隔同步（6帧 * 100ms）
                *move_timer += dt;
                while *move_timer >= step_time {
                    *move_timer -= step_time;
                    let (dx, dy) = dir_vec(*direction);
                    tf.translation.x += dx * TILE_WIDTH;
                    tf.translation.y -= dy * TILE_HEIGHT;
                    *side_progress += 1;
                    if *side_progress >= *side_len {
                        *side_progress = 0;
                        *direction = (*direction + 2) % 8;
                    }
                }
                anim.action = MirAction::Walking;
                anim.direction = *direction;
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
