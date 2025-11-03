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

use ggez::GameResult;
use crate::ecs::{
    GameContext,
    components::{
        PlayerInput, Player, Position, Path, LocalPlayer, MapData, MovementVelocity,
    },
    systems::System,
    Coord,
};

pub struct PathfindingSystem;

impl PathfindingSystem {
    pub fn new() -> Self {
        Self
    }

    /// 世界坐标 → 格子坐标
    fn world_to_grid(world_x: f32, world_y: f32) -> (i32, i32) {
        Coord::world_to_grid(world_x, world_y)
    }
}

impl System for PathfindingSystem {
    fn priority(&self) -> u32 {
        crate::ecs::systems::priority::PATHFINDING
    }

    fn update(&mut self, ctx: &mut GameContext, _dt: f32) -> GameResult {
        // 获取地图数据(用于寻路)
        let map_data = ctx.world
            .query_mut::<&MapData>()
            .into_iter()
            .next()
            .map(|(_, data)| data.clone());

        // 处理本地玩家的移动输入
        for (_entity, (player_input, position, path, _player, _local, velocity)) in ctx.world
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

                // 检查目标是否与当前位置不同
                if current_grid != target_grid {
                    // 检查是否需要更新路径 - 避免每帧重复设置导致卡顿
                    let needs_update = if let Some(current_target) = path.current_waypoint() {
                        // 目标改变了才更新
                        current_target != target_grid
                    } else {
                        // 没有路径,需要设置
                        true
                    };

                    if needs_update {
                        if player_input.use_pathfinding {
                            // 寻路模式 (双击): 使用A*算法
                            // TODO: 使用 A* 寻路算法
                            // 现在先使用简单的直线路径
                            tracing::debug!(
                                "🔍 寻路: ({}, {}) -> ({}, {})",
                                current_grid.0, current_grid.1,
                                target_grid.0, target_grid.1
                            );
                            path.set_path(vec![target_grid]);
                        } else {
                            // 直接移动模式 (长按跟随): 只在目标格子改变时更新
                            // 这样避免了每帧重设路径导致的卡顿
                            path.set_path(vec![target_grid]);
                        }

                        // 设置移动速度
                        if player_input.is_running {
                            velocity.max_speed = velocity.run_speed;
                        } else {
                            velocity.max_speed = velocity.walk_speed;
                        }
                    }
                } else {
                    // 已到达目标格子中心
                    // 对于长按模式:不清除 move_to,让 PlayerControlSystem 在松开鼠标时清除
                    // 对于寻路模式:清除 move_to
                    if player_input.use_pathfinding {
                        player_input.move_to = None;
                    }
                    path.clear();
                    velocity.stop();
                }
            } else {
                // 没有移动指令,确保路径被清除
                if path.is_valid {
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
