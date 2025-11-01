// ============================================================================
// Collision Detection Algorithm - 碰撞检测算法
// ============================================================================
//
// 提供地图碰撞检测的算法实现
//
// ============================================================================

use crate::ecs::components::MapData;
use crate::ecs::MapUtils;

/// 碰撞检测算法
pub struct Collision;

impl Collision {
    /// 检查某个位置是否可行走
    pub fn is_walkable(map_data: &MapData, x: i32, y: i32) -> bool {
        MapUtils::is_walkable(map_data, x, y)
    }
    
    /// 检查圆形区域是否与障碍物碰撞
    pub fn is_circle_blocked(
        map_data: &MapData,
        center_x: f32,
        center_y: f32,
        radius: f32,
    ) -> bool {
        use crate::ecs::Coord;
        
        let (grid_x, grid_y) = Coord::world_to_grid(center_x, center_y);
        let grid_radius = (radius / 48.0).ceil() as i32;
        
        for dy in -grid_radius..=grid_radius {
            for dx in -grid_radius..=grid_radius {
                let check_x = grid_x + dx;
                let check_y = grid_y + dy;
                
                // 计算距离
                let dist_sq = (dx * dx + dy * dy) as f32;
                if dist_sq <= (grid_radius * grid_radius) as f32 {
                    if !MapUtils::is_walkable(map_data, check_x, check_y) {
                        return true;
                    }
                }
            }
        }
        
        false
    }
}
