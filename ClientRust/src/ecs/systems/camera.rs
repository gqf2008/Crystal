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
use crate::ecs::components::{Position, Camera, Draggable, MouseInput};

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

    /// 🎯 更新摄像机系统 - 边缘滚屏
    pub fn update(world: &mut World) {
        // 边缘滚屏配置
        const EDGE_THRESHOLD: f32 = 100.0;
        const MIN_SCROLL_SPEED: f32 = 3.0;
        const MAX_SCROLL_SPEED: f32 = 20.0;
        const ACCELERATION: f32 = 0.5;

        // 查找摄像机实体
        let camera_query: Vec<_> = world
            .query::<(&Camera, &Position, &Draggable)>()
            .iter()
            .map(|(entity, (cam, pos, drag))| (entity, cam.clone(), pos.clone(), drag.is_dragging))
            .collect();

        if camera_query.is_empty() {
            return;
        }

        let (camera_entity, camera, _camera_pos, is_dragging) = &camera_query[0];

        // 边缘滚屏 (只在非拖拽状态下执行)
        if !is_dragging {
            let mouse_input = world.query_mut::<&MouseInput>()
                .into_iter()
                .next()
                .map(|(_, m)| (m.x, m.y));

            if let Some((mouse_x, mouse_y)) = mouse_input {
                let mut scroll_x = 0.0;
                let mut scroll_y = 0.0;

                // 水平方向
                if mouse_x < EDGE_THRESHOLD {
                    let ratio = (EDGE_THRESHOLD - mouse_x) / EDGE_THRESHOLD;
                    let accelerated_ratio = ratio.powf(1.0 + ACCELERATION);
                    scroll_x = -(MIN_SCROLL_SPEED + (MAX_SCROLL_SPEED - MIN_SCROLL_SPEED) * accelerated_ratio);
                } else if mouse_x > camera.screen_width - EDGE_THRESHOLD {
                    let dist = mouse_x - (camera.screen_width - EDGE_THRESHOLD);
                    let ratio = dist / EDGE_THRESHOLD;
                    let accelerated_ratio = ratio.powf(1.0 + ACCELERATION);
                    scroll_x = MIN_SCROLL_SPEED + (MAX_SCROLL_SPEED - MIN_SCROLL_SPEED) * accelerated_ratio;
                }

                // 垂直方向
                if mouse_y < EDGE_THRESHOLD {
                    let ratio = (EDGE_THRESHOLD - mouse_y) / EDGE_THRESHOLD;
                    let accelerated_ratio = ratio.powf(1.0 + ACCELERATION);
                    scroll_y = -(MIN_SCROLL_SPEED + (MAX_SCROLL_SPEED - MIN_SCROLL_SPEED) * accelerated_ratio);
                } else if mouse_y > camera.screen_height - EDGE_THRESHOLD {
                    let dist = mouse_y - (camera.screen_height - EDGE_THRESHOLD);
                    let ratio = dist / EDGE_THRESHOLD;
                    let accelerated_ratio = ratio.powf(1.0 + ACCELERATION);
                    scroll_y = MIN_SCROLL_SPEED + (MAX_SCROLL_SPEED - MIN_SCROLL_SPEED) * accelerated_ratio;
                }

                // 应用边缘滚动
                if scroll_x != 0.0 || scroll_y != 0.0 {
                    if let Ok(mut pos) = world.get::<&mut Position>(*camera_entity) {
                        pos.x += scroll_x;
                        pos.y += scroll_y;
                    }
                }
            }
        }
    }
}
