// ============================================================================
// Camera System - 摄像机系统
// ============================================================================
//
// 功能:
// - 屏幕/世界坐标转换
// - 鼠标拖拽
// - 缩放控制
// - 边缘滚屏
//
// ============================================================================

use hecs::World;
use crate::ecs::components::{Position, Camera, Draggable};

/// 相机系统
pub struct CameraSystem;

impl CameraSystem {
    /// 屏幕坐标转世界坐标
    pub fn screen_to_world(pos: &Position, camera: &Camera, screen_x: f32, screen_y: f32) -> (f32, f32) {
        (
            pos.x + (screen_x - camera.screen_width / 2.0) / camera.zoom,
            pos.y + (screen_y - camera.screen_height / 2.0) / camera.zoom,
        )
    }

    /// 世界坐标转屏幕坐标
    pub fn world_to_screen(pos: &Position, camera: &Camera, world_x: f32, world_y: f32) -> (f32, f32) {
        (
            (world_x - pos.x) * camera.zoom + camera.screen_width / 2.0,
            (world_y - pos.y) * camera.zoom + camera.screen_height / 2.0,
        )
    }

    /// 开始拖拽
    pub fn start_drag(draggable: &mut Draggable, pos: &Position, mouse_x: f32, mouse_y: f32) {
        draggable.is_dragging = true;
        draggable.drag_start_x = mouse_x;
        draggable.drag_start_y = mouse_y;
        draggable.drag_start_pos_x = pos.x;
        draggable.drag_start_pos_y = pos.y;
    }

    /// 更新拖拽
    pub fn update_drag(draggable: &Draggable, pos: &mut Position, camera: &Camera, mouse_x: f32, mouse_y: f32) {
        if draggable.is_dragging {
            let dx = mouse_x - draggable.drag_start_x;
            let dy = mouse_y - draggable.drag_start_y;
            pos.x = draggable.drag_start_pos_x - dx / camera.zoom;
            pos.y = draggable.drag_start_pos_y - dy / camera.zoom;
        }
    }

    /// 结束拖拽
    pub fn end_drag(draggable: &mut Draggable) {
        draggable.is_dragging = false;
    }

    /// 缩放
    pub fn zoom(_pos: &mut Position, camera: &mut Camera, delta: f32, _mouse_x: f32, _mouse_y: f32) {
        camera.zoom = (camera.zoom * (1.0 + delta * 0.1)).clamp(0.5, 3.0);
    }

    /// 🎯 更新摄像机系统 - 边缘滚屏（已禁用，使用智能跟随代替）
    pub fn update(_world: &mut World) {
        // 边缘滚屏已禁用，因为与智能相机跟随冲突
        // 现在角色移动时会自动触发智能跟随
        // 用户可以通过鼠标中键拖拽来手动移动视角
    }
}
