// Movement System - 移动系统
use bevy::prelude::*;
use crate::bevy::components::{GridPosition, Movement, RenderOffset};

/// 移动处理系统
pub fn movement_system(
    time: Res<Time>,
    mut query: Query<(&mut GridPosition, &Movement, &mut RenderOffset)>,
) {
    for (_grid_pos, _movement, _offset) in query.iter_mut() {
        // TODO: 实现网格移动逻辑
        // 1. 根据 Movement 组件更新 GridPosition
        // 2. 更新 RenderOffset 用于平滑动画
    }
}

/// 渲染偏移插值系统 - 平滑移动动画
pub fn render_offset_system(
    time: Res<Time>,
    mut query: Query<(&GridPosition, &mut Transform, &RenderOffset)>,
) {
    const CELL_WIDTH: f32 = 48.0;
    const CELL_HEIGHT: f32 = 32.0;
    
    for (grid_pos, mut transform, offset) in query.iter_mut() {
        // 计算目标世界坐标
        let target_x = (grid_pos.x as f32 * CELL_WIDTH) + offset.x;
        let target_y = (grid_pos.y as f32 * CELL_HEIGHT) + offset.y;
        
        // 插值到目标位置 (平滑移动)
        const LERP_FACTOR: f32 = 0.2;
        transform.translation.x += (target_x - transform.translation.x) * LERP_FACTOR;
        transform.translation.y += (target_y - transform.translation.y) * LERP_FACTOR;
    }
}
