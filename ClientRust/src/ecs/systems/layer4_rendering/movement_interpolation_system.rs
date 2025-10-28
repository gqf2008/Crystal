// ============================================================================
// Movement Interpolation System - 移动插值系统 (Layer 4)
// ============================================================================
//
// 职责：
// - 计算角色移动时的屏幕偏移量（offset_move）
// - 实现原版C#的OffSetMove机制，实现平滑移动效果
// - 纯渲染层插值计算，不修改游戏逻辑状态
//
// 核心原理：
// - 根据动画帧进度计算offset_move
// - 更新movement_grid指向目标格子
// - Position在移动逻辑中逐帧更新（由Layer 2负责）
//
// 参考：
// - Client/MirObjects/PlayerObject.cs Line 864-1000
//
// 替代：
// - deprecated/AnimationSystem::update_movement_animation()
//
// ============================================================================

use hecs::World;
use crate::ecs::{
    components::{MovementAnimation, Player, Position, MoveMode, PlayerAction},
    Coordinates,
};

/// 移动插值系统
pub struct MovementInterpolationSystem;

impl MovementInterpolationSystem {
    /// 更新角色移动动画插值
    /// 
    /// # 功能
    /// - 只有在移动时才更新插值
    /// - 根据动画帧进度计算屏幕偏移量
    /// - 同步 current_grid 和 movement_grid
    /// - 渲染系统使用 offset_move 计算最终屏幕位置
    pub fn update(world: &mut World) {
        for (_, (player, pos, movement_anim)) in 
            world.query_mut::<(&Player, &Position, &mut MovementAnimation)>() 
        {
            // 只有在移动时才更新插值
            if !player.is_moving {
                // 停止移动时清零偏移
                movement_anim.offset_move = (0.0, 0.0);
                movement_anim.move_distance = 0;
                
                // 同步current_grid和movement_grid到当前位置
                let (grid_x, grid_y) = Coordinates::world_to_grid(pos.x, pos.y);
                movement_anim.current_grid = (grid_x, grid_y);
                movement_anim.movement_grid = (grid_x, grid_y);
                continue;
            }
            
            // 获取当前动画帧信息
            let frame_count = player.action.frame_count();
            let frame_index = player.frame_index;
            
            // 计算移动距离（格子数）
            let move_distance = match player.action {
                PlayerAction::Walk => 1,
                PlayerAction::Run => 2,  // 原版跑步移动2格
                _ => 0,
            };
            
            // 如果有移动距离，更新offset_move和movement_grid
            if move_distance > 0 {
                movement_anim.move_distance = move_distance;
                movement_anim.update_offset(frame_index, frame_count, player.direction);
                
                // 根据移动模式更新movement_grid
                match player.move_mode {
                    MoveMode::AutoPathfinding => {
                        // 寻路模式：使用path中的目标格子
                        if player.path_index < player.path.len() {
                            let (target_grid_x, target_grid_y) = player.path[player.path_index];
                            movement_anim.movement_grid = (target_grid_x, target_grid_y);
                        }
                    }
                    MoveMode::DirectFollow => {
                        // 直接跟随模式：根据target位置计算目标格子
                        let (target_grid_x, target_grid_y) = 
                            Coordinates::world_to_grid(player.target_x, player.target_y);
                        movement_anim.movement_grid = (target_grid_x, target_grid_y);
                    }
                    MoveMode::Idle => {
                        // 空闲状态：movement_grid = current_grid
                        let (grid_x, grid_y) = Coordinates::world_to_grid(pos.x, pos.y);
                        movement_anim.movement_grid = (grid_x, grid_y);
                    }
                }
                
                // 更新current_grid为当前Position所在格子
                let (grid_x, grid_y) = Coordinates::world_to_grid(pos.x, pos.y);
                movement_anim.current_grid = (grid_x, grid_y);
            } else {
                movement_anim.offset_move = (0.0, 0.0);
                movement_anim.move_distance = 0;
            }
        }
    }
}
