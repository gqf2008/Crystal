// ============================================================================
// Layer 2: 核心逻辑层 - 本地预测系统
// ============================================================================
// 职责：客户端预测玩家移动，实现零延迟响应
// 
// 工作流程：
// 1. 读取 PlayerInput（由 InputCollectingSystem 写入）
// 2. 调用 PathfindingService.find_path() 计算路径
// 3. 立即写入 MovementVelocity（不等服务器确认）
// 4. 写入 Prediction（记录预测状态）
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
    input::PlayerInput,
    movement::{MovementVelocity, Path, MovementState, Movement},
    prediction::Prediction,
};
use crate::algorithms::Pathfinding;
use crate::ecs::coordinates::Coordinates;
use crate::ecs::components::map::MapData;

pub struct LocalPredictionSystem;

impl LocalPredictionSystem {
    pub fn new() -> Self {
        Self
    }

    /// 根据速度向量计算8方向
    /// 返回 0-7: 0=Up, 1=UpRight, 2=Right, 3=DownRight, 4=Down, 5=DownLeft, 6=Left, 7=UpLeft
    fn calculate_direction(dx: f32, dy: f32) -> u8 {
        if dx.abs() < 0.01 && dy.abs() < 0.01 {
            return 4; // 默认朝下
        }
        
        // 计算角度 (atan2 返回 -π 到 π)
        let angle = f32::atan2(dy, dx);
        
        // 转换为 0-8 的索引（每个方向 45 度 = π/4）
        // 右=0°, 右下=45°, 下=90°, 左下=135°, 左=180°/-180°, 左上=-135°, 上=-90°, 右上=-45°
        let slice = ((angle + std::f32::consts::PI + std::f32::consts::PI / 8.0) / (std::f32::consts::PI / 4.0)) as i32;
        
        // 映射到游戏的8方向: 0=Up, 1=UpRight, 2=Right, 3=DownRight, 4=Down, 5=DownLeft, 6=Left, 7=UpLeft
        match slice % 8 {
            0 => 6, // 左 (180°)
            1 => 7, // 左上 (-135°)
            2 => 0, // 上 (-90°)
            3 => 1, // 右上 (-45°)
            4 => 2, // 右 (0°)
            5 => 3, // 右下 (45°)
            6 => 4, // 下 (90°)
            7 => 5, // 左下 (135°)
            _ => 4, // 默认朝下
        }
    }

    /// 🎯 Layer 2 核心：本地预测系统（客户端立即响应，不等服务器）
    /// 
    /// 执行顺序：在 InputCollectingSystem 之后，ClientNetworkSystem 发送命令之前
    /// 
    /// 数据流：
    /// - 读取：PlayerInput（点击位置、按键）
    /// - 调用：PathfindingService（寻路计算）
    /// - 写入：MovementVelocity（速度）、Path（路径）、Prediction（预测状态）
    pub fn update(world: &mut World, map_data: &MapData, _dt: f32) {
        // 遍历所有本地玩家（通常只有一个）
        let mut player_count = 0;
        
        // 🐛 调试：检查查询是否找到玩家
        static mut FIRST_CALL: bool = true;
        unsafe {
            if FIRST_CALL {
                println!("[LocalPredictionSystem] 🔍 首次调用，开始查询玩家...");
                FIRST_CALL = false;
            }
        }
        
        for (_entity, (position, mut player, mut input, mut velocity, mut path, mut movement_state, mut prediction)) in world
            .query_mut::<(
                &Position,
                &mut Player,  // 改为可变，以便更新direction
                &mut PlayerInput,  // 改为可变，以便清除 move_to
                Option<&mut MovementVelocity>,
                Option<&mut Path>,
                Option<&mut Movement>,
                Option<&mut Prediction>,
            )>()
            .with::<&LocalPlayer>()
        {
            player_count += 1;
            
            // 1️⃣ 检查是否有新的移动输入
            if let Some((target_x, target_y)) = input.move_to {
                // 🎯 区分两种移动模式：
                // - use_pathfinding = true: 双击移动，使用 A* 寻路（避障）
                // - use_pathfinding = false: 长按移动，直接朝向目标（不避障）
                
                if input.use_pathfinding {
                    // === 模式 1: 自动寻路（双击）===
                    let (current_gx, current_gy) = Coordinates::world_to_grid(position.x, position.y);
                    let (target_gx, target_gy) = Coordinates::world_to_grid(target_x, target_y);

                    // 调用寻路算法
                    if let Some(path_points) = Pathfinding::find_path(map_data, (current_gx, current_gy), (target_gx, target_gy)) {
                        println!("[LocalPredictionSystem] 🎯 寻路模式: ({}, {}) -> ({}, {}), 长度: {}",
                            current_gx, current_gy, target_gx, target_gy, path_points.len());

                        // 2️⃣ 写入路径组件
                        if let Some(path) = path.as_deref_mut() {
                            path.set_path(path_points.clone());
                        }

                        // 5️⃣ 记录预测状态
                        if let Some(prediction) = prediction.as_deref_mut() {
                            prediction.predicted_position = position.clone();
                            prediction.last_input_sequence += 1;
                        }
                    } else {
                        println!("[LocalPredictionSystem] ⚠️ 寻路失败: ({}, {}) -> ({}, {})",
                            current_gx, current_gy, target_gx, target_gy);
                    }
                    
                    // 清除移动输入（寻路模式只处理一次）
                    input.move_to = None;
                } else {
                    // === 模式 2: 直接跟随（长按）===
                    let dx = target_x - position.x;
                    let dy = target_y - position.y;
                    let distance = (dx * dx + dy * dy).sqrt();
                    
                    if distance > 10.0 {
                        let norm_dx = dx / distance;
                        let norm_dy = dy / distance;
                        
                        // 直接设置速度
                        if let Some(vel) = velocity.as_deref_mut() {
                            let speed = if input.is_running { vel.run_speed } else { vel.walk_speed };
                            vel.set(norm_dx * speed, norm_dy * speed);
                            
                            // 更新朝向
                            player.direction = Self::calculate_direction(norm_dx, norm_dy);
                        }
                    } else {
                        // 太近了，停止
                        if let Some(vel) = velocity.as_deref_mut() {
                            vel.stop();
                        }
                    }
                    
                    // 注意：长按模式不清除 move_to（需要每帧更新）
                }
                
                // 4️⃣ 更新移动状态
                if let Some(movement_state) = movement_state.as_deref_mut() {
                    movement_state.state = if input.is_running {
                        MovementState::Running
                    } else {
                        MovementState::Walking
                    };
                }
            }
            
            // 3️⃣ 更新速度朝向当前waypoint（每帧都执行，无论是否有新输入）
            // 传奇标准速度：每格48像素
            // 走路：150px/s (约3格/秒)，跑步：250px/s (约5格/秒)
            let run_speed = if input.is_running { 250.0 } else { 150.0 };
            
            if let (Some(path_comp), Some(velocity_comp)) = (path.as_deref_mut(), velocity.as_deref_mut()) {
                if let Some(target_waypoint) = path_comp.current_waypoint() {
                    let (target_wx, target_wy) = Coordinates::grid_to_world_center(target_waypoint.0, target_waypoint.1);
                    let dx = target_wx - position.x;
                    let dy = target_wy - position.y;
                    let distance = (dx * dx + dy * dy).sqrt();
                    
                    println!("[LocalPredictionSystem] 当前位置: ({:.1}, {:.1}), waypoint: ({}, {}), 目标中心: ({:.1}, {:.1}), dx: {:.1}, dy: {:.1}, distance: {:.1}",
                        position.x, position.y, target_waypoint.0, target_waypoint.1, target_wx, target_wy, dx, dy, distance);
                    
                    // 检查是否到达waypoint（使用半格距离作为阈值：24像素）
                    if distance > 24.0 {
                        // 归一化方向向量
                        let norm_dx = dx / distance;
                        let norm_dy = dy / distance;
                        
                        // 设置速度
                        velocity_comp.set(norm_dx * run_speed, norm_dy * run_speed);
                        
                        // 🐛 调试：确认速度已设置
                        println!("[LocalPredictionSystem] ✅ 速度已设置: ({:.2}, {:.2}), magnitude: {:.2}",
                            velocity_comp.x, velocity_comp.y, velocity_comp.magnitude());
                        
                        // 🎯 更新玩家朝向
                        let new_direction = Self::calculate_direction(norm_dx, norm_dy);
                        player.direction = new_direction;
                        
                        println!("[LocalPredictionSystem] 设置速度: ({:.2}, {:.2}), 方向: {} (dx={:.2}, dy={:.2})",
                            norm_dx * run_speed, norm_dy * run_speed, new_direction, norm_dx, norm_dy);
                    } else {
                        // 到达当前waypoint,前进到下一个
                        let old_index = path_comp.current_index;
                        let old_wp = target_waypoint;
                        
                        println!("[LocalPredictionSystem] ✅ 到达waypoint {:?} (索引{}), 前进到下一个",
                            old_wp, old_index);
                        
                        if !path_comp.advance() {
                            // 路径完成，停止移动
                            velocity_comp.set(0.0, 0.0);
                            println!("[LocalPredictionSystem] ✅ 路径完成,停止移动");
                        } else {
                            let new_index = path_comp.current_index;
                            let new_wp = path_comp.current_waypoint();
                            println!("[LocalPredictionSystem] 📍 新waypoint: {:?} (索引{}), 路径总长: {}",
                                new_wp, new_index, path_comp.waypoints.len());
                        }
                    }
                } else {
                    // 没有路径，停止移动
                    if velocity_comp.magnitude() > 0.01 {
                        velocity_comp.set(0.0, 0.0);
                    }
                }
            }
        }
        
        if player_count == 0 {
            println!("[LocalPredictionSystem] ⚠️ 没有找到具有 LocalPlayer 标记的实体!");
        }
    }

    /// 辅助方法：获取当前格子坐标
    #[allow(dead_code)]
    fn get_grid_position(position: &Position) -> (i32, i32) {
        Coordinates::world_to_grid(position.x, position.y)
    }
}
