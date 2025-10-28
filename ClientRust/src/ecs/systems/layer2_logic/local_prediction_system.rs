// ============================================================================
// Layer 2: 核心逻辑层 - 本地预测系统
// ============================================================================
// 职责：客户端预测玩家移动，实现零延迟响应
// 
// 工作流程：
// 1. 读取 PlayerInputComponent（由 InputCollectingSystem 写入）
// 2. 调用 PathfindingService.find_path() 计算路径
// 3. 立即写入 VelocityComponent（不等服务器确认）
// 4. 写入 PredictionComponent（记录预测状态）
//
// 设计原则：
// - 客户端预测 = 零延迟操作感
// - 服务器权威 = 防作弊，最终校正
// - 预测 + 校正 = 流畅 + 公平
// ============================================================================

use hecs::World;
use mir2_shared::Point;
use crate::ecs::components::{
    LocalPlayer,
    Position,
    Player,
    input::PlayerInputComponent,
    movement::{VelocityComponent, PathComponent, MovementStateComponent, MovementState},
    prediction::PredictionComponent,
};
use crate::algorithms::Pathfinding;
use crate::ecs::coordinates::Coordinates;
use crate::ecs::components::map::MapData;

pub struct LocalPredictionSystem;

impl LocalPredictionSystem {
    pub fn new() -> Self {
        Self
    }

    /// 🎯 Layer 2 核心：本地预测系统（客户端立即响应，不等服务器）
    /// 
    /// 执行顺序：在 InputCollectingSystem 之后，ClientNetworkSystem 发送命令之前
    /// 
    /// 数据流：
    /// - 读取：PlayerInputComponent（点击位置、按键）
    /// - 调用：PathfindingService（寻路计算）
    /// - 写入：VelocityComponent（速度）、PathComponent（路径）、PredictionComponent（预测状态）
    pub fn update(world: &mut World, map_data: &MapData, _dt: f32) {
        // 遍历所有本地玩家（通常只有一个）
        for (_entity, (position, player, input, mut velocity, mut path, mut movement_state, mut prediction)) in world
            .query_mut::<(
                &Position,
                &Player,
                &PlayerInputComponent,
                Option<&mut VelocityComponent>,
                Option<&mut PathComponent>,
                Option<&mut MovementStateComponent>,
                Option<&mut PredictionComponent>,
            )>()
            .with::<&LocalPlayer>()
        {
            // 1️⃣ 检查是否有新的移动输入
            if let Some((target_x, target_y)) = input.move_to {
                let (current_gx, current_gy) = Coordinates::world_to_grid(position.x, position.y);
                let (target_gx, target_gy) = Coordinates::world_to_grid(target_x, target_y);

                // 调用寻路算法
                if let Some(path_points) = Pathfinding::find_path(map_data, (current_gx, current_gy), (target_gx, target_gy)) {
                    tracing::info!(
                        "[LocalPredictionSystem] 🎯 客户端预测寻路: ({}, {}) -> ({}, {}), 路径长度: {}",
                        current_gx,
                        current_gy,
                        target_gx,
                        target_gy,
                        path_points.len()
                    );

                    // 2️⃣ 写入路径组件（使用格子坐标）
                    if let Some(path) = path.as_deref_mut() {
                        path.set_path(path_points.clone());
                    }

                    // 3️⃣ 计算第一步的速度（需要转换为世界坐标来计算方向）
                    let run_speed = if input.is_running { 5.0 } else { 3.0 };
                    if let Some(velocity) = velocity.as_deref_mut() {
                        if path_points.len() > 1 {
                            // 获取下一个格子的世界坐标中心
                            let (next_wx, next_wy) = Coordinates::grid_to_world_center(path_points[1].0, path_points[1].1);
                            let dx = next_wx - position.x;
                            let dy = next_wy - position.y;
                            let distance = (dx * dx + dy * dy).sqrt();
                            if distance > 0.01 {
                                velocity.set((dx / distance) * run_speed, (dy / distance) * run_speed);
                            }
                        }
                    }

                    // 4️⃣ 更新移动状态
                    if let Some(movement_state) = movement_state.as_deref_mut() {
                        movement_state.state = if input.is_running {
                            MovementState::Running
                        } else {
                            MovementState::Walking
                        };
                    }

                    // 5️⃣ 记录预测状态（用于后续校正）
                    if let Some(prediction) = prediction.as_deref_mut() {
                        prediction.predicted_position = position.clone();
                        prediction.last_input_sequence += 1;
                        tracing::debug!(
                            "[LocalPredictionSystem] 记录预测状态: seq={}, pos=({}, {})",
                            prediction.last_input_sequence,
                            position.x,
                            position.y
                        );
                    }
                } else {
                    tracing::warn!(
                        "[LocalPredictionSystem] ⚠️ 寻路失败: ({}, {}) -> ({}, {})",
                        current_gx,
                        current_gy,
                        target_gx,
                        target_gy
                    );
                }
            }
        }
    }

    /// 辅助方法：获取当前格子坐标
    #[allow(dead_code)]
    fn get_grid_position(position: &Position) -> (i32, i32) {
        Coordinates::world_to_grid(position.x, position.y)
    }
}
