// Movement System - 移动系统
use bevy::prelude::*;
use crate::bevy::components::{GridPosition, Movement, RenderOffset};
use mir2_shared::MirDirection;

/// 移动状态
#[derive(Component, Debug)]
pub struct MovementState {
    pub moving: bool,
    pub timer: f32,
    pub target_x: i32,
    pub target_y: i32,
}

impl MovementState {
    pub fn new() -> Self {
        Self {
            moving: false,
            timer: 0.0,
            target_x: 0,
            target_y: 0,
        }
    }
}

/// 获取方向的偏移量
fn get_direction_offset(direction: MirDirection) -> (i32, i32) {
    use mir2_shared::MirDirection::*;
    match direction {
        Up => (0, -1),
        UpRight => (1, -1),
        Right => (1, 0),
        DownRight => (1, 1),
        Down => (0, 1),
        DownLeft => (-1, 1),
        Left => (-1, 0),
        UpLeft => (-1, -1),
    }
}

/// 移动处理系统
pub fn movement_system(
    time: Res<Time>,
    mut query: Query<(&mut GridPosition, &Movement, &mut RenderOffset)>,
) {
    for (mut grid_pos, movement, mut offset) in query.iter_mut() {
        // 简单的持续移动逻辑 (后续可以改为 FSM)
        // TODO: 添加碰撞检测、路径查找等
        
        // 暂时只更新方向,不自动移动
        // 移动将由单独的命令触发
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
