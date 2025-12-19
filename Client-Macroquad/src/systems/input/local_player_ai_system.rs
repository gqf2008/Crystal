// ============================================================================
// LocalPlayerAiSystem - 本地玩家自动战斗/挂机 AI
// Priority: priority::LOCAL_PLAYER_AI (119)
// ============================================================================
//
// 目标：让本地玩家在离线 mock 世界中，自动找怪 → 走近 → 攻击。
// 约束：
// - 支持开关：可随时启用/禁用 AI 控制
// - **模式互斥**：当启用挂机/AT/BT 控制时，本系统拥有本地玩家控制权；手动控制系统不应再写入 PlayerInput。
// - 复用现有链路：写入 PlayerInput.attack_target/move_to，由 CombatSystem/PathfindingSystem 驱动发包
//

use std::collections::HashSet;
use std::env;
use std::time::{Duration, Instant};

use macroquad::prelude::KeyCode;

use crate::{
    components::{Health, LocalPlayer, Monster, PlayerInput, Position, MovementMode},
    coord::Coord,
    game::{GameContext, GameResult},
    systems::LogicSystem,
};

use crate::ui::ui_state::{UiCommand, UiState};

// ============================================================================
// 轻量行为树（方案 A，自研最小实现）
// ============================================================================

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum BtStatus {
    Success,
    Failure,
    Running,
}

type NodeId = usize;

type ConditionFn = fn(&LocalPlayerAiSystem, &GameContext, &Blackboard) -> bool;
type ActionFn = fn(&mut LocalPlayerAiSystem, &mut GameContext, &mut Blackboard) -> BtStatus;

enum BtNode {
    Selector {
        children: Vec<NodeId>,
        running_child: usize,
    },
    Sequence {
        children: Vec<NodeId>,
        running_child: usize,
    },
    Condition(ConditionFn),
    Action(ActionFn),
}

struct BehaviorTree {
    nodes: Vec<BtNode>,
    root: NodeId,
}

impl BehaviorTree {
    fn tick(
        &mut self,
        sys: &mut LocalPlayerAiSystem,
        ctx: &mut GameContext,
        bb: &mut Blackboard,
    ) -> BtStatus {
        self.tick_node(self.root, sys, ctx, bb)
    }

    fn tick_node(
        &mut self,
        node_id: NodeId,
        sys: &mut LocalPlayerAiSystem,
        ctx: &mut GameContext,
        bb: &mut Blackboard,
    ) -> BtStatus {
        // 注意：这里不能在递归调用 tick_node 时持有对 `self.nodes[node_id]` 的可变借用。
        // 因此先用不可变借用取出必要数据（children/running_child），递归完再回写状态。
        match self.nodes[node_id] {
            BtNode::Condition(f) => {
                if f(sys, ctx, bb) {
                    BtStatus::Success
                } else {
                    BtStatus::Failure
                }
            }
            BtNode::Action(f) => f(sys, ctx, bb),
            BtNode::Sequence { .. } => {
                let (children, mut running_child) = match &self.nodes[node_id] {
                    BtNode::Sequence {
                        children,
                        running_child,
                    } => (children.clone(), *running_child),
                    _ => unreachable!(),
                };

                while running_child < children.len() {
                    let cid = children[running_child];
                    match self.tick_node(cid, sys, ctx, bb) {
                        BtStatus::Success => running_child += 1,
                        BtStatus::Failure => {
                            if let BtNode::Sequence { running_child: rc, .. } = &mut self.nodes[node_id] {
                                *rc = 0;
                            }
                            return BtStatus::Failure;
                        }
                        BtStatus::Running => {
                            if let BtNode::Sequence { running_child: rc, .. } = &mut self.nodes[node_id] {
                                *rc = running_child;
                            }
                            return BtStatus::Running;
                        }
                    }
                }

                if let BtNode::Sequence { running_child: rc, .. } = &mut self.nodes[node_id] {
                    *rc = 0;
                }
                BtStatus::Success
            }
            BtNode::Selector { .. } => {
                let (children, mut running_child) = match &self.nodes[node_id] {
                    BtNode::Selector {
                        children,
                        running_child,
                    } => (children.clone(), *running_child),
                    _ => unreachable!(),
                };

                while running_child < children.len() {
                    let cid = children[running_child];
                    match self.tick_node(cid, sys, ctx, bb) {
                        BtStatus::Success => {
                            if let BtNode::Selector { running_child: rc, .. } = &mut self.nodes[node_id] {
                                *rc = 0;
                            }
                            return BtStatus::Success;
                        }
                        BtStatus::Failure => running_child += 1,
                        BtStatus::Running => {
                            if let BtNode::Selector { running_child: rc, .. } = &mut self.nodes[node_id] {
                                *rc = running_child;
                            }
                            return BtStatus::Running;
                        }
                    }
                }

                if let BtNode::Selector { running_child: rc, .. } = &mut self.nodes[node_id] {
                    *rc = 0;
                }
                BtStatus::Failure
            }
        }
    }
}

struct Blackboard {
    now: Instant,
    player_entity: Option<hecs::Entity>,
    player_grid: Option<(i32, i32)>,
    player_has_move_goal: bool,
    stuck: bool,

    target_entity: Option<hecs::Entity>,
    target_grid: Option<(i32, i32)>,
}

impl Default for Blackboard {
    fn default() -> Self {
        Self {
            now: Instant::now(),
            player_entity: None,
            player_grid: None,
            player_has_move_goal: false,
            stuck: false,
            target_entity: None,
            target_grid: None,
        }
    }
}

#[derive(ecs_macros::LogicSystem)]
pub struct LocalPlayerAiSystem {
    last_scan: Instant,
    scan_interval: Duration,

    max_acquire_range: i32,

    // 卡住检测：处于运动状态时，若连续 N 帧坐标几乎不变，则视为“被障碍/人墙卡住”
    // 触发一次“换方向/换落点”，避免每帧抖动。
    last_player_pos: Option<(f32, f32)>,
    no_progress_frames: u32,
    stuck_frame_threshold: u32,
    repath_attempt: u32,
    last_melee_goal: Option<(i32, i32)>,

    bt: BehaviorTree,
    bb: Blackboard,

    debug_enabled: bool,
    last_debug_log: Instant,
    debug_interval: Duration,
}

impl Default for LocalPlayerAiSystem {
    fn default() -> Self {
        // BT 结构：
        // Root = Selector(
        //   Sequence(Cond(UserActive), Action(Noop)),
        //   Sequence(Cond(UserSuppressed), Action(Noop)),
        //   Sequence(
        //     Action(UpdateSnapshot),
        //     Action(AcquireTarget),
        //     Selector(
        //       Sequence(Cond(InMeleeRange), Action(StopAndAttack)),
        //       Action(ApproachTarget)
        //     )
        //   )
        // )

        let mut nodes: Vec<BtNode> = Vec::new();
        let mut push = |n: BtNode| {
            nodes.push(n);
            nodes.len() - 1
        };

        let act_update_snapshot = push(BtNode::Action(LocalPlayerAiSystem::bt_act_update_snapshot));
        let act_acquire_target = push(BtNode::Action(LocalPlayerAiSystem::bt_act_acquire_target));
        let cond_in_melee = push(BtNode::Condition(LocalPlayerAiSystem::bt_cond_in_melee_range));
        let act_stop_and_attack = push(BtNode::Action(LocalPlayerAiSystem::bt_act_stop_and_attack));
        let seq_in_melee = push(BtNode::Sequence {
            children: vec![cond_in_melee, act_stop_and_attack],
            running_child: 0,
        });
        let act_approach = push(BtNode::Action(LocalPlayerAiSystem::bt_act_approach_target));
        let sel_engage = push(BtNode::Selector {
            children: vec![seq_in_melee, act_approach],
            running_child: 0,
        });

        let seq_autobattle = push(BtNode::Sequence {
            children: vec![act_update_snapshot, act_acquire_target, sel_engage],
            running_child: 0,
        });

        // 模式互斥：只要启用 AI，本系统始终驱动自动战斗行为。
        let root = seq_autobattle;

        let bt = BehaviorTree { nodes, root };

        let debug_enabled = env::var("CRYSTAL_AI_LOG")
            .ok()
            .map(|v| {
                let v = v.trim().to_ascii_lowercase();
                v == "1" || v == "true" || v == "yes" || v == "on"
            })
            .unwrap_or(false);

        Self {
            last_scan: Instant::now(),
            scan_interval: Duration::from_millis(160),

            max_acquire_range: 26,

            last_player_pos: None,
            no_progress_frames: 0,
            // 约定：连续 50 帧没有位移视为卡住
            stuck_frame_threshold: 50,
            repath_attempt: 0,
            last_melee_goal: None,

            bt,
            bb: Blackboard::default(),

            debug_enabled,
            last_debug_log: Instant::now(),
            debug_interval: Duration::from_millis(1000),
        }
    }
}

impl LocalPlayerAiSystem {
    fn find_local_player_snapshot(
        ctx: &GameContext,
    ) -> Option<(hecs::Entity, (i32, i32), (f32, f32), Option<hecs::Entity>, bool)> {
        for (e, (_local, pos, input)) in ctx
            .world
            .query::<(&LocalPlayer, &Position, &PlayerInput)>()
            .iter()
        {
            let (pgx, pgy) = Coord::world_to_grid(pos.x, pos.y);
            let has_move_goal = input.move_to.is_some();
            return Some((e, (pgx, pgy), (pos.x, pos.y), input.attack_target, has_move_goal));
        }
        None
    }

    fn target_is_valid(ctx: &GameContext, target: hecs::Entity) -> bool {
        if ctx.world.get::<&Monster>(target).is_err() {
            return false;
        }
        if ctx.world.get::<&Position>(target).is_err() {
            return false;
        }
        // 不要锁定已死亡目标（HP=0 的怪）
        if let Ok(hp) = ctx.world.get::<&Health>(target) {
            if hp.current <= 0 {
                return false;
            }
        }
        true
    }

    fn acquire_nearest_monster(
        ctx: &GameContext,
        player_grid: (i32, i32),
        max_range: i32,
    ) -> Option<hecs::Entity> {
        let mut best: Option<(hecs::Entity, i32)> = None;

        for (e, (_m, pos, hp)) in ctx.world.query::<(&Monster, &Position, &Health)>().iter() {
            if hp.current <= 0 {
                continue;
            }
            let (mgx, mgy) = Coord::world_to_grid(pos.x, pos.y);
            let dx = (mgx - player_grid.0).abs();
            let dy = (mgy - player_grid.1).abs();
            let dist = dx.max(dy);

            if dist > max_range {
                continue;
            }

            match best {
                None => best = Some((e, dist)),
                Some((_be, bdist)) if dist < bdist => best = Some((e, dist)),
                _ => {}
            }
        }

        best.map(|(e, _)| e)
    }

    fn occupied_tiles(ctx: &GameContext, local_player: hecs::Entity, target_monster: hecs::Entity) -> HashSet<(i32, i32)> {
        let mut occ = HashSet::new();

        // 其他玩家
        for (e, (_p, pos)) in ctx.world.query::<(&crate::components::Player, &Position)>().iter() {
            if e == local_player {
                continue;
            }
            let g = Coord::world_to_grid(pos.x, pos.y);
            occ.insert(g);
        }

        // 其他怪物（不把目标怪物自身占位加入，避免永远找不到相邻格时过度限制）
        for (e, (_m, pos)) in ctx.world.query::<(&Monster, &Position)>().iter() {
            if e == target_monster {
                continue;
            }
            let g = Coord::world_to_grid(pos.x, pos.y);
            occ.insert(g);
        }

        occ
    }

    fn choose_melee_goal(
        player_grid: (i32, i32),
        monster_grid: (i32, i32),
        occupied: &HashSet<(i32, i32)>,
        attempt: u32,
        avoid_goal: Option<(i32, i32)>,
    ) -> (i32, i32) {
        // 选择怪物周围 8 格中的一个作为落点（更像“绕到旁边砍”），
        // 若被卡住（attempt 变化），会轮换起点，避免原地踏步。
        let mut candidates: Vec<(i32, i32)> = Vec::with_capacity(8);
        for dy in -1..=1 {
            for dx in -1..=1 {
                if dx == 0 && dy == 0 {
                    continue;
                }
                candidates.push((monster_grid.0 + dx, monster_grid.1 + dy));
            }
        }

        // 按与玩家的距离排序（更自然：尽量走最近的那一格）
        candidates.sort_by_key(|(gx, gy)| {
            let dx = (gx - player_grid.0).abs();
            let dy = (gy - player_grid.1).abs();
            dx.max(dy)
        });

        // 轮换起点（简单 deterministic）
        let start = (attempt as usize) % candidates.len();
        for i in 0..candidates.len() {
            let idx = (start + i) % candidates.len();
            let g = candidates[idx];
            if Some(g) == avoid_goal {
                continue;
            }
            if occupied.contains(&g) {
                continue;
            }
            return g;
        }

        // 全被占用：退回“最像贴脸”的那一格（让 PathfindingSystem 的 nearest_walkable_goal 去兜底）
        candidates[0]
    }

    fn choose_escape_goal(
        ctx: &GameContext,
        player_grid: (i32, i32),
        target_grid: Option<(i32, i32)>,
        occupied: &HashSet<(i32, i32)>,
        attempt: u32,
    ) -> Option<(i32, i32)> {
        // 脱困策略：从玩家周围半径 1~R 的环上找一个“可走且未被占用”的格子。
        // 目的不是最短路，而是“侧移/挪开”以摆脱动态阻挡（人墙/怪堆）。
        // 若有 target，则优先选择“更远离目标”的方向（更容易脱离怪堆）。
        const MAX_R: i32 = 6;

        for r in 1_i32..=MAX_R {
            let mut ring: Vec<(i32, i32)> = Vec::new();
            for dy in -r..=r {
                for dx in -r..=r {
                    if dx == 0 && dy == 0 {
                        continue;
                    }
                    if dx.abs().max(dy.abs()) != r {
                        continue;
                    }
                    ring.push((player_grid.0 + dx, player_grid.1 + dy));
                }
            }

            if ring.is_empty() {
                continue;
            }

            // 轮换起点（deterministic），避免一直尝试同一方向。
            let start = (attempt as usize) % ring.len();

            let mut best: Option<((i32, i32), i32)> = None;
            for i in 0..ring.len() {
                let g = ring[(start + i) % ring.len()];
                if occupied.contains(&g) {
                    continue;
                }
                if !Self::is_walkable_grid(ctx, g.0, g.1) {
                    continue;
                }

                let score = if let Some(tg) = target_grid {
                    let dx = (g.0 - tg.0).abs();
                    let dy = (g.1 - tg.1).abs();
                    dx.max(dy)
                } else {
                    // 无目标时：同一环内分数一致，保持“先到先得”。
                    0
                };

                match best {
                    None => best = Some((g, score)),
                    Some((_bg, bs)) if score > bs => best = Some((g, score)),
                    _ => {}
                }
            }

            if let Some((g, _)) = best {
                return Some(g);
            }
        }

        None
    }

    // ===== Behavior Tree: Conditions =====

    fn bt_cond_in_melee_range(&self, _ctx: &GameContext, bb: &Blackboard) -> bool {
        let Some(pg) = bb.player_grid else {
            return false;
        };
        let Some(tg) = bb.target_grid else {
            return false;
        };
        let dx = (tg.0 - pg.0).abs();
        let dy = (tg.1 - pg.1).abs();
        dx.max(dy) <= 1
    }

    // ===== Behavior Tree: Actions =====

    fn bt_act_update_snapshot(&mut self, ctx: &mut GameContext, bb: &mut Blackboard) -> BtStatus {
        let Some((player_entity, player_grid, player_pos, current_target, has_move_goal)) =
            Self::find_local_player_snapshot(ctx)
        else {
            bb.player_entity = None;
            bb.player_grid = None;
            bb.player_has_move_goal = false;
            bb.stuck = false;
            bb.target_entity = None;
            bb.target_grid = None;

            if self.debug_enabled && bb.now.duration_since(self.last_debug_log) >= self.debug_interval {
                self.last_debug_log = bb.now;
                eprintln!("[AI] no local player snapshot (need LocalPlayer+Position+PlayerInput)");
            }
            return BtStatus::Failure;
        };

        bb.player_entity = Some(player_entity);
        bb.player_grid = Some(player_grid);
        bb.player_has_move_goal = has_move_goal;

        // 卡住检测（只在“有移动目标”时启用）：连续 N 帧位置几乎不变 -> 触发一次 stuck
        // 注意：用世界坐标（像素）而不是 grid，避免平滑移动时误判。
        const EPS_PX: f32 = 0.01;
        let mut stuck_event = false;

        if !has_move_goal {
            self.no_progress_frames = 0;
        } else if let Some((lx, ly)) = self.last_player_pos {
            let dx = (player_pos.0 - lx).abs();
            let dy = (player_pos.1 - ly).abs();
            let progressed = dx > EPS_PX || dy > EPS_PX;

            if progressed {
                self.no_progress_frames = 0;
            } else {
                self.no_progress_frames = self.no_progress_frames.saturating_add(1);
                if self.no_progress_frames >= self.stuck_frame_threshold {
                    stuck_event = true;
                    // 触发一次后归零，避免每帧都 repath 导致抖动
                    self.no_progress_frames = 0;
                }
            }
        }

        self.last_player_pos = Some(player_pos);
        bb.stuck = stuck_event;

        // 目标保持（先不做重搜，重搜在 AcquireTarget）
        bb.target_entity = current_target;
        bb.target_grid = None;
        if let Some(t) = bb.target_entity {
            if let Ok(pos) = ctx.world.get::<&Position>(t) {
                bb.target_grid = Some(Coord::world_to_grid(pos.x, pos.y));
            }
        }

        BtStatus::Success
    }

    fn bt_act_acquire_target(&mut self, ctx: &mut GameContext, bb: &mut Blackboard) -> BtStatus {
        let Some(player_entity) = bb.player_entity else {
            return BtStatus::Failure;
        };
        let Some(player_grid) = bb.player_grid else {
            return BtStatus::Failure;
        };

        let prev_target = bb.target_entity;
        let mut did_scan = false;

        // 校验现有目标
        let mut target = bb
            .target_entity
            .filter(|t| Self::target_is_valid(ctx, *t));
        // 关键修复：如果旧目标失效（死亡/消失），立即触发重搜，不等待节流窗口。
        // 这能避免"打一会儿就停了"的问题：怪物死亡后立即找新怪，不等 160ms。
        let target_lost = prev_target.is_some() && target.is_none();
        if target_lost {
            self.last_scan = bb.now - self.scan_interval;
        }
        // 若玩家已经离当前目标太远，则丢弃目标并按“玩家当前坐标周边”重新找怪。
        // 这能避免 AI 一直追着出生点附近的旧目标不放。
        if let Some(t) = target {
            if let Ok(pos) = ctx.world.get::<&Position>(t) {
                let (mgx, mgy) = Coord::world_to_grid(pos.x, pos.y);
                let dx = (mgx - player_grid.0).abs();
                let dy = (mgy - player_grid.1).abs();
                let dist = dx.max(dy);
                if dist > self.max_acquire_range {
                    target = None;
                    // 触发“立即重搜”（不必再等节流窗口）
                    self.last_scan = bb.now - self.scan_interval;
                }
            }
        }

        // 节流重搜
        // 关键修复：即使已有旧目标，也要定期按“玩家当前坐标周边”重新评估最近怪。
        // 否则玩家移动到别处后，AI 可能会一直追着出生点附近的旧目标不放。
        if bb.now.duration_since(self.last_scan) >= self.scan_interval {
            self.last_scan = bb.now;
            did_scan = true;
            if let Some(candidate) = Self::acquire_nearest_monster(ctx, player_grid, self.max_acquire_range) {
                target = Some(candidate);
            }
        }

        bb.target_entity = target;
        bb.target_grid = None;

        if let Some(t) = bb.target_entity {
            if let Ok(pos) = ctx.world.get::<&Position>(t) {
                bb.target_grid = Some(Coord::world_to_grid(pos.x, pos.y));
            }

            // 维持追砍目标
            if let Ok(mut input) = ctx.world.get::<&mut PlayerInput>(player_entity) {
                input.attack_target = Some(t);
            }

            if self.debug_enabled && bb.now.duration_since(self.last_debug_log) >= self.debug_interval {
                self.last_debug_log = bb.now;
                let changed = prev_target != bb.target_entity;
                let tg = bb.target_grid;
                eprintln!(
                    "[AI] pg={:?} target={:?} tg={:?} scan={} changed={} has_move_goal={} stuck={} repath={} max_range={} no_progress_frames={}",
                    Some(player_grid),
                    bb.target_entity,
                    tg,
                    did_scan,
                    changed,
                    bb.player_has_move_goal,
                    bb.stuck,
                    self.repath_attempt,
                    self.max_acquire_range,
                    self.no_progress_frames,
                );
            }

            return BtStatus::Success;
        }

        // 没怪：只清攻击目标，不打断已有移动
        if let Ok(mut input) = ctx.world.get::<&mut PlayerInput>(player_entity) {
            input.attack_target = None;
        }

        if self.debug_enabled && bb.now.duration_since(self.last_debug_log) >= self.debug_interval {
            self.last_debug_log = bb.now;
            eprintln!(
                "[AI] pg={:?} target=None scan={} has_move_goal={} stuck={} repath={} max_range={} no_progress_frames={}",
                Some(player_grid),
                did_scan,
                bb.player_has_move_goal,
                bb.stuck,
                self.repath_attempt,
                self.max_acquire_range,
                self.no_progress_frames,
            );
        }
        BtStatus::Failure
    }

    fn bt_act_stop_and_attack(&mut self, ctx: &mut GameContext, bb: &mut Blackboard) -> BtStatus {
        let Some(player_entity) = bb.player_entity else {
            return BtStatus::Failure;
        };
        let Some(target_entity) = bb.target_entity else {
            // 目标丢失：交给 AcquireTarget 处理
            if let Ok(mut input) = ctx.world.get::<&mut PlayerInput>(player_entity) {
                input.attack_target = None;
            }
            return BtStatus::Failure;
        };

        if let Ok(mut input) = ctx.world.get::<&mut PlayerInput>(player_entity) {
            input.attack_target = Some(target_entity);
            input.move_to = None;
            input.movement_mode = MovementMode::None;
            input.run = false;
        }

        BtStatus::Success
    }

    fn bt_act_approach_target(&mut self, ctx: &mut GameContext, bb: &mut Blackboard) -> BtStatus {
        let Some(player_entity) = bb.player_entity else {
            return BtStatus::Failure;
        };
        let Some(player_grid) = bb.player_grid else {
            return BtStatus::Failure;
        };
        let Some(target_entity) = bb.target_entity else {
            return BtStatus::Failure;
        };

        // 读取目标位置
        let Ok(target_pos) = ctx.world.get::<&Position>(target_entity) else {
            return BtStatus::Failure;
        };
        let target_grid = Coord::world_to_grid(target_pos.x, target_pos.y);
        bb.target_grid = Some(target_grid);

        // 占位集合（动态阻挡）：用于脱困和近战落点筛选。
        let mut occupied = Self::occupied_tiles(ctx, player_entity, target_entity);
        // 把目标怪物自身格子也视为“不可落脚”，避免脱困时踩进怪身上。
        occupied.insert(target_grid);

        // 卡住：先脱困一步（侧移/挪开），再继续追砍。
        // 这比“只换怪物周围落点”更能处理动态人墙阻挡。
        if bb.stuck {
            self.repath_attempt = self.repath_attempt.wrapping_add(1);
            if let Some((egx, egy)) =
                Self::choose_escape_goal(ctx, player_grid, Some(target_grid), &occupied, self.repath_attempt)
            {
                let (ewx, ewy) = Coord::grid_to_world_center(egx, egy);
                if let Ok(mut input) = ctx.world.get::<&mut PlayerInput>(player_entity) {
                    input.attack_target = Some(target_entity);
                    input.move_to = Some((ewx, ewy));
                    input.movement_mode = MovementMode::Pathfinding;
                    input.run = true;
                }

                if self.debug_enabled && bb.now.duration_since(self.last_debug_log) >= self.debug_interval {
                    self.last_debug_log = bb.now;
                    eprintln!("[AI] stuck: escape_step to grid=({},{})", egx, egy);
                }

                return BtStatus::Running;
            }

            // 依然无法找到脱困落点：硬重置一次（清目标/清移动/清路径），避免一直原地卡死。
            if let Ok(mut input) = ctx.world.get::<&mut PlayerInput>(player_entity) {
                input.attack_target = None;
                input.move_to = None;
                input.movement_mode = MovementMode::None;
                input.run = false;
            }
            if let Ok(mut path) = ctx.world.get::<&mut crate::components::movement::Path>(player_entity) {
                path.clear();
            }
            self.last_melee_goal = None;
            // 下帧强制重搜目标（不等节流窗口）
            self.last_scan = bb.now - self.scan_interval;

            if self.debug_enabled && bb.now.duration_since(self.last_debug_log) >= self.debug_interval {
                self.last_debug_log = bb.now;
                eprintln!("[AI] stuck: no escape goal; reset target+move and force rescan");
            }

            return BtStatus::Failure;
        }

        // 避障/防卡：优先选择怪物周围的“可落脚格子”。
        // 如果检测到卡住，会轮换目标格子触发重新寻路。
        let avoid_goal = if bb.stuck {
            self.repath_attempt = self.repath_attempt.wrapping_add(1);
            self.last_melee_goal
        } else {
            None
        };

        // 关键修复：近战落脚点必须是“可走格子”。
        // 否则 Pathfinding 会一直想走进墙里 -> 表现为“原地跑路/卡住”。
        let mut chosen: Option<(i32, i32)> = None;
        for j in 0..8_u32 {
            let (agx, agy) = Self::choose_melee_goal(
                player_grid,
                target_grid,
                &occupied,
                self.repath_attempt.wrapping_add(j),
                avoid_goal,
            );
            if Self::is_walkable_grid(ctx, agx, agy) {
                chosen = Some((agx, agy));
                break;
            }
        }

        let Some((agx, agy)) = chosen else {
            // 找不到可落脚点：停下，下一帧再重试/重搜
            if let Ok(mut input) = ctx.world.get::<&mut PlayerInput>(player_entity) {
                input.move_to = None;
                input.movement_mode = MovementMode::None;
                input.run = false;
            }
            return BtStatus::Failure;
        };

        self.last_melee_goal = Some((agx, agy));
        let (awx, awy) = Coord::grid_to_world_center(agx, agy);

        if let Ok(mut input) = ctx.world.get::<&mut PlayerInput>(player_entity) {
            input.attack_target = Some(target_entity);
            input.move_to = Some((awx, awy));
            input.movement_mode = MovementMode::Pathfinding;
            // 追怪接近：默认跑步（更符合“追砍”节奏）
            input.run = true;
        }

        BtStatus::Running
    }

    fn is_walkable_grid(ctx: &GameContext, gx: i32, gy: i32) -> bool {
        if gx < 0 || gy < 0 {
            return false;
        }
        let mut q = ctx.world.query::<&crate::components::MapData>();
        let Some((_, map)) = q.iter().next() else {
            // 没地图数据时退化为“允许”，避免 AI 完全停摆
            return true;
        };
        if gx >= map.width || gy >= map.height {
            return false;
        }
        map.cells
            .get(gx as usize)
            .and_then(|col| col.get(gy as usize))
            .map(|c| c.is_walkable())
            .unwrap_or(false)
    }
}

impl LogicSystem for LocalPlayerAiSystem {
    fn update(&mut self, ctx: &mut GameContext, _dt: f32) -> GameResult {
        // ===== AI 开关（F8） =====
        // 说明：放在最前面，确保“关闭后还能再打开”。
        if !ctx.input_blocked && ctx.input().key_pressed(KeyCode::F8) {
            ctx.session.local_player_ai_enabled = !ctx.session.local_player_ai_enabled;

            // 模式切换：无论开/关，都清理上一种控制模式残留的意图，避免“切了还在走/追砍”。
            for (e, (_local, input, path)) in ctx
                .world
                .query_mut::<(
                    &LocalPlayer,
                    &mut PlayerInput,
                    &mut crate::components::movement::Path,
                )>()
                .into_iter()
            {
                input.attack_target = None;
                input.move_to = None;
                input.movement_mode = MovementMode::None;
                input.run = false;
                path.clear();

                // 同时停掉速度/动作，避免“关了 AI 还在原地跑步动画”。
                if let Ok(mut mv) = ctx.world.get::<&mut crate::components::MovementVelocity>(e) {
                    mv.stop();
                }
                if let Ok(mut m) = ctx.world.get::<&mut crate::components::Movement>(e) {
                    m.set_state(crate::components::MovementState::Idle);
                }
                if let Ok(mut p) = ctx.world.get::<&mut crate::components::Player>(e) {
                    p.action = crate::components::PlayerAction::Stand;
                }
                let _ = ctx.world.remove_one::<crate::components::AttackState>(e);
                break;
            }

            // 给玩家一个可见反馈（写到聊天系统提示）
            if let Some((_e, ui)) = ctx.world.query::<&UiState>().iter().next() {
                let msg = if ctx.session.local_player_ai_enabled {
                    "[AI] 已开启（F8 切换）".to_string()
                } else {
                    "[AI] 已关闭（F8 切换）".to_string()
                };
                ui.borrow_mut().pending_commands.push(UiCommand::PushSystemChatLine(msg));
            }
        }

        // 未开启：本帧不写入 PlayerInput
        if !ctx.session.local_player_ai_enabled {
            return Ok(());
        }

        // UI 正在强占输入（例如对话框/输入框）时，暂停挂机，避免“边点 UI 边乱跑”。
        if ctx.input_blocked {
            return Ok(());
        }

        // 为避免 `&mut self` 与 `&mut self.bb` 的同时借用冲突，将 bt/bb 临时移出。
        let mut bt = std::mem::replace(
            &mut self.bt,
            BehaviorTree {
                nodes: Vec::new(),
                root: 0,
            },
        );
        let mut bb = std::mem::take(&mut self.bb);
        bb.now = Instant::now();

        // 行为树驱动：默认“让位用户输入”，否则自动战斗。
        let _ = bt.tick(self, ctx, &mut bb);

        self.bb = bb;
        self.bt = bt;

        Ok(())
    }
}
