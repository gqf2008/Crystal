// ============================================================================
// Map Helper - 地图辅助函数
// ============================================================================

use crate::ecs::components::{MapData, CELL_WIDTH, CELL_HEIGHT};

pub struct MapHelper;

impl MapHelper {
    /// 🎯 找到地图中心的可行走位置（用于角色出生点）
    pub fn find_center_walkable_position(map_data: &MapData) -> (i32, i32) {
        let center_x = map_data.width / 2;
        let center_y = map_data.height / 2;
        
        // 螺旋搜索：从中心向外扩散
        for radius in 0i32..50i32 {
            for dx in -radius..=radius {
                for dy in -radius..=radius {
                    // 只检查当前半径的边界格子
                    if dx.abs() == radius || dy.abs() == radius {
                        let x = center_x + dx;
                        let y = center_y + dy;
                        
                        if Self::is_walkable(map_data, x, y) {
                            return (x, y);
                        }
                    }
                }
            }
        }
        
        // 如果实在找不到，返回中心
        (center_x, center_y)
    }
    
    /// 🎯 检查格子是否可行走（没有障碍物）
    pub fn is_walkable(map_data: &MapData, x: i32, y: i32) -> bool {
        // 边界检查
        if x < 0 || x >= map_data.width || y < 0 || y >= map_data.height {
            return false;
        }
        
        // ✅ 修正: 传奇地图是按 cells[x][y] 存储的
        let cell = &map_data.cells[x as usize][y as usize];
        
        // ✅ 传奇正确的障碍物判断逻辑：
        // back_image 的第 29 位 (0x20000000) 标记该格子是否有障碍物
        // 不能简单判断 front_image != 0，因为桥梁、地面装饰等也有 front_image 但可以行走
        let has_obstacle = (cell.back_image & 0x20000000) != 0;
        
        // 有障碍物标记 = 不可行走
        !has_obstacle
    }
    
    /// 🎯 格子坐标转世界坐标（中心点）
    pub fn grid_to_world(grid_x: i32, grid_y: i32) -> (f32, f32) {
        let world_x = (grid_x * CELL_WIDTH + CELL_WIDTH / 2) as f32;
        let world_y = (grid_y * CELL_HEIGHT + CELL_HEIGHT / 2) as f32;
        
        (world_x, world_y)
    }
    
    /// 🎯 世界坐标转格子坐标
    pub fn world_to_grid(world_x: f32, world_y: f32) -> (i32, i32) {
        let grid_x = (world_x / CELL_WIDTH as f32).floor() as i32;
        let grid_y = (world_y / CELL_HEIGHT as f32).floor() as i32;
        
        (grid_x, grid_y)
    }
}
