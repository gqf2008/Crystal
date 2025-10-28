// ============================================================================
// Pathfinding Algorithm - A* 寻路算法
// ============================================================================
//
// 无状态的寻路算法实现，提供 A* 路径规划
//
// 使用场景：
// - 客户端：本地预测移动路径
// - 服务器：验证路径合法性
// - AI系统：怪物寻路
//
// ============================================================================

use crate::objects::pathfinder::PathFinder;
use crate::ecs::components::MapData;
use crate::ecs::MapUtils;
use mir2_shared::Point;

/// A* 寻路算法
pub struct Pathfinding;

impl Pathfinding {
    /// 查找从起点到终点的路径
    /// 
    /// # 参数
    /// - map_data: 地图数据
    /// - start: 起点格子坐标 (x, y)
    /// - goal: 终点格子坐标 (x, y)
    /// 
    /// # 返回
    /// - Some(path): 路径点列表（包含起点和终点）
    /// - None: 无法到达
    pub fn find_path(
        map_data: &MapData,
        start: (i32, i32),
        goal: (i32, i32),
    ) -> Option<Vec<(i32, i32)>> {
        // 检查起点和终点是否相同
        if start == goal {
            return Some(vec![start]);
        }
        
        // 检查起点和终点是否可行走
        if !MapUtils::is_walkable(map_data, start.0, start.1) {
            tracing::warn!("❌ 寻路失败: 起点不可行走 ({}, {})", start.0, start.1);
            return None;
        }
        
        if !MapUtils::is_walkable(map_data, goal.0, goal.1) {
            tracing::warn!("❌ 寻路失败: 终点不可行走 ({}, {})", goal.0, goal.1);
            return None;
        }
        
        // 创建寻路器
        let map_data_clone = map_data.clone();
        let pathfinder = PathFinder::new(
            map_data.width,
            map_data.height,
            Box::new(move |p: Point| !MapUtils::is_walkable(&map_data_clone, p.x, p.y))
        );
        
        // 执行 A* 算法
        let start_point = Point::new(start.0, start.1);
        let goal_point = Point::new(goal.0, goal.1);
        
        pathfinder.find_path(start_point, goal_point)
            .map(|path| path.iter().map(|p| (p.x, p.y)).collect())
    }
    
    /// 验证路径是否合法（服务器端使用）
    /// 
    /// # 参数
    /// - map_data: 地图数据
    /// - path: 待验证的路径
    /// 
    /// # 返回
    /// - true: 路径合法（没有穿墙）
    /// - false: 路径非法
    pub fn validate_path(
        map_data: &MapData,
        path: &[(i32, i32)],
    ) -> bool {
        if path.is_empty() {
            return false;
        }
        
        // 检查路径中的每个点是否可行走
        for &(x, y) in path {
            if !MapUtils::is_walkable(map_data, x, y) {
                tracing::warn!("❌ 路径验证失败: ({}, {}) 不可行走", x, y);
                return false;
            }
        }
        
        // 检查相邻点之间的距离（防止跳跃）
        for window in path.windows(2) {
            let (x1, y1) = window[0];
            let (x2, y2) = window[1];
            
            let dx = (x2 - x1).abs();
            let dy = (y2 - y1).abs();
            
            // 只允许8方向移动（dx和dy都不超过1）
            if dx > 1 || dy > 1 {
                tracing::warn!("❌ 路径验证失败: ({}, {}) -> ({}, {}) 距离过远", x1, y1, x2, y2);
                return false;
            }
        }
        
        true
    }
    
    /// 简化路径（移除冗余的中间点）
    /// 
    /// 例如：[A, B, C, D] 如果 A->C 是直线且可行走，则可以简化为 [A, C, D]
    pub fn simplify_path(
        map_data: &MapData,
        path: &[(i32, i32)],
    ) -> Vec<(i32, i32)> {
        if path.len() <= 2 {
            return path.to_vec();
        }
        
        let mut simplified = vec![path[0]];
        let mut current_index = 0;
        
        while current_index < path.len() - 1 {
            // 尝试找到最远的可直达点
            let mut farthest_index = current_index + 1;
            
            for i in (current_index + 2)..path.len() {
                if Self::is_line_walkable(map_data, path[current_index], path[i]) {
                    farthest_index = i;
                } else {
                    break;
                }
            }
            
            simplified.push(path[farthest_index]);
            current_index = farthest_index;
        }
        
        simplified
    }
    
    /// 检查两点之间的直线是否可行走（Bresenham 算法）
    fn is_line_walkable(
        map_data: &MapData,
        start: (i32, i32),
        end: (i32, i32),
    ) -> bool {
        let (mut x0, mut y0) = start;
        let (x1, y1) = end;
        
        let dx = (x1 - x0).abs();
        let dy = (y1 - y0).abs();
        let sx = if x0 < x1 { 1 } else { -1 };
        let sy = if y0 < y1 { 1 } else { -1 };
        let mut err = dx - dy;
        
        loop {
            // 检查当前点是否可行走
            if !MapUtils::is_walkable(map_data, x0, y0) {
                return false;
            }
            
            if x0 == x1 && y0 == y1 {
                break;
            }
            
            let e2 = 2 * err;
            if e2 > -dy {
                err -= dy;
                x0 += sx;
            }
            if e2 < dx {
                err += dx;
                y0 += sy;
            }
        }
        
        true
    }
    
    /// 计算路径长度（曼哈顿距离）
    pub fn calculate_path_length(path: &[(i32, i32)]) -> i32 {
        if path.len() < 2 {
            return 0;
        }
        
        let mut length = 0;
        for window in path.windows(2) {
            let (x1, y1) = window[0];
            let (x2, y2) = window[1];
            length += (x2 - x1).abs() + (y2 - y1).abs();
        }
        
        length
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_path_validation() {
        // 测试路径验证逻辑
        let path = vec![(0, 0), (1, 0), (2, 0)];
        assert!(path.windows(2).all(|w| {
            let dx = (w[1].0 - w[0].0).abs();
            let dy = (w[1].1 - w[0].1).abs();
            dx <= 1 && dy <= 1
        }));
    }
    
    #[test]
    fn test_path_length() {
        let path = vec![(0, 0), (1, 0), (1, 1), (2, 1)];
        let length = Pathfinding::calculate_path_length(&path);
        assert_eq!(length, 3);
    }
}
