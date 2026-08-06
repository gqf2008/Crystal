// ============================================================================
// Pathfinding System - 寻路系统
// Priority: 350 (在 PlayerControlSystem 之后, MovementSystem 之前)
// ============================================================================
//
// **职责**:
// - 读取 PlayerInput.move_to - 将输入转换为路径
// - 根据 use_pathfinding 决定是否使用 A* 寻路
// - 将路径写入 Path 组件供 MovementSystem 使用
//
// **数据流**:
// ```
// PlayerInput.move_to (世界坐标)
//     ↓ (坐标转换)
// 格子坐标
//     ↓ (寻路 or 直线)
// Path.waypoints
//     ↓
// MovementSystem 读取并执行移动
// ```
//
// ============================================================================

use crate::game::{Coord, GameContext, GameResult, PathFinder};
use crate::{
    components::{LocalPlayer, MapData, MovementVelocity, Path, Player, PlayerInput, Position},
    systems::LogicSystem,
};
use std::collections::HashSet;

pub struct PathfindingSystem;

impl PathfindingSystem {
    pub fn new() -> Self {
        Self
    }

    /// 世界坐标 → 格子坐标
    fn world_to_grid(world_x: f32, world_y: f32) -> (i32, i32) {
        Coord::world_to_grid(world_x, world_y)
    }

    /// 将逐格 A* 路径压缩为“跑步两格一步”的 waypoint 列表。
    ///
    /// 原版 Crystal 协议语义：Run 包一次推进 2 格（直线/斜线）。
    /// Macroquad 本地移动若仍按 1 格 waypoint 前进，只会导致：
    /// - 本地在跑，但网络侧只能发 Walk（否则服务端会推进 2 格产生漂移）
    /// - 其它玩家视角/服务端节奏异常
    ///
    /// 规则：只有当连续两步方向一致时，才跳过中间格子（2 格一步）；否则保持 1 格。
    fn compress_waypoints_for_run(
        current_grid: (i32, i32),
        waypoints: Vec<(i32, i32)>,
    ) -> Vec<(i32, i32)> {
        let mut out: Vec<(i32, i32)> = Vec::with_capacity(waypoints.len());
        let mut prev = current_grid;
        let mut i = 0usize;

        while i < waypoints.len() {
            let next1 = waypoints[i];

            if i + 1 < waypoints.len() {
                let next2 = waypoints[i + 1];

                let dx1 = (next1.0 - prev.0).clamp(-1, 1);
                let dy1 = (next1.1 - prev.1).clamp(-1, 1);
                let dx2 = (next2.0 - next1.0).clamp(-1, 1);
                let dy2 = (next2.1 - next1.1).clamp(-1, 1);

                let dir_ok = (dx1 != 0 || dy1 != 0) && (dx1, dy1) == (dx2, dy2);
                let two_step_ok = (next2.0 - prev.0, next2.1 - prev.1) == (dx1 * 2, dy1 * 2);

                if dir_ok && two_step_ok {
                    out.push(next2);
                    prev = next2;
                    i += 2;
                    continue;
                }
            }

            out.push(next1);
            prev = next1;
            i += 1;
        }

        out
    }

    /// 使用 A* 算法计算路径
    fn calculate_path(
        map_data: &MapData,
        start_grid: (i32, i32),
        target_grid: (i32, i32),
    ) -> Option<Vec<(i32, i32)>> {
        let start = (start_grid.0 as usize, start_grid.1 as usize);
        let goal = (target_grid.0 as usize, target_grid.1 as usize);

        // 创建地图障碍检测闭包
        let cells = map_data.cells.clone();
        let width = map_data.width as usize;
        let height = map_data.height as usize;

        let is_blocking = move |x: usize, y: usize| -> bool {
            if x >= width || y >= height {
                return true; // 地图外视为阻挡
            }
            // ✅ 关键修正：地图数据结构是 cells[x][y]，不是 cells[y][x]！
            // outer vec 有 Width 个元素，inner vec 有 Height 个元素
            if x >= cells.len() || y >= cells[x].len() {
                return true;
            }
            !cells[x][y].is_walkable() // true = 阻挡
        };

        // 创建寻路器并计算路径
        let pathfinder = PathFinder::new(width, height, is_blocking);

        pathfinder.find_path(start, goal).map(|path| {
            path.into_iter()
                .map(|p| (p.0 as i32, p.1 as i32)) // 转换 (usize, usize) -> (i32, i32)
                .collect()
        })
    }

    /// A*（带动态阻挡）
    ///
    /// 用途：本地挂机 AI 在怪堆/人墙场景下，如果只按静态地图寻路，
    /// 往往会每帧撞人/撞怪导致“原地跑”。这里把实体占位当作临时阻挡格，让 A* 能绕开。
    fn calculate_path_with_blocked(
        map_data: &MapData,
        start_grid: (i32, i32),
        target_grid: (i32, i32),
        blocked: HashSet<(usize, usize)>,
    ) -> Option<Vec<(i32, i32)>> {
        let start = (start_grid.0 as usize, start_grid.1 as usize);
        let goal = (target_grid.0 as usize, target_grid.1 as usize);

        let cells = map_data.cells.clone();
        let width = map_data.width as usize;
        let height = map_data.height as usize;

        let is_blocking = move |x: usize, y: usize| -> bool {
            if x >= width || y >= height {
                return true;
            }
            if x >= cells.len() || y >= cells[x].len() {
                return true;
            }
            if blocked.contains(&(x, y)) {
                return true;
            }
            !cells[x][y].is_walkable()
        };

        let pathfinder = PathFinder::new(width, height, is_blocking);
        pathfinder
            .find_path(start, goal)
            .map(|path| path.into_iter().map(|p| (p.0 as i32, p.1 as i32)).collect())
    }

    fn is_grid_walkable(map_data: &MapData, gx: i32, gy: i32) -> bool {
        if gx < 0 || gy < 0 || gx >= map_data.width || gy >= map_data.height {
            return false;
        }
        let x = gx as usize;
        let y = gy as usize;
        if x >= map_data.cells.len() || y >= map_data.cells[x].len() {
            return false;
        }
        map_data.cells[x][y].is_walkable()
    }

    fn nearest_walkable_goal(
        map_data: &MapData,
        target: (i32, i32),
        max_radius: i32,
    ) -> Option<(i32, i32)> {
        if Self::is_grid_walkable(map_data, target.0, target.1) {
            return Some(target);
        }

        for r in 1..=max_radius {
            for dx in -r..=r {
                // 上下边
                for dy in [-r, r] {
                    let gx = target.0 + dx;
                    let gy = target.1 + dy;
                    if Self::is_grid_walkable(map_data, gx, gy) {
                        return Some((gx, gy));
                    }
                }
            }
            for dy in (-r + 1)..=(r - 1) {
                // 左右边
                for dx in [-r, r] {
                    let gx = target.0 + dx;
                    let gy = target.1 + dy;
                    if Self::is_grid_walkable(map_data, gx, gy) {
                        return Some((gx, gy));
                    }
                }
            }
        }

        None
    }
}

impl LogicSystem for PathfindingSystem {
    fn update(&mut self, ctx: &mut GameContext, _dt: f32) -> GameResult {
        // 获取地图数据(用于寻路)
        let map_data = ctx
            .world
            .query_mut::<&MapData>()
            .into_iter()
            .next()
            .cloned();

        // 动态占位（仅用于挂机 AI 的寻路避障）
        // 注意：这里必须在 query_mut 循环之前收集，避免 hecs 的可变借用冲突。
        let dynamic_occupied: Option<HashSet<(usize, usize)>> =
            if ctx.session.local_player_ai_enabled {
                let mut occ: HashSet<(usize, usize)> = HashSet::new();

                for (_, pos) in ctx.world.query::<(&Player, &Position)>().iter() {
                    let (gx, gy) = Coord::world_to_grid(pos.x, pos.y);
                    if gx >= 0 && gy >= 0 {
                        occ.insert((gx as usize, gy as usize));
                    }
                }
                for (_, pos) in ctx
                    .world
                    .query::<(&crate::components::Monster, &Position)>()
                    .iter()
                {
                    let (gx, gy) = Coord::world_to_grid(pos.x, pos.y);
                    if gx >= 0 && gy >= 0 {
                        occ.insert((gx as usize, gy as usize));
                    }
                }

                Some(occ)
            } else {
                None
            };

        // 处理本地玩家的移动输入
        for (player_input, position, path, player, _local, velocity) in ctx.world.query_mut::<(
            &mut PlayerInput,
            &Position,
            &mut Path,
            &Player,
            &LocalPlayer,
            &mut MovementVelocity,
        )>() {
            // 检查是否有移动指令
            if let Some((target_x, target_y)) = player_input.move_to {
                let current_grid = Self::world_to_grid(position.x, position.y);
                let target_grid = Self::world_to_grid(target_x, target_y);

                tracing::trace!(
                    "🗺️ PathfindingSystem: current=({},{}), target=({},{}), mode={:?}",
                    current_grid.0,
                    current_grid.1,
                    target_grid.0,
                    target_grid.1,
                    player_input.movement_mode
                );

                // 检查目标是否与当前位置不同
                if current_grid != target_grid {
                    // 检查是否需要更新路径 - 避免每帧重复设置导致卡顿
                    let needs_update = if !path.waypoints.is_empty() {
                        // 检查路径的最终目标是否改变了（不是当前waypoint）
                        let final_target = path.waypoints[path.waypoints.len() - 1];
                        final_target != target_grid
                    } else {
                        // 没有路径,需要设置
                        true
                    };

                    // 🎯 DirectFollow模式: 不使用Path，每帧直接计算velocity方向
                    use crate::components::MovementMode;

                    if player_input.movement_mode == MovementMode::DirectFollow {
                        // 🚀 平滑跟随: 直接设置velocity朝向目标，完全不用Path
                        let dx = target_x - position.x;
                        let dy = target_y - position.y;
                        let distance = (dx * dx + dy * dy).sqrt();

                        if distance > 5.0 {
                            // 距离目标超过5像素才移动
                            // 🛡️ DirectFollow 模式需要预检查移动方向是否有障碍物
                            // 归一化方向向量
                            let dir_x = dx / distance;
                            let dir_y = dy / distance;

                            // 预测移动一小段距离后的位置（比如10像素）
                            let check_distance = 10.0;
                            let next_x = position.x + dir_x * check_distance;
                            let next_y = position.y + dir_y * check_distance;
                            // ✅ 使用统一坐标换算，避免手写 /48 /32 带来的边界误差
                            let (next_grid_x, next_grid_y) = Coord::world_to_grid(next_x, next_y);

                            // 检查是否有障碍物（需要地图数据）
                            let has_obstacle = if let Some(ref map) = map_data {
                                // 检查目标格子是否在地图范围内
                                let in_bounds = next_grid_x >= 0
                                    && next_grid_y >= 0
                                    && next_grid_x < map.width
                                    && next_grid_y < map.height;

                                if !in_bounds {
                                    true // 边界外视为障碍物
                                } else {
                                    let x = next_grid_x as usize;
                                    let y = next_grid_y as usize;
                                    if x >= map.cells.len() || y >= map.cells[x].len() {
                                        true
                                    } else {
                                        !map.cells[x][y].is_walkable()
                                    }
                                }
                            } else {
                                false // 没有地图数据，不阻挡
                            };

                            if !has_obstacle {
                                // 前方没有障碍物，可以移动
                                // 🎬 根据 Player.action 设置速度
                                use crate::components::PlayerAction;
                                let speed = if player.action == PlayerAction::Run {
                                    velocity.run_speed
                                } else {
                                    velocity.walk_speed
                                };

                                // 直接设置velocity，MovementSystem会直接用它更新position
                                velocity.x = dir_x * speed;
                                velocity.y = dir_y * speed;
                                velocity.max_speed = speed;
                            } else {
                                // 前方有障碍物，停止移动
                                velocity.stop();
                            }

                            // ❌ 不设置Path！让MovementSystem直接用velocity更新
                            path.clear(); // 确保Path不干扰
                        } else {
                            velocity.stop();
                        }
                        continue; // 跳过后续的Path逻辑
                    }

                    if needs_update {
                        match player_input.movement_mode {
                            MovementMode::Pathfinding => {
                                // 寻路模式 (双击): 使用A*算法
                                tracing::debug!(
                                    "🔍 寻路: ({}, {}) -> ({}, {})",
                                    current_grid.0,
                                    current_grid.1,
                                    target_grid.0,
                                    target_grid.1
                                );

                                // 🎯 使用 A* 算法计算完整路径（考虑障碍物）
                                if let Some(ref map_data) = map_data {
                                    // 目标不可走时，先找一个最近可走的落点
                                    let goal =
                                        Self::nearest_walkable_goal(map_data, target_grid, 8)
                                            .unwrap_or(target_grid);

                                    let use_dynamic_blocked = ctx.session.local_player_ai_enabled;
                                    let mut blocked = dynamic_occupied.clone().unwrap_or_default();

                                    // 不要把自己当前格子视为阻挡，否则 A* 会直接失败。
                                    if current_grid.0 >= 0 && current_grid.1 >= 0 {
                                        blocked.remove(&(
                                            current_grid.0 as usize,
                                            current_grid.1 as usize,
                                        ));
                                    }

                                    // 若目标格子被占用（动态变化），这里保持“按占用阻挡”处理，
                                    // 让上层 AI/脱困逻辑换落点；避免强行走进人/怪格子。
                                    let path_res = if use_dynamic_blocked {
                                        Self::calculate_path_with_blocked(
                                            map_data,
                                            current_grid,
                                            goal,
                                            blocked,
                                        )
                                    } else {
                                        Self::calculate_path(map_data, current_grid, goal)
                                    };

                                    match path_res {
                                        Some(full_path) => {
                                            tracing::debug!(
                                                "✅ A* 找到路径，共 {} 个格子",
                                                full_path.len()
                                            );
                                            if full_path.len() <= 10 {
                                                tracing::debug!("完整路径: {:?}", full_path);
                                            } else {
                                                tracing::debug!(
                                                    "路径开始: {:?} ...",
                                                    &full_path[..5]
                                                );
                                                tracing::debug!(
                                                    "路径结束: ... {:?}",
                                                    &full_path[full_path.len() - 5..]
                                                );
                                            }
                                            // A* 通常会把起点也包含在路径里。
                                            // 若直接把起点作为第一个 waypoint，MovementSystem 会先把角色“拉回”到当前格子中心，
                                            // 双击小地图时体验很明显（看起来像被回拽/瞬移）。
                                            // 这里移除起点，直接朝下一个格子走。
                                            let mut waypoints = full_path;
                                            if waypoints.first().copied() == Some(current_grid) {
                                                waypoints.remove(0);
                                            }

                                            // 跑步：将逐格路径压缩为“每步两格”（方向一致时才跳过中间格）。
                                            // 这样本地 MovementSystem 与网络 Run(2格/包) 语义一致。
                                            use crate::components::PlayerAction;
                                            if player.action == PlayerAction::Run
                                                && waypoints.len() >= 2
                                            {
                                                waypoints = Self::compress_waypoints_for_run(
                                                    current_grid,
                                                    waypoints,
                                                );
                                            }

                                            if waypoints.is_empty() {
                                                // 已经在目标格子（或目标非常近），直接视为完成移动。
                                                path.clear();
                                                velocity.stop();
                                                player_input.move_to = None;
                                                player_input.movement_mode = MovementMode::None;
                                            } else {
                                                path.set_path(waypoints);

                                                // ✅ 关键修复：初始 velocity 必须指向“第一个 waypoint 的格子中心”，
                                                // 而不是指向原始 move_to 的世界坐标。
                                                // 否则当点击点落在不可走格/墙体边缘时，会出现：
                                                // A* 算出可走路径 → 但 velocity 仍然顶向不可走点 → CollisionSystem 每帧清 path → 下一帧重复算路。
                                                if let Some((wx_gx, wx_gy)) =
                                                    path.current_waypoint()
                                                {
                                                    let (wx, wy) =
                                                        Coord::grid_to_world_center(wx_gx, wx_gy);
                                                    use crate::components::PlayerAction;
                                                    let speed =
                                                        if player.action == PlayerAction::Run {
                                                            velocity.run_speed
                                                        } else {
                                                            velocity.walk_speed
                                                        };

                                                    let dx = wx - position.x;
                                                    let dy = wy - position.y;
                                                    let distance = (dx * dx + dy * dy).sqrt();
                                                    if distance > 0.1 {
                                                        velocity.x = (dx / distance) * speed;
                                                        velocity.y = (dy / distance) * speed;
                                                    } else {
                                                        velocity.stop();
                                                    }
                                                }
                                            }
                                        }
                                        None => {
                                            tracing::warn!("❌ A* 找不到路径（含避障），停止移动");
                                            path.clear();
                                            velocity.stop();
                                            // 关键：若处于“本地挂机 AI 追砍”场景，不要清空 move_to。
                                            // 否则 AI 的 stuck 检测无法触发，表现为“跑跑就卡死”。
                                            let ai_should_retry =
                                                ctx.session.local_player_ai_enabled
                                                    && player_input.attack_target.is_some();
                                            if !ai_should_retry {
                                                player_input.move_to = None;
                                                player_input.movement_mode = MovementMode::None;
                                            }
                                            continue;
                                        }
                                    }
                                } else {
                                    tracing::warn!("⚠️ 地图数据不存在，停止寻路");
                                    path.clear();
                                    velocity.stop();
                                    let ai_should_retry = ctx.session.local_player_ai_enabled
                                        && player_input.attack_target.is_some();
                                    if !ai_should_retry {
                                        player_input.move_to = None;
                                        player_input.movement_mode = MovementMode::None;
                                    }
                                    continue;
                                }

                                // 🎬 根据 Player.action 计算初始速度（MovementSystem会从Player.action读取）
                                use crate::components::PlayerAction;
                                let _speed = if player.action == PlayerAction::Run {
                                    tracing::debug!("🏃 使用跑步速度: {}", velocity.run_speed);
                                    velocity.run_speed
                                } else {
                                    tracing::debug!("🚶 使用行走速度: {}", velocity.walk_speed);
                                    velocity.walk_speed
                                };

                                // ⚠️ 不要在这里再用 (target_x, target_y) 设置 velocity。
                                // Pathfinding 场景应以 waypoint 为准（上面已设置），否则会导致顶墙/反复算路。
                            }
                            MovementMode::DirectFollow => {
                                // DirectFollow已在上面单独处理
                            }
                            MovementMode::None => {
                                // 无移动模式,不设置路径
                            }
                        }
                    } else {
                        // 目标没变,不更新路径
                        // tracing::trace!("路径目标未改变,跳过更新");
                    }
                } else {
                    // 当前格子 == 目标格子
                    // 这只是格子坐标相同,角色可能还在移动到格子中心
                    // 对于长按模式:保持 move_to,等鼠标松开或移到其他格子
                    // 对于寻路模式:清除 move_to,结束移动
                    use crate::components::MovementMode;
                    tracing::trace!(
                        "📍 到达目标格子 ({}, {}), mode={:?}",
                        target_grid.0,
                        target_grid.1,
                        player_input.movement_mode
                    );
                    if player_input.movement_mode == MovementMode::Pathfinding {
                        tracing::debug!("✅ 寻路到达目标格子,清除 move_to");
                        player_input.move_to = None;
                    } else {
                        tracing::trace!("💡 长按模式,保持 move_to");
                    }
                    // 不清除路径! 让 MovementSystem 完成移动到格子中心
                }
            } else {
                // 没有移动指令,确保路径被清除
                if path.is_valid {
                    tracing::debug!("🧹 无移动指令,清除路径");
                    path.clear();
                    velocity.stop();
                }
            }
        }

        Ok(())
    }
}

impl Default for PathfindingSystem {
    fn default() -> Self {
        Self::new()
    }
}

// 声明为逻辑系统
crate::logic_system!(PathfindingSystem);
