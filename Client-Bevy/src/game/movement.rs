// ============================================================================
// 移动（M8）
// 本地玩家：沿寻路路径按 100ms/格 步进（原版 GameScene.MoveTime=100），
//   每步发 Walk/Run 包（参考 Client-Macroquad player_control_system：Run=2 格）。
// 远端对象：ObjectWalk/Run/Turn 插值移动（参考 macroquad position_interpolation）。
// ============================================================================

use std::collections::VecDeque;

use bevy::prelude::*;
use mir2_shared::enums::MirDirection;

use crate::actor::{depth_z, ActorAnim, LocalPlayer, NetObjectId};
use crate::map_renderer::{TILE_HEIGHT, TILE_WIDTH};
use crate::network::NetworkContext;
use crate::scenes::AppState;

/// 服务器对象移动事件（网络 handler 写入）
#[derive(Resource, Default)]
pub struct NetMotions {
    pub pending: Vec<NetMotion>,
}

#[derive(Debug, Clone)]
pub enum NetMotion {
    Walk { object_id: u32, x: i32, y: i32, dir: u8 },
    Run { object_id: u32, x: i32, y: i32, dir: u8 },
    Turn { object_id: u32, x: i32, y: i32, dir: u8 },
}

impl NetMotion {
    pub fn object_id(&self) -> u32 {
        match self {
            NetMotion::Walk { object_id, .. }
            | NetMotion::Run { object_id, .. }
            | NetMotion::Turn { object_id, .. } => *object_id,
        }
    }
}

/// 本地玩家移动目标（寻路路径，瓦片坐标）
#[derive(Component)]
pub struct LocalMove {
    pub path: VecDeque<(i32, i32)>,
    /// 步进计时（毫秒累计）
    pub step_timer_ms: f32,
    /// 是否跑步（中键 AutoRun / 双击）
    pub run: bool,
}

/// 一段插值移动（本地与远端通用）
#[derive(Component)]
pub struct MoveTween {
    pub from: Vec2,
    pub to: Vec2,
    pub t: f32,
    pub dur: f32,
    pub action: mir2_shared::enums::MirAction,
    pub dir: u8,
}

/// 瓦片坐标 → 世界像素（脚点）
pub fn tile_to_world(tx: i32, ty: i32) -> Vec2 {
    Vec2::new(
        tx as f32 * TILE_WIDTH + TILE_WIDTH / 2.0,
        -(ty as f32 * TILE_HEIGHT + TILE_HEIGHT),
    )
}

/// 世界像素 → 瓦片坐标
pub fn world_to_tile(wx: f32, wy: f32) -> (i32, i32) {
    (
        ((wx - TILE_WIDTH / 2.0) / TILE_WIDTH).round() as i32,
        ((-wy - TILE_HEIGHT) / TILE_HEIGHT).round() as i32,
    )
}

/// 计算朝向（dx/dy ∈ {-1,0,1}）
pub fn direction_from_delta(dx: i32, dy: i32) -> Option<MirDirection> {
    Some(match (dx, dy) {
        (0, -1) => MirDirection::Up,
        (1, -1) => MirDirection::UpRight,
        (1, 0) => MirDirection::Right,
        (1, 1) => MirDirection::DownRight,
        (0, 1) => MirDirection::Down,
        (-1, 1) => MirDirection::DownLeft,
        (-1, 0) => MirDirection::Left,
        (-1, -1) => MirDirection::UpLeft,
        _ => return None,
    })
}

pub struct MovementPlugin;

impl Plugin for MovementPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(
            Update,
            (
                apply_net_motions,
                advance_move_tweens,
                advance_local_move,
                apply_self_position,
            )
                .chain()
                .run_if(in_state(AppState::Game)),
        );
    }
}

/// 服务器权威位置（UserLocation）：距离超过 2 格时瞬移校正
fn apply_self_position(
    mut net: ResMut<NetworkContext>,
    mut players: Query<&mut Transform, (With<LocalPlayer>, Without<NetObjectId>)>,
) {
    let Some((tx, ty, _dir)) = net.self_position.take() else {
        return;
    };
    let Ok(mut tf) = players.single_mut() else {
        return;
    };
    let cur = world_to_tile(tf.translation.x, tf.translation.y);
    let dist = ((tx - cur.0).abs() + (ty - cur.1).abs()).max(
        ((tx - cur.0).abs()).max((ty - cur.1).abs()),
    );
    if dist > 2 {
        let p = tile_to_world(tx, ty);
        tf.translation.x = p.x;
        tf.translation.y = p.y;
        tf.translation.z = depth_z(p.y);
        tracing::info!("📍 服务器位置校正 -> ({},{})", tx, ty);
    }
}

/// 消耗 NetMotions：给对象实体挂 MoveTween / 转向
fn apply_net_motions(
    mut commands: Commands,
    mut motions: ResMut<NetMotions>,
    mut actors: Query<(Entity, &NetObjectId, &mut ActorAnim, &Transform, Option<&LocalPlayer>)>,
) {
    let pending: Vec<NetMotion> = motions.pending.drain(..).collect();
    for motion in pending {
        for (e, id, mut anim, tf, local) in &mut actors {
            if id.0 != motion.object_id() {
                continue;
            }
            // 本地玩家移动由客户端驱动，跳过服务器回显
            if local.is_some() {
                continue;
            }
            let from = Vec2::new(tf.translation.x, tf.translation.y);
            match motion {
                NetMotion::Turn { dir, .. } => {
                    anim.direction = dir;
                    anim.action = mir2_shared::enums::MirAction::Standing;
                    anim.frame_index = 0;
                }
                NetMotion::Walk { x, y, dir, .. } => {
                    commands.entity(e).insert(MoveTween {
                        from,
                        to: tile_to_world(x, y),
                        t: 0.0,
                        dur: 0.16,
                        action: mir2_shared::enums::MirAction::Walking,
                        dir,
                    });
                    anim.action = mir2_shared::enums::MirAction::Walking;
                    anim.direction = dir;
                    anim.frame_index = 0;
                }
                NetMotion::Run { x, y, dir, .. } => {
                    commands.entity(e).insert(MoveTween {
                        from,
                        to: tile_to_world(x, y),
                        t: 0.0,
                        dur: 0.20,
                        action: mir2_shared::enums::MirAction::Running,
                        dir,
                    });
                    anim.action = mir2_shared::enums::MirAction::Running;
                    anim.direction = dir;
                    anim.frame_index = 0;
                }
            }
        }
    }
}

/// 推进插值移动
fn advance_move_tweens(
    mut commands: Commands,
    time: Res<Time>,
    mut actors: Query<(Entity, &mut MoveTween, &mut Transform, &mut ActorAnim)>,
) {
    for (e, mut tween, mut tf, mut anim) in &mut actors {
        tween.t += time.delta_secs();
        let k = (tween.t / tween.dur).clamp(0.0, 1.0);
        let pos = tween.from.lerp(tween.to, k);
        tf.translation.x = pos.x;
        tf.translation.y = pos.y;
        // z 深度排序跟随脚底 Y
        tf.translation.z = depth_z(pos.y);
        if tween.t >= tween.dur {
            commands.entity(e).remove::<MoveTween>();
            anim.action = mir2_shared::enums::MirAction::Standing;
            anim.frame_index = 0;
        }
    }
}

/// 本地玩家沿路径步进：100ms/格（走），Run 一次 2 格
// 注：编译器对 Mut<T> 元组解构的 unused_mut 判定与 E0596 自相矛盾，显式允许
#[allow(unused_mut)]
fn advance_local_move(
    mut commands: Commands,
    time: Res<Time>,
    net: Res<NetworkContext>,
    mut players: Query<(Entity, &mut LocalMove, &mut Transform, &mut ActorAnim), With<LocalPlayer>>,
) {
    let Ok((e, mut lm, mut tf, mut anim)) = players.single_mut() else {
        return;
    };
    if lm.path.is_empty() {
        return;
    }
    lm.step_timer_ms += time.delta_secs() * 1000.0;
    if lm.step_timer_ms < 100.0 {
        return;
    }
    lm.step_timer_ms = 0.0;

    // 当前瓦片
    let cur = world_to_tile(tf.translation.x, tf.translation.y);
    let Some(next) = lm.path.pop_front() else {
        return;
    };

    // Run：连走 2 格（若路径第二格存在且可走）
    let (target, send_run) = if lm.run {
        if let Some(&third) = lm.path.front() {
            if let Some(dir2) = direction_from_delta(
                (third.0 - cur.0).clamp(-1, 1),
                (third.1 - cur.1).clamp(-1, 1),
            ) {
                let _ = dir2;
                // 第二格与第三格方向一致才连跑
                let d1 = direction_from_delta(next.0 - cur.0, next.1 - cur.1);
                let d2 = direction_from_delta(third.0 - next.0, third.1 - next.1);
                if d1 == d2 {
                    lm.path.pop_front();
                    (third, true)
                } else {
                    (next, false)
                }
            } else {
                (next, false)
            }
        } else {
            (next, false)
        }
    } else {
        (next, false)
    };

    let dir = match direction_from_delta(target.0 - cur.0, target.1 - cur.1) {
        Some(d) => d,
        None => {
            // 目标不可达/同格：跳过
            return;
        }
    };

    // 发送移动包（服务器步进语义）
    if send_run {
        net.send_packet(&mir2_shared::packets::client::movement::Run { direction: dir });
    } else {
        net.send_packet(&mir2_shared::packets::client::movement::Walk { direction: dir });
    }

    // 本地插值移动
    let from = Vec2::new(tf.translation.x, tf.translation.y);
    let to = tile_to_world(target.0, target.1);
    commands.entity(e).insert(MoveTween {
        from,
        to,
        t: 0.0,
        dur: if send_run { 0.20 } else { 0.16 },
        action: if send_run {
            mir2_shared::enums::MirAction::Running
        } else {
            mir2_shared::enums::MirAction::Walking
        },
        dir: dir as u8,
    });
    anim.action = if send_run {
        mir2_shared::enums::MirAction::Running
    } else {
        mir2_shared::enums::MirAction::Walking
    };
    anim.direction = dir as u8;
    anim.frame_index = 0;
}
