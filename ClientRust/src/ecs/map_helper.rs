// ============================================================================
// Map Helper - 地图辅助函数 (兼容层)
// ============================================================================
//
// ⚠️ 已弃用: 此模块仅为向后兼容保留
// 🔄 新代码请使用:
//   - MapUtils::find_center_walkable_position()
//   - MapUtils::is_walkable()
//   - CoordinateSystem::grid_to_world_center()
//   - CoordinateSystem::world_to_grid()
// ============================================================================

use crate::ecs::components::MapData;
use crate::ecs::coordinate_system::{CoordinateSystem, MapUtils};

pub struct MapHelper;

impl MapHelper {
    /// 🎯 找到地图中心的可行走位置（用于角色出生点）
    /// 
    /// 🔄 委托给 MapUtils
    #[inline]
    #[deprecated(note = "使用 MapUtils::find_center_walkable_position() 代替")]
    pub fn find_center_walkable_position(map_data: &MapData) -> (i32, i32) {
        MapUtils::find_center_walkable_position(map_data)
    }
    
    /// 🎯 检查格子是否可行走（没有障碍物）
    /// 
    /// 🔄 委托给 MapUtils
    #[inline]
    #[deprecated(note = "使用 MapUtils::is_walkable() 代替")]
    pub fn is_walkable(map_data: &MapData, x: i32, y: i32) -> bool {
        MapUtils::is_walkable(map_data, x, y)
    }
    
    /// 🎯 格子坐标转世界坐标(格子中心位置)
    /// 
    /// 🔄 委托给 CoordinateSystem
    #[inline]
    #[deprecated(note = "使用 CoordinateSystem::grid_to_world_center() 代替")]
    pub fn grid_to_world(grid_x: i32, grid_y: i32) -> (f32, f32) {
        CoordinateSystem::grid_to_world_center(grid_x, grid_y)
    }
    
    /// 🎯 世界坐标转格子坐标
    /// 
    /// 🔄 委托给 CoordinateSystem
    #[inline]
    #[deprecated(note = "使用 CoordinateSystem::world_to_grid() 代替")]
    pub fn world_to_grid(world_x: f32, world_y: f32) -> (i32, i32) {
        CoordinateSystem::world_to_grid(world_x, world_y)
    }
}
