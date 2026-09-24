// ============================================================================
// 移动（M8）
// 本地玩家：沿寻路路径按 100ms/格 步进（原版 GameScene.MoveTime=100），
//   每步发 Walk/Run 包（参考 Client-Macroquad player_control_system：Run=2 格）。
// 远端对象：ObjectWalk/Run/Turn 插值移动（参考 macroquad position_interpolation）。
// ============================================================================

use std::collections::VecDeque;

use bevy::prelude::*;
use mir2_shared::enums::MirDirection;

use crate::actor::{depth_z, ActorAnim, LocalPlayer, MountState, NetObjectId, Sitting};
use crate::game::sound::SoundBank;
use crate::map_renderer::{GameData, TILE_HEIGHT, TILE_WIDTH};
use crate::network::{NetConnection, SessionState};
use crate::scenes::AppState;

/// 服务器对象移动事件（网络 handler 发送，移动系统消费）
#[derive(Message, Debug, Clone)]
pub enum NetMotion {
    Walk {
        object_id: u32,
        x: i32,
        y: i32,
        dir: u8,
    },
    Run {
        object_id: u32,
        x: i32,
        y: i32,
        dir: u8,
    },
    Turn {
        object_id: u32,
        x: i32,
        y: i32,
        dir: u8,
    },
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

/// 收到服务端权威位置时，本地是否该**作废**正在进行的寻路（并就地采纳服务端位置）。
///
/// 判据（2026-09-24 实测标定）：
/// - 偏差 ≤ 2 格 = 客户端预测的正常领先。一次 `Run` 就是 2 格，移动包又在"到达那一步"才发，
///   所以服务端位置天然可能落后 2 格；此时打断本地路径会让路径永远走不完（#77 的教训）。
/// - 偏差 ≥ 3 格 = 瞬移级（换图/`@mapmove`/被拒后跑偏）。实测连续 `@mapmove` 时客户端带着过期
///   路径一路跑，5s 一次采样读数 417→434→449→464→478 而服务端原地不动，永不收敛；
///   此时必须采纳服务端位置并清掉那条过期路径。
pub fn should_abort_local_move(client_tile: (i32, i32), server_tile: (i32, i32)) -> bool {
    (client_tile.0 - server_tile.0)
        .abs()
        .max((client_tile.1 - server_tile.1).abs())
        >= 3
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
fn lookahead_direction(
    last: (i32, i32),
    path: &VecDeque<(i32, i32)>,
) -> Option<mir2_shared::enums::MirDirection> {
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

/// #1548：C# Functions.NextDir（顺时针下一方向）
pub fn next_direction(d: MirDirection) -> MirDirection {
    match d {
        MirDirection::Up => MirDirection::UpRight,
        MirDirection::UpRight => MirDirection::Right,
        MirDirection::Right => MirDirection::DownRight,
        MirDirection::DownRight => MirDirection::Down,
        MirDirection::Down => MirDirection::DownLeft,
        MirDirection::DownLeft => MirDirection::Left,
        MirDirection::Left => MirDirection::UpLeft,
        MirDirection::UpLeft => MirDirection::Up,
    }
}

/// #1548：C# Functions.PreviousDir（逆时针上一方向）
pub fn previous_direction(d: MirDirection) -> MirDirection {
    match d {
        MirDirection::Up => MirDirection::UpLeft,
        MirDirection::UpLeft => MirDirection::Left,
        MirDirection::Left => MirDirection::DownLeft,
        MirDirection::DownLeft => MirDirection::Down,
        MirDirection::Down => MirDirection::DownRight,
        MirDirection::DownRight => MirDirection::Right,
        MirDirection::Right => MirDirection::UpRight,
        MirDirection::UpRight => MirDirection::Up,
    }
}

/// #1548：C# Functions.PointMove（从 (x,y) 沿 dir 走 dist 格）
pub fn point_move(x: i32, y: i32, dir: MirDirection, dist: i32) -> (i32, i32) {
    let (dx, dy) = match dir {
        MirDirection::Up => (0, -1),
        MirDirection::UpRight => (1, -1),
        MirDirection::Right => (1, 0),
        MirDirection::DownRight => (1, 1),
        MirDirection::Down => (0, 1),
        MirDirection::DownLeft => (-1, 1),
        MirDirection::Left => (-1, 0),
        MirDirection::UpLeft => (-1, -1),
        _ => (0, 0),
    };
    (x + dx * dist, y + dy * dist)
}

/// #1548：鼠标相对玩家 → 8 方向扇区（对齐 C# GameScene.MouseDirection(45F)）
/// 用角度扇区（45° 容差）而非瓦片差：鼠标在扇区内移动方向稳定，不抖动
pub fn mouse_direction(player_world: Vec2, mouse_world: Vec2) -> MirDirection {
    let dx = mouse_world.x - player_world.x;
    let dy = mouse_world.y - player_world.y;
    // 玩家脚下极小范围：不转向（C# InRange(p, 2) 归零防抖）
    if dx.abs() < 8.0 && dy.abs() < 8.0 {
        return MirDirection::Up;
    }
    let angle = dy.atan2(dx).to_degrees(); // [-180, 180]，0°=正右、逆时针正
                                           // C# MouseDirection：0°=正上（Up）、顺时针 45°/扇区；数学角转 C# 角 = 90° - angle
    let mut deg = 90.0 - angle + 22.5;
    if deg < 0.0 {
        deg += 360.0;
    }
    let sector = ((deg / 45.0) as i32).rem_euclid(8);
    match sector {
        0 => MirDirection::Up,
        1 => MirDirection::UpRight,
        2 => MirDirection::Right,
        3 => MirDirection::DownRight,
        4 => MirDirection::Down,
        5 => MirDirection::DownLeft,
        6 => MirDirection::Left,
        _ => MirDirection::UpLeft,
    }
}

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
    mut commands: Commands,
    mut session: ResMut<SessionState>,
    // 本地玩家同时带 NetObjectId（此前误用 Without<NetObjectId> 把玩家自己排除，
    // 服务器 UserLocation 校正永不生效 → 客户端位置漂移（#57 实测）
    mut players: Query<(Entity, &mut Transform), (With<LocalPlayer>, With<NetObjectId>)>,
    local_moves: Query<(), (With<LocalPlayer>, With<LocalMove>)>,
) {
    // 本地移动中：服务器位置必然滞后于客户端（网络往返），瞬移校正会与 LocalMove 每帧拉扯，
    // 导致路径永远走不完（#77 实测：客户端 10 格路径只发 1 个 Run，服务器坐标停在出生点附近）。
    // 不消费该校正值，等移动结束后下一帧再应用。
    //
    // **例外（2026-09-24 实测的"越跑越偏"）**：偏差达到"瞬移级"时必须立刻采纳并**作废本地寻路**。
    // 实机复现：连续 `@mapmove 2 <x> <y>`（服务端每次都精确落到目标格）时，客户端带着**过期路径**
    // 继续往前跑——5s 采样一次分别读到 (417,201)、(434,201)、(449,212)、(464,227)、(478,239)，
    // 而服务端一直在 (399,199)/(400,199)/(402,200) 附近：越跑越偏、永不收敛。
    // 近战方向、拾取距离、点 NPC 判定全部按**本地格**算 ⇒ 这种跑偏会让战斗与交互整体失效。
    if !local_moves.is_empty() {
        if let Some((tx, ty, _dir)) = session.self_position {
            if let Ok((e, mut tf)) = players.single_mut() {
                let cur = world_to_tile(tf.translation.x, tf.translation.y);
                if should_abort_local_move(cur, (tx, ty)) {
                    let p = tile_to_world(tx, ty);
                    tf.translation.x = p.x;
                    tf.translation.y = p.y;
                    tf.translation.z = depth_z(-p.y);
                    commands.entity(e).remove::<LocalMove>();
                    session.self_position = None;
                    tracing::info!(
                        "📍 瞬移级校正 -> ({},{})：本地寻路作废（原本地格 ({},{}）",
                        tx,
                        ty,
                        cur.0,
                        cur.1
                    );
                }
            }
        }
        return;
    }
    let Some((tx, ty, _dir)) = session.self_position.take() else {
        return;
    };
    let Ok((_e, mut tf)) = players.single_mut() else {
        tracing::debug!(
            "📍 位置校正：玩家 Query 未匹配（self_position 丢弃 ({},{})）",
            tx,
            ty
        );
        return;
    };
    let cur = world_to_tile(tf.translation.x, tf.translation.y);
    tracing::debug!(
        "📍 位置校正检查：server=({},{}) cur=({},{})",
        tx,
        ty,
        cur.0,
        cur.1
    );
    // 服务器位置是**权威**：只要和本地不一致就采用（C# `GameScene.UserLocation`
    // `Client/MirScenes/GameScene.cs:2244-2252` 就是无条件 `User.CurrentLocation = p.Location`）。
    //
    // 此前门限是「距离 > 2 格才校正」——**1 格偏差永远校不回来**，实机后果（2026-09-24）：
    // 本地预测多走一格被服务端拒掉后，客户端认为在 (289,611)、服务端记的是 (289,612)；
    // 近战按"正前方一格"结算落在空地上 → 64 次攻击全是空挥、怪物一点血不掉；
    // 同理拾取/点 NPC 也会按错格判距离。
    // 边界：本地正在寻路（LocalMove 非空）时前面已 return，不会与预测每帧拉扯。
    if cur != (tx, ty) {
        let p = tile_to_world(tx, ty);
        tf.translation.x = p.x;
        tf.translation.y = p.y;
        tf.translation.z = depth_z(-p.y);
        tracing::info!("📍 服务器位置校正 -> ({},{})", tx, ty);
    }
}

/// 安全版「延迟组件操作」：实体可能在命令落地前就被 despawn（**换图重建**就是这种情况——
/// `MapChanged` 会整批重建场景对象，本地玩家实体也在其中）。
///
/// 为什么需要它（#3028 记录的真实故障）：`apply_net_motions`/`object_state` 这类系统用
/// `commands.entity(e).insert/remove` 时捕获的是**当时的** `Entity`；若同一帧稍后另一个
/// 系统把该实体 despawn（换图重建），命令落地时就命中失效实体：
///
/// ```text
/// WARN bevy_ecs::error::handler: Encountered an error in command `<...remove<Sitting>...>`:
///      Entity despawned: The entity with ID 18081v1 is invalid; its index now has generation 2.
/// ```
///
/// 后果不止一条日志：本地玩家实体连同它的状态一起消失（`state` 探针读不到 tile，玩家视角卡死）。
/// 这里改成 `commands.queue` + `get_entity_mut`，**在命令真正落地时**再确认实体还有效，
/// 失效就静默跳过（语义正确：实体都没了，本来也不需要再改它）。
///
/// 阳性对照见 `tests::entity_command_safety`（同一条测试里同时验证「不安全写法会被抓到」，
/// 证明这道门禁真的能红）。
pub(crate) fn safe_insert<B: bevy::prelude::Bundle>(
    commands: &mut bevy::prelude::Commands,
    entity: bevy::prelude::Entity,
    bundle: B,
) {
    commands.queue(move |world: &mut bevy::prelude::World| {
        if let Ok(mut ec) = world.get_entity_mut(entity) {
            ec.insert(bundle);
        }
    });
}

/// 见 [`safe_insert`]。移除失效实体上的组件同样应当静默跳过。
pub(crate) fn safe_remove<C: bevy::prelude::Component>(
    commands: &mut bevy::prelude::Commands,
    entity: bevy::prelude::Entity,
) {
    commands.queue(move |world: &mut bevy::prelude::World| {
        if let Ok(mut ec) = world.get_entity_mut(entity) {
            ec.remove::<C>();
        }
    });
}

/// 见 [`safe_insert`]。**删除**同样是延迟命令：实体可能已被换图重建/其它系统在本帧删掉，
/// 直接 `commands.entity(e).despawn()` 会打到失效 Entity（`despawn` 在 Bevy 里是
/// `queue_handled(.., warn)`，但为一致性与可读性，本仓统一走这一个出口）。
pub(crate) fn safe_despawn(commands: &mut bevy::prelude::Commands, entity: bevy::prelude::Entity) {
    commands.queue(move |world: &mut bevy::prelude::World| {
        if let Ok(ec) = world.get_entity_mut(entity) {
            ec.despawn();
        }
    });
}

/// 见 [`safe_insert`]。**挂子实体**同样要防「父实体在命令落地前已被 despawn」。
///
/// 与 `insert/remove/despawn` 的区别（实测，勿照抄结论）：Bevy 的
/// `EntityCommands::with_children` → `with_related_entities` 是**立即执行**的——它只是把
/// 子实体的 `spawn((bundle, ChildOf(parent)))` 排进命令队列，**不会**对父实体做任何操作，
/// 所以命中失效父实体时**不 panic**；代价是**留下孤儿子实体**（血条/伤害飘字挂在已消失的对象上）。
/// 本出口的作用就是「父实体没了就别生成孤儿」，与 [`safe_insert`] 等保持同一个出口概念。
///
/// 闭包类型用 **World 版** `ChildSpawner`（`EntityWorldMut::with_children` 的参数类型），
/// 因此调用方写法与原来的 `p.spawn((..))` 完全一致，只把 `commands.entity(e)` 换成本函数。
///
/// 阳性对照见 `tests::entity_command_safety_survives_despawn`：
/// 裸 `with_children` 必须留下孤儿子实体（错误处理计数仍为 0），本出口必须留 0。
pub(crate) fn safe_with_children<F>(
    commands: &mut bevy::prelude::Commands,
    parent: bevy::prelude::Entity,
    spawn_children: F,
) where
    F: FnOnce(&mut bevy::ecs::hierarchy::ChildSpawner) + Send + Sync + 'static,
{
    commands.queue(move |world: &mut bevy::prelude::World| {
        if let Ok(mut ec) = world.get_entity_mut(parent) {
            ec.with_children(spawn_children);
        }
    });
}

/// 消耗 NetMotions：给对象实体挂 MoveTween / 转向
fn apply_net_motions(
    mut commands: Commands,
    mut motions: MessageReader<NetMotion>,
    mut actors: Query<(
        Entity,
        &NetObjectId,
        &mut ActorAnim,
        &Transform,
        Option<&LocalPlayer>,
    )>,
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
                    // #573：移动/转身即解除坐下（C# 坐下状态被移动打断）
                    safe_remove::<Sitting>(&mut commands, e);
                    anim.direction = dir;
                    anim.action = mir2_shared::enums::MirAction::Standing;
                    anim.frame_index = 0;
                }
                NetMotion::Walk { x, y, dir, .. } => {
                    safe_remove::<Sitting>(&mut commands, e);
                    safe_insert(
                        &mut commands,
                        e,
                        MoveTween {
                            from,
                            to: tile_to_world(x, y),
                            t: 0.0,
                            dur: 0.16,
                            action: mir2_shared::enums::MirAction::Walking,
                            dir,
                        },
                    );
                    anim.action = mir2_shared::enums::MirAction::Walking;
                    anim.direction = dir;
                    anim.frame_index = 0;
                }
                NetMotion::Run { x, y, dir, .. } => {
                    safe_remove::<Sitting>(&mut commands, e);
                    safe_insert(
                        &mut commands,
                        e,
                        MoveTween {
                            from,
                            to: tile_to_world(x, y),
                            t: 0.0,
                            dur: 0.20,
                            action: mir2_shared::enums::MirAction::Running,
                            dir,
                        },
                    );
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
    mut actors: Query<(
        Entity,
        &mut MoveTween,
        &mut Transform,
        &mut ActorAnim,
        Option<&LocalMove>,
    )>,
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
    game_data: Res<GameData>,
    // #2633 批次4 步7：步声骑乘参数改读 `MountState`（HudState 已于步9 删除）；
    // 实体缺失视同未骑乘（原 hud.riding=false 默认）
    mount_q: Query<&MountState, With<LocalPlayer>>,
    sound_bank: Res<SoundBank>,
    mut audio_assets: ResMut<Assets<AudioSource>>,
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
    // C# CanRun：stepCounter > 0 才能跑（Walk 先走一格热身）。首段降级 Walk，避免
    // stepCounter=0 时发 Run 被服务器拒绝，导致客户端/服务器位置不同步（#2498）。
    let warmed_up = lm.last.is_some();
    let mut use_run = warmed_up && lm.run && lm.path.len() >= 2 && d1.0.abs() + d1.1.abs() == 1;
    if use_run {
        let second = *lm.path.get(1).unwrap();
        let d2 = (second.0 - first.0, second.1 - first.1);
        use_run = d1 == d2; // 仅同向直线
    }
    let target = if use_run {
        *lm.path.get(1).unwrap()
    } else {
        first
    };
    tracing::debug!(
        "🚶 move: use_run={} path_len={} target=({},{})",
        use_run,
        lm.path.len(),
        target.0,
        target.1
    );
    let target_world = tile_to_world(target.0, target.1);
    let dx = target_world.x - tf.translation.x;
    let dy = target_world.y - tf.translation.y;
    let dist = (dx * dx + dy * dy).sqrt();
    let speed = if lm.run { RUN_SPEED } else { WALK_SPEED };
    let step = speed * dt;

    // 动画：走路/跑步 + 方向（walk 向前看 2 个节点避免方向乱跳）
    // 首段用稳定段起点（from=last/step_origin）而非实时 cur——滑行中 cur 会在瓦片
    // 边界漂移，若用 first-cur 计算方向会在 8 方向间乱跳（对角移动方向抖动根因）
    let desired = if use_run {
        direction_from_delta(d1.0, d1.1)
    } else if let Some(last) = lm.last {
        lookahead_direction(last, &lm.path)
            .or_else(|| direction_from_delta(first.0 - last.0, first.1 - last.1))
    } else {
        direction_from_delta(first.0 - from.0, first.1 - from.1)
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
        // #1572：到达目标格播步声（C# PlayStepSound 按地面类型/跑/骑乘/帧）
        if let Some(map_reader) = &game_data.map_reader {
            if target.0 >= 0 && target.1 >= 0 {
                let cells = &map_reader.map_cells;
                if let Some(row) = cells.get(target.0 as usize) {
                    if let Some(cell) = row.get(target.1 as usize) {
                        if let Some(sound_id) = crate::game::sound::step_sound_for_cell(
                            cell,
                            use_run,
                            mount_q.single().is_ok(),
                            anim.frame_index.clamp(0, 255) as u8,
                        ) {
                            crate::game::sound::play_sound(
                                &mut commands,
                                &mut audio_assets,
                                &sound_bank,
                                sound_id,
                            );
                            tracing::debug!(
                                "👣 步声 #{} @ ({},{}) run={}",
                                sound_id,
                                target.0,
                                target.1,
                                use_run
                            );
                        }
                    }
                }
            }
        }
        if let Some(d) = seg_dir {
            tracing::debug!(
                "🚶 到达发包: from=({},{}) target=({},{}) dir={:?} run={}",
                from.0,
                from.1,
                target.0,
                target.1,
                d,
                use_run
            );
            if use_run {
                net.send_packet(&mir2_shared::packets::client::movement::Run { direction: d });
            } else {
                net.send_packet(&mir2_shared::packets::client::movement::Walk { direction: d });
            }
        } else {
            tracing::debug!(
                "🚶 到达跳过发包: from=({},{}) target=({},{}) seg_dir=None run={}",
                from.0,
                from.1,
                target.0,
                target.1,
                use_run
            );
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
    /// 回归（2026-09-24 实机挖出）：本地预测超前服务器**1 格**时，`UserLocation` 校正必须
    /// 把本地玩家拉回服务器位置。
    ///
    /// 此前门限是「距离 > 2 格才校正」→ 1 格偏差永远校不回来。实机后果：客户端以为在
    /// (289,611)、服务端记的是 (289,612)，近战按"正前方一格"结算落在空地 →
    /// 连打 64 次全是空挥、怪物技能不掉血（`l5a_combat` 首跑即此现象）。
    /// C# `GameScene.UserLocation`（`Client/MirScenes/GameScene.cs:2244-2252`）是**无条件采用**。
    ///
    /// 阳性对照（实做）：把门限改回 `dist > 2` → 本测试立即红。
    #[test]
    fn user_location_corrects_one_tile_desync() {
        use crate::actor::LocalPlayer;
        use crate::actor::NetObjectId;
        use crate::network::SessionState;
        use bevy::prelude::*;

        let mut app = App::new();
        app.add_plugins(MinimalPlugins);
        app.init_resource::<SessionState>();
        app.add_systems(Update, super::apply_self_position);

        let start = super::tile_to_world(289, 611);
        let player = app
            .world_mut()
            .spawn((
                LocalPlayer,
                NetObjectId(7),
                Transform::from_xyz(start.x, start.y, 0.0),
            ))
            .id();
        // 服务端权威位置比客户端多 1 格（本地预测多走一格、被服务端拒掉后的典型形态）
        app.world_mut().resource_mut::<SessionState>().self_position = Some((289, 612, 0));
        app.update();

        let tf = app.world().get::<Transform>(player).unwrap();
        assert_eq!(
            super::world_to_tile(tf.translation.x, tf.translation.y),
            (289, 612),
            "1 格偏差必须被 UserLocation 校正拉回服务器位置"
        );
    }

    use super::*;

    /// 门禁（#3028「死亡态 + 换图」丢本地玩家实体）：命令落地前实体被 despawn 时，
    /// **不得**产生 ECS 错误（错的就是这一条：`insert<MoveTween>`/`remove<Sitting>`
    /// 命中已失效实体 → 日志刷 "Entity despawned ..." 且本地玩家实体消失）。
    ///
    /// 阳性对照写在同一条测试里：不安全写法（`commands.entity(e).insert(..)`）**必须**
    /// 被同一个错误探测抓到（计数 > 0），否则说明这道门禁是假的、永远不会红。
    #[test]
    fn entity_command_safety_survives_despawn() {
        use bevy::ecs::error::{BevyError, ErrorContext, ErrorHandler, FallbackErrorHandler};
        use std::sync::atomic::{AtomicUsize, Ordering};
        static SINK: AtomicUsize = AtomicUsize::new(0);
        fn sink(_e: BevyError, _c: ErrorContext) {
            SINK.fetch_add(1, Ordering::SeqCst);
        }

        #[derive(Component)]
        struct Mark;

        #[derive(Resource)]
        struct Target(Entity);

        // unsafe_mode=true 走旧写法（阳性对照）；false 走 safe_* helper（本轮修复）
        // 返回 (错误处理被调用次数, 孤儿 Mark 实体数)
        fn drive(unsafe_mode: bool) -> (usize, usize) {
            #[derive(Resource)]
            struct Mode(bool);
            let mut app = App::new();
            app.add_plugins(MinimalPlugins);
            app.insert_resource(FallbackErrorHandler(sink as ErrorHandler));
            let e = app.world_mut().spawn(Mark).id();
            app.insert_resource(Target(e));
            app.insert_resource(Mode(unsafe_mode));
            app.add_systems(
                Update,
                (
                    // ① 换图重建语义：先 despawn；chain 的 sync point 让它在本系统之后落地
                    |mut commands: Commands, t: Res<Target>| {
                        commands.entity(t.0).despawn();
                    },
                    // ② 稍后的系统仍拿着**旧 Entity** 排队列组件操作（真实故障形态）
                    |mut commands: Commands, t: Res<Target>, mode: Res<Mode>| {
                        if mode.0 {
                            commands.entity(t.0).insert(MoveTween {
                                from: Vec2::ZERO,
                                to: Vec2::ZERO,
                                t: 0.0,
                                dur: 0.16,
                                action: mir2_shared::enums::MirAction::Walking,
                                dir: 0,
                            });
                            // 同族第三处：**挂子实体**（血条 / 伤害飘字）打到已 despawn 的父实体
                            commands.entity(t.0).with_children(|p| {
                                p.spawn(Mark);
                            });
                        } else {
                            crate::game::movement::safe_insert(
                                &mut commands,
                                t.0,
                                MoveTween {
                                    from: Vec2::ZERO,
                                    to: Vec2::ZERO,
                                    t: 0.0,
                                    dur: 0.16,
                                    action: mir2_shared::enums::MirAction::Walking,
                                    dir: 0,
                                },
                            );
                            crate::game::movement::safe_remove::<Sitting>(&mut commands, t.0);
                            crate::game::movement::safe_with_children(&mut commands, t.0, |p| {
                                p.spawn(Mark);
                            });
                        }
                    },
                )
                    .chain(),
            );
            SINK.store(0, Ordering::SeqCst);
            app.update();
            let sink = SINK.load(Ordering::SeqCst);
            let orphans = app.world_mut().query::<&Mark>().iter(app.world()).count();
            (sink, orphans)
        }

        let (unsafe_errors, unsafe_orphans) = drive(true);
        assert!(
            unsafe_errors > 0,
            "阳性对照：不安全写法必须被错误探测抓到，否则这道门禁是假的"
        );
        // Bevy 的 `EntityCommands::with_children` 是**立即执行**（`with_related_entities` 直接
        // 把子实体 spawn 排进命令队列）⇒ 父实体已 despawn 时它**不报错**，但会留下**孤儿子实体**。
        // 这正是它和 `insert/remove/despawn` 的区别：不是崩溃风险，而是脏实体风险。
        assert_eq!(
            unsafe_orphans, 1,
            "阳性对照②：裸 with_children 会为已 despawn 的父实体生成孤儿子实体"
        );
        let (safe_errors, safe_orphans) = drive(false);
        assert_eq!(
            safe_errors, 0,
            "实体被 despawn 后，safe_insert/safe_remove/safe_with_children 不得产生 ECS 错误（#3028）"
        );
        assert_eq!(
            safe_orphans, 0,
            "safe_with_children 必须跳过已 despawn 的父实体（不留孤儿）"
        );
    }
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

/// 诊断：模拟对角直线 + 转弯路径的逐段方向序列（验证方向是否抖动）
#[test]
fn diag_diagonal_direction_sequence() {
    use std::collections::VecDeque;
    // 场景：玩家 (0,0)，点击对角远处 (5,3) → 3 对角 + 2 直线
    let path: VecDeque<(i32, i32)> = [(1, 1), (2, 2), (3, 3), (4, 3), (5, 3)]
        .into_iter()
        .collect();
    let mut last: Option<(i32, i32)> = None;
    let mut dir: u8 = 0; // 初始 Up
    let mut seq: Vec<u8> = Vec::new();
    let mut p = path.clone();
    // 模拟"到达"序列：每到达一格记录稳定后的方向
    while let Some(&first) = p.front() {
        let from = last.unwrap_or((0, 0));
        let desired = if let Some(l) = last {
            lookahead_direction(l, &p)
                .or_else(|| direction_from_delta(first.0 - l.0, first.1 - l.1))
                .unwrap_or(mir2_shared::enums::MirDirection::Up) as u8
        } else {
            direction_from_delta(first.0 - from.0, first.1 - from.1)
                .unwrap_or(mir2_shared::enums::MirDirection::Up) as u8
        };
        // 逐步转向到 desired（最多 8 步，模拟足够时间转到位）
        for _ in 0..8 {
            dir = step_towards_direction(dir, desired, 1);
        }
        seq.push(dir);
        last = Some(first);
        p.pop_front();
    }
    // 期望：对角段稳定 DownRight(3)，直线段稳定 Right(1)，无来回跳
    eprintln!("方向序列: {:?}", seq);
    // 抖动检查：相邻方向差 <= 1（不允许来回大幅摆动）
    for w in seq.windows(2) {
        let diff = (w[1] as i32 - w[0] as i32).rem_euclid(8);
        assert!(diff <= 2 || diff >= 6, "方向抖动: {} -> {}", w[0], w[1]);
    }
}

/// 集成实测：advance_local_move 对角路径移动，检查 anim.direction 是否抖动
#[test]
fn diag_advance_local_move_direction_stability() {
    use std::time::Duration;
    let mut app = App::new();
    app.add_plugins(MinimalPlugins);
    app.insert_resource(crate::network::NetConnection::default());
    // #1572：advance_local_move 新增步声依赖
    app.insert_resource(crate::map_renderer::GameData {
        map: None,
        map_reader: None,
        desired_map: None,
        player_spawn: None,
        minimap_index: 0,
        map_title: String::new(),
    });
    app.insert_resource(crate::game::sound::SoundBank::default());
    app.insert_resource(bevy::asset::Assets::<bevy::audio::AudioSource>::default());
    app.add_systems(Update, advance_local_move);

    // 对角路径（3 对角 + 2 直线）
    let path: std::collections::VecDeque<(i32, i32)> = [(1, 1), (2, 2), (3, 3), (4, 3), (5, 3)]
        .into_iter()
        .collect();
    app.world_mut().spawn((
        crate::actor::LocalPlayer,
        LocalMove {
            path,
            step_timer_ms: 0.0,
            run: false,
            last: None,
            step_origin: None,
            turn_acc: 0.0,
        },
        Transform::from_translation(tile_to_world(0, 0).extend(0.0)),
        crate::actor::ActorAnim::default(),
    ));

    let mut dirs: Vec<u8> = Vec::new();
    let mut prev_tf: Option<f32> = None;
    let mut moved = false;
    for _ in 0..240 {
        app.world_mut()
            .resource_mut::<Time>()
            .advance_by(Duration::from_millis(16));
        app.update();
        let world = app.world_mut();
        let mut q = world.query::<(&crate::actor::ActorAnim, &Transform)>();
        let (anim, tf) = q.single(&*world).unwrap();
        dirs.push(anim.direction);
        if prev_tf
            .map(|p| (tf.translation.x - p).abs() > 0.01)
            .unwrap_or(false)
        {
            moved = true;
        }
        prev_tf = Some(tf.translation.x);
    }
    eprintln!("移动发生: {}", moved);
    eprintln!("方向序列(前 40): {:?}", &dirs[..40.min(dirs.len())]);
    eprintln!(
        "方向集合: {:?}",
        dirs.iter().collect::<std::collections::HashSet<_>>()
    );
    // 抖动检查：连续帧方向差（环形）>1 的次数应很少（转向期间允许短暂过渡）
    let mut flips = 0;
    for w in dirs.windows(2) {
        let diff = (w[1] as i32 - w[0] as i32).rem_euclid(8);
        if diff != 0 && diff != 1 && diff != 7 {
            flips += 1;
        }
    }
    eprintln!("大跳变帧数: {}", flips);
    assert!(flips <= 4, "方向抖动过大: flips={}", flips);
}

/// 诊断：find_path 对角目标是否产生平滑直线（无锯齿 = 路线不偏离）
#[test]
fn diag_find_path_diagonal_smooth() {
    let map = crate::map_renderer::LoadedMap {
        name: "test".into(),
        width: 20,
        height: 20,
        walkable: vec![vec![true; 20]; 20],
        doors: vec![vec![0u8; 20]; 20],
    };
    let path = crate::game::pathfinding::find_path(&map, (0, 0), (5, 3)).unwrap();
    let mut deltas = Vec::new();
    let mut prev = (0, 0);
    for &n in &path {
        deltas.push((n.0 - prev.0, n.1 - prev.1));
        prev = n;
    }
    eprintln!("path: {:?}", path);
    eprintln!("deltas: {:?}", deltas);
    // 理论最短：max(|dx|,|dy|)=5 步（3 对角 + 2 直）
    assert!(path.len() <= 6, "路径过长(锯齿/绕路): {:?}", path);
    // 无锯齿：delta 不应出现 "横→竖→横" 交替（如 (1,0),(0,1),(1,0)）
    for w in deltas.windows(3) {
        let a = (w[0].0.abs(), w[0].1.abs());
        let b = (w[1].0.abs(), w[1].1.abs());
        let c = (w[2].0.abs(), w[2].1.abs());
        let zigzag =
            a == (1, 0) && b == (0, 1) && c == (1, 0) || a == (0, 1) && b == (1, 0) && c == (0, 1);
        assert!(!zigzag, "锯齿路径: {:?}", deltas);
    }
}
#[test]
fn test_mouse_direction_sectors_stable() {
    let player = Vec2::new(0.0, 0.0);
    assert_eq!(
        mouse_direction(player, Vec2::new(100.0, 0.0)),
        MirDirection::Right
    );
    assert_eq!(
        mouse_direction(player, Vec2::new(100.0, 100.0)),
        MirDirection::UpRight
    );
    assert_eq!(
        mouse_direction(player, Vec2::new(0.0, 100.0)),
        MirDirection::Up
    );
    assert_eq!(
        mouse_direction(player, Vec2::new(0.0, -100.0)),
        MirDirection::Down
    );
    assert_eq!(
        mouse_direction(player, Vec2::new(-100.0, -100.0)),
        MirDirection::DownLeft
    );
    // 玩家脚下 → 防抖 Up
    assert_eq!(
        mouse_direction(player, Vec2::new(3.0, -3.0)),
        MirDirection::Up
    );
    // 扇区内稳定：角度 20° 与 10° 都应是 Right（0°~22.5° 边界容差内）
    assert_eq!(
        mouse_direction(player, Vec2::new(100.0, 18.0)),
        MirDirection::Right
    );
    assert_eq!(
        mouse_direction(player, Vec2::new(100.0, 10.0)),
        MirDirection::Right
    );
}

#[test]
fn test_next_previous_direction_roundtrip() {
    for d in [
        MirDirection::Up,
        MirDirection::UpRight,
        MirDirection::Right,
        MirDirection::DownRight,
        MirDirection::Down,
        MirDirection::DownLeft,
        MirDirection::Left,
        MirDirection::UpLeft,
    ] {
        let n = next_direction(d);
        assert_eq!(previous_direction(n), d, "next+previous 应还原 {}", d as u8);
    }
    assert_eq!(next_direction(MirDirection::UpLeft), MirDirection::Up);
    assert_eq!(previous_direction(MirDirection::Up), MirDirection::UpLeft);
}

#[test]
fn test_point_move_distances() {
    assert_eq!(point_move(5, 5, MirDirection::Up, 1), (5, 4));
    assert_eq!(point_move(5, 5, MirDirection::Right, 2), (7, 5));
    assert_eq!(point_move(5, 5, MirDirection::DownLeft, 1), (4, 6));
}

#[test]
fn test_walk_fallback_tries_next_then_previous() {
    // #1548：C# CanWalk(dir, out dir)：原方向不可走 → NextDir → PreviousDir
    let mut walkable = vec![vec![true; 3]; 3];
    walkable[1][0] = false; // 北墙
    let map = crate::map_renderer::LoadedMap {
        name: String::new(),
        width: 3,
        height: 3,
        walkable,
        doors: vec![vec![0u8; 3]; 3],
    };
    let from = (1, 1);
    let dir = MirDirection::Up;
    let mut chosen = None;
    for d in [dir, next_direction(dir), previous_direction(dir)] {
        let pp = point_move(from.0, from.1, d, 1);
        if map.is_walkable(pp.0, pp.1) {
            chosen = Some(d);
            break;
        }
    }
    assert_eq!(chosen, Some(MirDirection::UpRight), "北墙时应回退北东");
}

/// 门禁（2026-09-24 实机缺陷）：**瞬移级偏差必须打断本地寻路**，1-2 格偏差**不能**打断。
///
/// 实机复现（`@mapmove` 连续传送）：服务端每次精确落到目标格，客户端却带着过期路径继续跑，
/// 5s 采样一次读到 (417,201)→(434,201)→(449,212)→(464,227)→(478,239)，越跑越偏、永不收敛；
/// 近战方向/拾取距离/点 NPC 全按本地格算 ⇒ 战斗与交互整体失效。
/// 另一侧：一次 `Run` 就是 2 格、且移动包在"到达那一步"才发，所以 ≤2 格是正常预测领先，
/// 打断它会让路径永远走不完（#77 实测）。
///
/// 阳性对照（实做）：把门限改成 `>= 1` → 本测试里 1 格与 2 格两条断言立即红。
#[cfg(test)]
#[test]
fn should_abort_local_move_only_on_teleport_scale_drift() {
    // 正常：同格 / 预测领先 1-2 格（Run 两格）→ 不打断
    assert!(!should_abort_local_move((300, 300), (300, 300)));
    assert!(
        !should_abort_local_move((301, 300), (300, 300)),
        "领先 1 格是常态"
    );
    assert!(
        !should_abort_local_move((302, 301), (300, 300)),
        "一次 Run 的 2 格仍是常态"
    );
    // 瞬移级：≥3 格 → 必须打断（否则越跑越偏）
    assert!(
        should_abort_local_move((303, 300), (300, 300)),
        "3 格起算瞬移级"
    );
    assert!(
        should_abort_local_move((417, 201), (399, 199)),
        "实测跑偏样例"
    );
    // 同一输入连读一致
    assert_eq!(
        should_abort_local_move((478, 239), (402, 200)),
        should_abort_local_move((478, 239), (402, 200))
    );
}
