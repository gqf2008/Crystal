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
use std::time::{Duration, Instant};

use macroquad::prelude::KeyCode;

use crate::{
    components::{LocalPlayer, Monster, PlayerInput, Position, MovementMode},
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

    // 卡住检测：位置长时间不变且存在移动目标，则视为“被障碍/人墙卡住”
    last_player_grid: Option<(i32, i32)>,
    last_progress: Instant,
    stuck_timeout: Duration,
    repath_attempt: u32,
    last_melee_goal: Option<(i32, i32)>,

    bt: BehaviorTree,
    bb: Blackboard,
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

        Self {
            last_scan: Instant::now(),
            scan_interval: Duration::from_millis(160),

            max_acquire_range: 26,

            last_player_grid: None,
            last_progress: Instant::now(),
            stuck_timeout: Duration::from_millis(850),
            repath_attempt: 0,
            last_melee_goal: None,

            bt,
            bb: Blackboard::default(),
        }
    }
}

impl LocalPlayerAiSystem {
    fn find_local_player_snapshot(
        ctx: &GameContext,
    ) -> Option<(hecs::Entity, (i32, i32), Option<hecs::Entity>, bool)> {
        for (e, (_local, pos, input)) in ctx
            .world
            .query::<(&LocalPlayer, &Position, &PlayerInput)>()
            .iter()
        {
            let (pgx, pgy) = Coord::world_to_grid(pos.x, pos.y);
            let has_move_goal = input.move_to.is_some();
            return Some((e, (pgx, pgy), input.attack_target, has_move_goal));
        }
        None
    }

    fn target_is_valid(ctx: &GameContext, target: hecs::Entity) -> bool {
        ctx.world.get::<&Monster>(target).is_ok() && ctx.world.get::<&Position>(target).is_ok()
    }

    fn acquire_nearest_monster(
        ctx: &GameContext,
        player_grid: (i32, i32),
        max_range: i32,
    ) -> Option<hecs::Entity> {
        let mut best: Option<(hecs::Entity, i32)> = None;

        for (e, (_m, pos)) in ctx.world.query::<(&Monster, &Position)>().iter() {
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
        let Some((player_entity, player_grid, current_target, has_move_goal)) = Self::find_local_player_snapshot(ctx)
        else {
            bb.player_entity = None;
            bb.player_grid = None;
            bb.player_has_move_goal = false;
            bb.stuck = false;
            bb.target_entity = None;
            bb.target_grid = None;
            return BtStatus::Failure;
        };

        bb.player_entity = Some(player_entity);
        bb.player_grid = Some(player_grid);
        bb.player_has_move_goal = has_move_goal;

        // 卡住检测（只在“有移动目标”时启用）
        if self.last_player_grid != Some(player_grid) {
            self.last_player_grid = Some(player_grid);
            self.last_progress = bb.now;
            bb.stuck = false;
        } else if has_move_goal && bb.now.duration_since(self.last_progress) >= self.stuck_timeout {
            bb.stuck = true;
        } else {
            bb.stuck = false;
        }

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

        // 校验现有目标
        let mut target = bb
            .target_entity
            .filter(|t| Self::target_is_valid(ctx, *t));

        // 节流重搜
        if target.is_none() && bb.now.duration_since(self.last_scan) >= self.scan_interval {
            self.last_scan = bb.now;
            target = Self::acquire_nearest_monster(ctx, player_grid, self.max_acquire_range);
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

            return BtStatus::Success;
        }

        // 没怪：只清攻击目标，不打断已有移动
        if let Ok(mut input) = ctx.world.get::<&mut PlayerInput>(player_entity) {
            input.attack_target = None;
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

        // 避障/防卡：优先选择怪物周围的“可落脚格子”。
        // 如果检测到卡住，会轮换目标格子触发重新寻路。
        let avoid_goal = if bb.stuck {
            self.repath_attempt = self.repath_attempt.wrapping_add(1);
            self.last_melee_goal
        } else {
            None
        };
        let occupied = Self::occupied_tiles(ctx, player_entity, target_entity);
        let (agx, agy) = Self::choose_melee_goal(
            player_grid,
            target_grid,
            &occupied,
            self.repath_attempt,
            avoid_goal,
        );
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
}

impl LogicSystem for LocalPlayerAiSystem {
    fn update(&mut self, ctx: &mut GameContext, _dt: f32) -> GameResult {
        // ===== AI 开关（F8） =====
        // 说明：放在最前面，确保“关闭后还能再打开”。
        if !ctx.input_blocked && ctx.input().key_pressed(KeyCode::F8) {
            ctx.session.local_player_ai_enabled = !ctx.session.local_player_ai_enabled;

            // 模式切换：无论开/关，都清理上一种控制模式残留的意图，避免“切了还在走/追砍”。
            for (_e, (_local, input, path)) in ctx
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
