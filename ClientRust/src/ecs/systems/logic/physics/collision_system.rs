// ============================================================================
// Layer 4: Physics & Movement - CollisionSystem
// Priority: 410 (在MovementSystem之后执行,检查并修正位置)
// ============================================================================
//
// **职责**：
// - 碰撞检测 (检查MovementSystem移动后的位置)
// - 地图边界限制
// - 停止velocity以阻止继续移动到障碍物
//
// **逻辑来源**：
// - C# MapControl.ValidPoint(): 检查是否可移动
//   - (M2CellInfo[x, y].BackImage & 0x20000000) == 0
// - Map.HasTarget(): 检查是否有阻挡实体
//
// ============================================================================

use hecs::World;
use ggez::GameResult;
use crate::ecs::GameContext;
use crate::ecs::components::{Position, movement::MovementVelocity, map::MapBounds};
use crate::ecs::systems::LogicSystem;

/// 碰撞检测系统
#[derive(ecs_macros::LogicSystem)]
pub struct CollisionSystem {
    /// 停止移动阈值(像素)
    stop_threshold: f32,
}

impl CollisionSystem {
    pub fn new() -> Self {
        Self {
            stop_threshold: 1.0,
        }
    }

    /// 检查位置是否在地图边界内
    fn is_within_bounds(x: f32, y: f32, bounds: &MapBounds) -> bool {
        x >= 0.0 && y >= 0.0 && x < bounds.width as f32 && y < bounds.height as f32
    }

    /// 将位置限制在地图边界内
    fn clamp_to_bounds(pos: &mut Position, bounds: &MapBounds) {
        pos.x = pos.x.clamp(0.0, (bounds.width - 1) as f32);
        pos.y = pos.y.clamp(0.0, (bounds.height - 1) as f32);
    }
}

impl Default for CollisionSystem {
    fn default() -> Self {
        Self::new()
    }
}

impl LogicSystem for CollisionSystem {
    

    fn update(&mut self, ctx: &mut GameContext, _delay_time: f32) -> GameResult {
        use crate::ecs::components::map::MapData;
        
        // 获取地图数据
        let (map_width, map_height, map_cells) = {
            let mut width = None;
            let mut height = None;
            let mut cells = None;
            
            for (_, map_data) in ctx.world.query::<&MapData>().iter() {
                width = Some(map_data.width);
                height = Some(map_data.height);
                cells = Some(map_data.cells.clone());
                break;
            }
            
            (width, height, cells)
        };

        let Some(width) = map_width else { return Ok(()); };
        let Some(height) = map_height else { return Ok(()); };
        let Some(cells) = map_cells else { return Ok(()); };

        // 检查移动方向的下一个格子是否有障碍物
        use crate::ecs::components::{PlayerInput, Player};
        for (_entity, (mut pos, vel, player_input, mut player)) in ctx.world.query_mut::<(&mut Position, &mut MovementVelocity, Option<&mut PlayerInput>, Option<&mut Player>)>() {
            // 🎯 检查velocity是否为零或接近零
            // 注意: 如果velocity为零,说明没有移动,不需要检查碰撞
            if vel.x.abs() < 0.01 && vel.y.abs() < 0.01 {
                continue;
            }
            
            // 🎯 关键修复：预测下一帧的位置，而不是检查当前位置
            // 这样可以在实际移动前就阻止碰撞
            let next_x = pos.x + vel.x * _delay_time;
            let next_y = pos.y + vel.y * _delay_time;
            
            let grid_x = (next_x / 48.0) as i32;
            let grid_y = (next_y / 32.0) as i32;
            
            // 边界检查
            if grid_x < 0 || grid_y < 0 || grid_x >= width || grid_y >= height {
                vel.stop();
                // 🎯 不清除 move_to，保持动画继续播放
                tracing::warn!("🛑 边界碰撞！停止velocity但保持动画 - Grid({}, {}), Pos({:.1}, {:.1})", 
                               grid_x, grid_y, pos.x, pos.y);
                continue;
            }

            // 数组范围检查
            if (grid_x as usize) >= cells.len() || (grid_y as usize) >= cells[grid_x as usize].len() {
                continue;
            }

            // 检查下一个位置的格子是否有障碍物
            let cell = &cells[grid_x as usize][grid_y as usize];
            let has_obstacle = (cell.back_image & 0x20000000) != 0;
            
            if has_obstacle {
                // 🎯 关键修复：下一个位置有障碍物！立即停止velocity
                // 但保持 move_to 不变，让动画继续播放
                vel.stop();
                
                // 🎯 不清除 move_to，不修改 movement_mode
                // 这样 PlayerStateSystem 仍认为在移动状态，动画会继续
                // 但由于 velocity=0，position 不会更新，形成"原地踏步"
                
                if player_input.is_some() {
                    tracing::warn!("🛑 碰撞！停止velocity但保持动画 - NextGrid({}, {}), CurPos({:.1}, {:.1})", 
                                   grid_x, grid_y, pos.x, pos.y);
                }
            }
        }

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_bounds_check() {
        let bounds = MapBounds { width: 100, height: 100 };
        
        assert!(CollisionSystem::is_within_bounds(50.0, 50.0, &bounds));
        assert!(!CollisionSystem::is_within_bounds(-1.0, 50.0, &bounds));
        assert!(!CollisionSystem::is_within_bounds(50.0, -1.0, &bounds));
        assert!(!CollisionSystem::is_within_bounds(100.0, 50.0, &bounds));
        assert!(!CollisionSystem::is_within_bounds(50.0, 100.0, &bounds));
    }

    #[test]
    fn test_clamp_to_bounds() {
        let bounds = MapBounds { width: 100, height: 100 };
        let mut pos = Position { x: -10.0, y: 110.0 };
        
        CollisionSystem::clamp_to_bounds(&mut pos, &bounds);
        
        assert_eq!(pos.x, 0.0);
        assert_eq!(pos.y, 99.0);
    }
}
