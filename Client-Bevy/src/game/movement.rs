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
use crate::network::{NetConnection, SessionState};
use crate::scenes::AppState;

/// 服务器对象移动事件（网络 handler 发送，移动系统消费）
#[derive(Message, Debug, Clone)]
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
    /// 当前路径段离开的节点（到达节点时更新；用于稳定方向，避免滑行中方向抖动）
    pub last: Option<(i32, i32)>,
    /// 路径起点（首帧固定；防止滑行中 cur 越过瓦片边界导致首格误判已到达而不发包，#77）
    pub step_origin: Option<(i32, i32)>,
    /// 转向计时器（固定角速度：每 125ms 转 1 个方向）
    pub turn_acc: f32,
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

/// 向前看最多 2 个路径节点，返回整体前进方向（若 2 格共线则用 2 格方向，
/// 否则用第 1 格方向）——减少短锯齿路径引起的方向乱跳
fn lookahead_direction(last: (i32, i32), path: &VecDeque<(i32, i32)>) -> Option<mir2_shared::enums::MirDirection> {
    let p0 = *path.front()?;
    let d0 = (p0.0 - last.0, p0.1 - last.1);
    if path.len() >= 2 {
        let p1 = path.iter().nth(1).copied().unwrap_or(p0);
        let d1 = (p1.0 - p0.0, p1.1 - p0.1);
        if d1 == d0 {
            return direction_from_delta(d0.0 * 2, d0.1 * 2);
        }
    }
    direction_from_delta(d0.0, d0.1)
}

/// 逐步转向（对齐 macroquad MovementSystem::step_towards_direction）：
/// 每帧最多转 max_steps 步，选择最短旋转方向（顺时针/逆时针）
fn step_towards_direction(current: u8, desired: u8, max_steps: i32) -> u8 {
    let cur = current % 8;
    let des = desired % 8;
    let diff = (des as i32 - cur as i32).rem_euclid(8);
    if diff == 0 {
        return current;
    }
    let cw = diff;
    let ccw = 8 - diff;
    let steps = max_steps.clamp(1, 3);
    if cw <= ccw {
        ((cur as i32 + cw.min(steps)) % 8) as u8
    } else {
        ((cur as i32 - ccw.min(steps)).rem_euclid(8)) as u8
    }
}

pub struct MovementPlugin;

impl Plugin for MovementPlugin {
    fn build(&self, app: &mut App) {
        app.add_message::<NetMotion>();
        app.add_systems(
            Update,
            (
                apply_net_motions,
                advance_move_tweens,
                advance_local_move,
                apply_self_position,
            )
                .chain()
                .after(crate::network::network_system)
                .run_if(in_state(AppState::Game)),
        );
    }
}

/// 服务器权威位置（UserLocation）：距离超过 2 格时瞬移校正
fn apply_self_position(
    mut session: ResMut<SessionState>,
    // 本地玩家同时带 NetObjectId（此前误用 Without<NetObjectId> 把玩家自己排除，
    // 服务器 UserLocation 校正永不生效 → 客户端位置漂移（#57 实测）
    mut players: Query<&mut Transform, (With<LocalPlayer>, With<NetObjectId>)>,
    local_moves: Query<(), (With<LocalPlayer>, With<LocalMove>)>,
) {
    // 本地移动中：服务器位置必然滞后于客户端（网络往返），瞬移校正会与 LocalMove 每帧拉扯，
    // 导致路径永远走不完（#77 实测：客户端 10 格路径只发 1 个 Run，服务器坐标停在出生点附近）。
    // 不消费该校正值，等移动结束后下一帧再应用。
    if !local_moves.is_empty() {
        return;
    }
    let Some((tx, ty, _dir)) = session.self_position.take() else {
        return;
    };
    let Ok(mut tf) = players.single_mut() else {
        tracing::debug!("📍 位置校正：玩家 Query 未匹配（self_position 丢弃 ({},{})）", tx, ty);
        return;
    };
    let cur = world_to_tile(tf.translation.x, tf.translation.y);
    tracing::debug!("📍 位置校正检查：server=({},{}) cur=({},{})", tx, ty, cur.0, cur.1);
    let dist = ((tx - cur.0).abs() + (ty - cur.1).abs()).max(
        ((tx - cur.0).abs()).max((ty - cur.1).abs()),
    );
    if dist > 2 {
        let p = tile_to_world(tx, ty);
        tf.translation.x = p.x;
        tf.translation.y = p.y;
        tf.translation.z = depth_z(-p.y);
        tracing::info!("📍 服务器位置校正 -> ({},{})", tx, ty);
    }
}

/// 消耗 NetMotions：给对象实体挂 MoveTween / 转向
fn apply_net_motions(
    mut commands: Commands,
    mut motions: MessageReader<NetMotion>,
    mut actors: Query<(Entity, &NetObjectId, &mut ActorAnim, &Transform, Option<&LocalPlayer>)>,
) {
    let pending: Vec<NetMotion> = motions.read().cloned().collect();
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
    mut actors: Query<
        (Entity, &mut MoveTween, &mut Transform, &mut ActorAnim, Option<&LocalMove>),
    >,
) {
    for (e, mut tween, mut tf, mut anim, lm) in &mut actors {
        tween.t += time.delta_secs();
        let k = (tween.t / tween.dur).clamp(0.0, 1.0);
        let pos = tween.from.lerp(tween.to, k);
        tf.translation.x = pos.x;
        tf.translation.y = pos.y;
        // z 深度排序跟随脚底 Y
        tf.translation.z = depth_z(-pos.y);
        if tween.t >= tween.dur {
            commands.entity(e).remove::<MoveTween>();
            // 路径还有下一步 → 保持走路/跑步动画（否则每格复位成站立 = 像瞬移/机器人）
            let still_moving = lm.map(|lm| !lm.path.is_empty()).unwrap_or(false);
            if !still_moving {
                anim.action = mir2_shared::enums::MirAction::Standing;
                anim.frame_index = 0;
            }
        }
    }
}

/// 本地玩家沿路径连续速度移动（对齐 macroquad MovementSystem）：
/// - 走 100px/s、跑 150px/s（1.5 倍），每帧平滑位移 → 丝滑
/// - 走到路径节点附近(5px)后对齐并推进下一个节点；跨格时发 Walk/Run 包
#[allow(unused_mut)]
fn advance_local_move(
    mut commands: Commands,
    time: Res<Time>,
    net: Res<NetConnection>,
    mut players: Query<(Entity, &mut LocalMove, &mut Transform, &mut ActorAnim), With<LocalPlayer>>,
) {
    // 与动画帧率同步（C#：走 1 格/6 帧/100ms，跑 2 格/6 帧/100ms）
    // walk = 48/0.6 = 80px/s，run = 96/0.6 = 160px/s → 脚部与地面严格同步
    const WALK_SPEED: f32 = 80.0;
    const RUN_SPEED: f32 = 160.0;
    const ARRIVAL: f32 = 5.0;

    let Ok((e, mut lm, mut tf, mut anim)) = players.single_mut() else {
        return;
    };
    let dt = time.delta_secs();
    if lm.path.is_empty() {
        // 路径结束：恢复站立
        if anim.action != mir2_shared::enums::MirAction::Standing {
            anim.action = mir2_shared::enums::MirAction::Standing;
            anim.frame_index = 0;
        }
        return;
    }

    // 步进决策（#77 修复）：只有 2 格同向直线才跨 2 格发 Run（服务器 Run=2 格/次），
    // 转弯/斜线段逐格发 Walk——此前 run_step2 对斜线段整段不发包（direction_from_delta 返回
    // None），服务器坐标滞后于客户端，近战永远打不到目标（实测 10 格路径只发 1 个 Run）。
    let cur = world_to_tile(tf.translation.x, tf.translation.y);
    // 首帧固定路径起点（此后 cur 随滑动漂移，不能作为段起点——否则首格会被误判已到达）
    if lm.last.is_none() && lm.step_origin.is_none() {
        lm.step_origin = Some(cur);
    }
    let from = lm.last.or(lm.step_origin).unwrap_or(cur);
    let first = *lm.path.front().unwrap();
    let d1 = (first.0 - from.0, first.1 - from.1);
    let mut use_run = lm.run && lm.path.len() >= 2 && d1.0.abs() + d1.1.abs() == 1;
    if use_run {
        let second = *lm.path.get(1).unwrap();
        let d2 = (second.0 - first.0, second.1 - first.1);
        use_run = d1 == d2; // 仅同向直线
    }
    let target = if use_run { *lm.path.get(1).unwrap() } else { first };
    tracing::debug!("🚶 move: use_run={} path_len={} target=({},{})", use_run, lm.path.len(), target.0, target.1);
    let target_world = tile_to_world(target.0, target.1);
    let dx = target_world.x - tf.translation.x;
    let dy = target_world.y - tf.translation.y;
    let dist = (dx * dx + dy * dy).sqrt();
    let speed = if lm.run { RUN_SPEED } else { WALK_SPEED };
    let step = speed * dt;

    // 动画：走路/跑步 + 方向（walk 向前看 2 个节点避免方向乱跳）
    let desired = if use_run {
        direction_from_delta(d1.0, d1.1)
    } else if let Some(last) = lm.last {
        lookahead_direction(last, &lm.path)
            .or_else(|| direction_from_delta(first.0 - last.0, first.1 - last.1))
    } else {
        direction_from_delta(first.0 - cur.0, first.1 - cur.1)
    }
    .unwrap_or(mir2_shared::enums::MirDirection::Up) as u8;
    // 固定角速度转向：每 125ms 转 1 个方向（8 方向/秒，平滑不抖动）
    lm.turn_acc += dt;
    let mut turn_steps = 0i32;
    while lm.turn_acc >= 0.125 {
        lm.turn_acc -= 0.125;
        turn_steps += 1;
    }
    anim.direction = step_towards_direction(anim.direction, desired, turn_steps.max(1));
    anim.action = if lm.run {
        mir2_shared::enums::MirAction::Running
    } else {
        mir2_shared::enums::MirAction::Walking
    };

    if dist <= step || dist < ARRIVAL {
        // 到达目标格：对齐并推进（seg_dir 用单格步进方向，保证任意 8 方向都能发包）
        let seg_dir = direction_from_delta(d1.0, d1.1);
        tf.translation.x = target_world.x;
        tf.translation.y = target_world.y;
        if use_run {
            lm.path.pop_front();
            lm.path.pop_front();
        } else {
            lm.path.pop_front();
        }
        lm.last = Some(target);
        if let Some(d) = seg_dir {
            tracing::debug!("🚶 到达发包: from=({},{}) target=({},{}) dir={:?} run={}", from.0, from.1, target.0, target.1, d, use_run);
            if use_run {
                net.send_packet(&mir2_shared::packets::client::movement::Run { direction: d });
            } else {
                net.send_packet(&mir2_shared::packets::client::movement::Walk { direction: d });
            }
        } else {
            tracing::debug!("🚶 到达跳过发包: from=({},{}) target=({},{}) seg_dir=None run={}", from.0, from.1, target.0, target.1, use_run);
        }
    } else {
        // 平滑滑向目标
        tf.translation.x += dx / dist * step;
        tf.translation.y += dy / dist * step;
    }
    // z 深度跟随脚底
    tf.translation.z = depth_z(-tf.translation.y);
}


#[cfg(test)]
mod tests {
    use super::*;
    use crate::actor::{ActorAnim, NetObjectId};

    #[test]
    fn test_tile_world_roundtrip() {
        for (tx, ty) in [(0i32, 0i32), (5, 3), (100, 200)] {
            let w = tile_to_world(tx, ty);
            let (back_tx, back_ty) = world_to_tile(w.x, w.y);
            assert_eq!((back_tx, back_ty), (tx, ty), "roundtrip ({},{})", tx, ty);
        }
    }

    #[test]
    fn test_direction_from_delta() {
        assert_eq!(direction_from_delta(0, -1), Some(MirDirection::Up));
        assert_eq!(direction_from_delta(1, 1), Some(MirDirection::DownRight));
        assert_eq!(direction_from_delta(-1, 0), Some(MirDirection::Left));
        assert_eq!(direction_from_delta(0, 0), None);
    }

    /// 网络→游戏消息管道：MessageWriter 写入 NetMotion，
    /// apply_net_motions 同帧消费并更新角色朝向（替代原手写 Vec 队列）。
    #[test]
    fn net_motion_message_pipeline() {
        let mut app = App::new();
        app.add_plugins(MinimalPlugins);
        app.add_message::<NetMotion>();
        app.add_systems(Update, apply_net_motions);

        // 非本地角色实体（无 LocalPlayer）
        let e = app
            .world_mut()
            .spawn((NetObjectId(7), ActorAnim::default(), Transform::default()))
            .id();

        // 服务器 Turn 消息 → 同帧消费并转向
        app.world_mut()
            .resource_mut::<Messages<NetMotion>>()
            .write(NetMotion::Turn {
                object_id: 7,
                x: 0,
                y: 0,
                dir: 3,
            });
        app.update();

        let anim = app.world().get::<ActorAnim>(e).unwrap();
        assert_eq!(anim.direction, 3);
        assert_eq!(anim.action, mir2_shared::enums::MirAction::Standing);
    }
}
