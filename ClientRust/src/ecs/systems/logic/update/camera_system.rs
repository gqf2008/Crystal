// ============================================================================
// Layer 5: State Update - CameraSystem
// Priority: 530
// ============================================================================
//
// **职责**：
// - 相机模式控制（Manual/FollowPlayer/Fixed）
// - 相机拖拽（中键拖动）
// - 相机缩放（滚轮）
// - 相机震动效果
//
// **注意**：
// - 坐标变换（世界→屏幕）由渲染系统执行（读取 Camera 组件）
// - 坐标变换（屏幕→世界）由输入系统执行（PlayerControlSystem）
//
// ============================================================================

use hecs::World;
use ggez::GameResult;
use ggez::input::mouse::MouseButton;
use crate::ecs::components::{Camera, CameraMode, Draggable, InputEvent, Position};
use crate::ecs::systems::{System, priority};
use crate::ecs::WorldExt;

/// 摄像机系统(矩阵计算)
/// 
/// 职责：
/// - 从 GlobalEvents 读取鼠标事件
/// - 处理相机模式切换（拖拽时切换到 Manual 模式）
/// - 处理相机拖拽（中键）
/// - 处理相机缩放（滚轮，所有模式下都生效）
/// - 计算震动效果
pub struct CameraSystem {
    /// 震动强度
    shake_intensity: f32,
    /// 震动持续时间
    shake_duration: f32,
    /// 震动时间
    shake_time: f32,
}

impl CameraSystem {
    pub fn new() -> Self {
        Self {
            shake_intensity: 0.0,
            shake_duration: 0.0,
            shake_time: 0.0,
        }
    }

    /// 触发摄像机震动
    pub fn trigger_shake(&mut self, intensity: f32, duration: f32) {
        self.shake_intensity = intensity;
        self.shake_duration = duration;
        self.shake_time = 0.0;
    }

    /// 开始拖拽
    pub fn start_drag(draggable: &mut crate::ecs::components::Draggable, pos: &crate::ecs::components::Position, x: f32, y: f32) {
        draggable.is_dragging = true;
        draggable.drag_start_x = x;
        draggable.drag_start_y = y;
        draggable.drag_start_pos_x = pos.x;
        draggable.drag_start_pos_y = pos.y;
    }

    /// 更新拖拽
    pub fn update_drag(
        draggable: &crate::ecs::components::Draggable,
        pos: &mut crate::ecs::components::Position,
        camera: &Camera,
        x: f32,
        y: f32,
    ) {
        if !draggable.is_dragging {
            return;
        }

        let dx = (x - draggable.drag_start_x) / camera.zoom;
        let dy = (y - draggable.drag_start_y) / camera.zoom;

        pos.x = draggable.drag_start_pos_x - dx;
        pos.y = draggable.drag_start_pos_y - dy;
    }

    /// 结束拖拽
    pub fn end_drag(draggable: &mut crate::ecs::components::Draggable) {
        draggable.is_dragging = false;
    }

    /// 缩放
    pub fn zoom(
        pos: &mut crate::ecs::components::Position,
        camera: &mut Camera,
        scroll_y: f32,
        mouse_x: f32,
        mouse_y: f32,
    ) {
        let old_zoom = camera.zoom;
        let zoom_speed = 0.1;
        
        if scroll_y > 0.0 {
            camera.zoom = (camera.zoom + zoom_speed).min(3.0);
        } else if scroll_y < 0.0 {
            camera.zoom = (camera.zoom - zoom_speed).max(0.5);
        }

        // 以鼠标位置为中心缩放
        let zoom_ratio = camera.zoom / old_zoom;
        let center_x = mouse_x - camera.screen_width / 2.0;
        let center_y = mouse_y - camera.screen_height / 2.0;
        
        pos.x += center_x / old_zoom - center_x / camera.zoom;
        pos.y += center_y / old_zoom - center_y / camera.zoom;
    }

    /// 计算震动偏移
    fn calculate_shake_offset(&self) -> (f32, f32) {
        if self.shake_time >= self.shake_duration {
            return (0.0, 0.0);
        }

        let progress = self.shake_time / self.shake_duration;
        let strength = self.shake_intensity * (1.0 - progress);
        
        // 简单的随机震动
        let offset_x = (self.shake_time * 50.0).sin() * strength;
        let offset_y = (self.shake_time * 60.0).cos() * strength;

        (offset_x, offset_y)
    }
}

impl Default for CameraSystem {
    fn default() -> Self {
        Self::new()
    }
}

impl System for CameraSystem {
    fn priority(&self) -> u32 {
        priority::CAMERA
    }

    fn update(&mut self, world: &mut World, delay_time: f32) -> GameResult {
        // 1. 从 GlobalEvents 读取输入事件
        let input_events = {
            let global_events = world.global_events();
            global_events.input_events.clone()
        };

        // 2. 查询 Camera + Draggable + Position + CameraMode 组件
        let mut camera_query: Vec<_> = world
            .query_mut::<(&mut Camera, &mut Draggable, &mut Position, &mut CameraMode)>()
            .into_iter()
            .collect();

        if let Some((_, (ref mut camera, ref mut draggable, ref mut pos, ref mut mode))) = camera_query.first_mut() {
            // 3. 处理鼠标事件
            for event in &input_events {
                match event {
                    InputEvent::MouseDown { button, x, y } => {
                        if *button == MouseButton::Middle {
                            // 切换到手动控制模式
                            **mode = CameraMode::Manual;
                            // 开始拖拽
                            draggable.is_dragging = true;
                            draggable.drag_start_x = *x;
                            draggable.drag_start_y = *y;
                            draggable.drag_start_pos_x = pos.x;
                            draggable.drag_start_pos_y = pos.y;
                            tracing::debug!("📹 切换到手动模式并开始拖拽相机: ({:.0}, {:.0})", x, y);
                        }
                    }
                    InputEvent::MouseMove { x, y, .. } => {
                        if draggable.is_dragging {
                            // 更新拖拽
                            let dx = (*x - draggable.drag_start_x) / camera.zoom;
                            let dy = (*y - draggable.drag_start_y) / camera.zoom;
                            pos.x = draggable.drag_start_pos_x - dx;
                            pos.y = draggable.drag_start_pos_y - dy;
                        }
                    }
                    InputEvent::MouseUp { button, .. } => {
                        if *button == MouseButton::Middle && draggable.is_dragging {
                            // 结束拖拽
                            draggable.is_dragging = false;
                            tracing::debug!("📹 结束拖拽相机: 最终位置 ({:.0}, {:.0})", pos.x, pos.y);
                        }
                    }
                    InputEvent::MouseWheel { x: _scroll_x, y: scroll_y } => {
                        // 缩放 - 注意: ggez 的 MouseWheel 事件不包含鼠标位置
                        // 我们简单地以屏幕中心为缩放点
                        let old_zoom = camera.zoom;
                        let zoom_speed = 0.1;

                        if *scroll_y > 0.0 {
                            camera.zoom = (camera.zoom + zoom_speed).min(3.0);
                        } else if *scroll_y < 0.0 {
                            camera.zoom = (camera.zoom - zoom_speed).max(0.5);
                        }

                        if camera.zoom != old_zoom {
                            tracing::debug!("🔍 相机缩放: {:.2}x", camera.zoom);
                        }
                    }
                    _ => {}
                }
            }

            // 确保zoom在合理范围
            camera.zoom = camera.zoom.clamp(0.5, 3.0);
        }

        // 4. 更新震动时间
        if self.shake_time < self.shake_duration {
            self.shake_time += delay_time;
        }

        // 5. 计算震动偏移（暂时不使用）
        let (shake_x, shake_y) = self.calculate_shake_offset();
        let _ = (shake_x, shake_y); // 暂时不用,避免警告

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_shake_trigger() {
        let mut system = CameraSystem::new();
        system.trigger_shake(10.0, 0.5);

        assert_eq!(system.shake_intensity, 10.0);
        assert_eq!(system.shake_duration, 0.5);
        assert_eq!(system.shake_time, 0.0);
    }

    #[test]
    fn test_shake_decay() {
        let mut system = CameraSystem::new();
        system.trigger_shake(10.0, 1.0);

        let (x1, y1) = system.calculate_shake_offset();
        assert!(x1.abs() <= 10.0 && y1.abs() <= 10.0);

        system.shake_time = 0.5;
        let (x2, y2) = system.calculate_shake_offset();
        assert!(x2.abs() < x1.abs() || y2.abs() < y1.abs()); // 震动应该衰减
    }
}
