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

use crate::game::{GameResult, GameContext, Coord, PathFinder};
use crate::{
    components::{
        PlayerInput, Player, Position, Path, LocalPlayer, MapData, MovementVelocity,
    },
    systems::LogicSystem,
};
// use crate::objects::pathfinder::PathFinder;  // TODO: 实现 PathFinder

pub struct PathfindingSystem;

impl PathfindingSystem {
    pub fn new() -> Self {
        Self
    }

    /// 世界坐标 → 格子坐标
    fn world_to_grid(world_x: f32, world_y: f32) -> (i32, i32) {
        Coord::world_to_grid(world_x, world_y)
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
                .map(|p| (p.0 as i32, p.1 as i32))  // 转换 (usize, usize) -> (i32, i32)
                .collect()
        })
    }
}

impl LogicSystem for PathfindingSystem {

    fn update(&mut self, ctx: &mut GameContext, _dt: f32) -> GameResult {
        // 获取地图数据(用于寻路)
        let map_data = ctx.world
            .query_mut::<&MapData>()
            .into_iter()
            .next()
            .map(|(_, data)| data.clone());

        // 处理本地玩家的移动输入
        for (_entity, (player_input, position, path, player, _local, velocity)) in ctx.world
            .query_mut::<(
                &mut PlayerInput,
                &Position,
                &mut Path,
                &Player,
                &LocalPlayer,
                &mut MovementVelocity,
            )>()
            .into_iter()
        {
            // 检查是否有移动指令
            if let Some((target_x, target_y)) = player_input.move_to {
                let current_grid = Self::world_to_grid(position.x, position.y);
                let target_grid = Self::world_to_grid(target_x, target_y);
                
                tracing::trace!("🗺️ PathfindingSystem: current=({},{}), target=({},{}), mode={:?}", 
                    current_grid.0, current_grid.1, target_grid.0, target_grid.1, player_input.movement_mode);

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
                        
                        if distance > 5.0 { // 距离目标超过5像素才移动
                            // 🛡️ DirectFollow 模式需要预检查移动方向是否有障碍物
                            // 归一化方向向量
                            let dir_x = dx / distance;
                            let dir_y = dy / distance;
                            
                            // 预测移动一小段距离后的位置（比如10像素）
                            let check_distance = 10.0;
                            let next_x = position.x + dir_x * check_distance;
                            let next_y = position.y + dir_y * check_distance;
                            let next_grid_x = (next_x / 48.0) as i32;
                            let next_grid_y = (next_y / 32.0) as i32;
                            
                            // 检查是否有障碍物（需要地图数据）
                            let has_obstacle = if let Some(ref map) = map_data {
                                // 检查目标格子是否在地图范围内
                                let in_bounds = next_grid_x >= 0 && next_grid_y >= 0 
                                    && next_grid_x < map.width && next_grid_y < map.height;
                                
                                if in_bounds {
                                    let cell = &map.cells[next_grid_x as usize][next_grid_y as usize];
                                    (cell.back_image & 0x20000000) != 0
                                } else {
                                    true // 边界外视为障碍物
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
                                tracing::info!(
                                    "🔍 寻路: ({}, {}) -> ({}, {})",
                                    current_grid.0, current_grid.1,
                                    target_grid.0, target_grid.1
                                );
                                
                                // 🎯 使用 A* 算法计算完整路径
                                if let Some(ref map_data) = map_data {
                                    match Self::calculate_path(map_data, current_grid, target_grid) {
                                        Some(full_path) => {
                                            tracing::info!("✅ A* 找到路径，共 {} 个格子", full_path.len());
                                            if full_path.len() <= 10 {
                                                tracing::debug!("完整路径: {:?}", full_path);
                                            } else {
                                                tracing::debug!("路径开始: {:?} ...", &full_path[..5]);
                                                tracing::debug!("路径结束: ... {:?}", &full_path[full_path.len()-5..]);
                                            }
                                            path.set_path(full_path);
                                        }
                                        None => {
                                            tracing::warn!("❌ A* 找不到路径，使用直线");
                                            path.set_path(vec![target_grid]);
                                        }
                                    }
                                } else {
                                    tracing::warn!("⚠️ 地图数据不存在，使用直线路径");
                                    path.set_path(vec![target_grid]);
                                }
                                
                                // 🎬 根据 Player.action 计算初始速度（MovementSystem会从Player.action读取）
                                use crate::components::PlayerAction;
                                let speed = if player.action == PlayerAction::Run {
                                    tracing::debug!("🏃 使用跑步速度: {}", velocity.run_speed);
                                    velocity.run_speed
                                } else {
                                    tracing::debug!("🚶 使用行走速度: {}", velocity.walk_speed);
                                    velocity.walk_speed
                                };
                                
                                // 🚀 立即计算朝向目标的初始velocity（让MovementSystem第一帧就能检测到has_velocity）
                                let dx = target_x - position.x;
                                let dy = target_y - position.y;
                                let distance = (dx * dx + dy * dy).sqrt();
                                if distance > 5.0 {
                                    velocity.x = (dx / distance) * speed;
                                    velocity.y = (dy / distance) * speed;
                                    tracing::debug!("� 初始velocity=({:.1}, {:.1})", velocity.x, velocity.y);
                                }
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
                    tracing::trace!("📍 到达目标格子 ({}, {}), mode={:?}", 
                        target_grid.0, target_grid.1, player_input.movement_mode);
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
