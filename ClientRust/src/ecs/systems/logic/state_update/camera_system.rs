// ============================================================================
// Layer 5: State Update - CameraSystem
// Priority: 530
// ============================================================================
//
// **职责**：
// - 摄像机矩阵计算
// - 震动效果
// - 过场动画
// - 最终视图矩阵
//
// ============================================================================

use hecs::World;
use ggez::GameResult;
use crate::ecs::components::Camera;
use crate::ecs::systems::{System, priority};

/// 摄像机系统(矩阵计算)
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
        // 更新震动时间
        if self.shake_time < self.shake_duration {
            self.shake_time += delay_time;
        }

        // 计算震动偏移
        let (shake_x, shake_y) = self.calculate_shake_offset();

        // 应用到摄像机组件
        for (_, camera) in world.query_mut::<&mut Camera>() {
            // 可以在这里添加震动偏移到摄像机的view matrix
            // camera.shake_offset = (shake_x, shake_y);
            let _ = (shake_x, shake_y); // 暂时不用,避免警告
            
            // 确保zoom在合理范围
            camera.zoom = camera.zoom.clamp(0.5, 3.0);
        }

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
