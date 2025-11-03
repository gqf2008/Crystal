// ============================================================================
// Layer 5: State Update - AnimationSystem
// Priority: 500
// ============================================================================
//
// **职责**：
// - 动画状态机更新
// - 帧切换
// - 动画混合
//
// **逻辑来源**：
// - C# MapObject.SetAction(): 设置动作
// - MapObject.ProcessFrames(): 更新帧
//   - 不同动作有不同帧数
//   - 循环/非循环动画
//   - 帧间隔控制
//
// ============================================================================

use hecs::World;
use ggez::GameResult;
use crate::ecs::GameContext;
use crate::ecs::components::animation_state::{AnimationControl, AnimationState};
use crate::ecs::systems::{System, priority};

/// 动画系统
pub struct AnimationSystem {
    /// 累积时间(秒)
    accumulated_time: f32,
}

impl AnimationSystem {
    pub fn new() -> Self {
        Self {
            accumulated_time: 0.0,
        }
    }

    /// 更新动画帧
    fn update_animation_frame(control: &mut AnimationControl, delta_time: f32) {
        let frame_count = control.current_state.frame_count();
        let frame_interval = control.current_state.frame_interval() as f32 / 60.0; // 转换为秒

        // 累积时间
        control.state_change_time = control.state_change_time.checked_sub(
            std::time::Duration::from_secs_f32(delta_time)
        ).unwrap_or(std::time::Instant::now());

        // 检查是否该切换帧
        if control.state_duration() >= frame_interval {
            control.current_frame = (control.current_frame + 1) % frame_count;

            // 如果动画播放完毕
            if control.current_frame == 0 && !control.loop_animation {
                // 非循环动画结束,切换到Idle
                control.set_state(AnimationState::Idle);
            }
        }
    }
}

impl Default for AnimationSystem {
    fn default() -> Self {
        Self::new()
    }
}

impl System for AnimationSystem {
    fn priority(&self) -> u32 {
        priority::ANIMATION
    }

    fn update(&mut self, ctx: &mut GameContext, delay_time: f32) -> GameResult {
        self.accumulated_time += delay_time;

        // 更新所有动画控制器
        for (_, control) in ctx.world.query_mut::<&mut AnimationControl>() {
            Self::update_animation_frame(control, delay_time);
        }

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_animation_frame_update() {
        let mut control = AnimationControl::new();
        control.set_state(AnimationState::Walk);

        // Walk有6帧
        assert_eq!(control.current_state.frame_count(), 6);
        assert!(control.current_state.is_looping());
    }

    #[test]
    fn test_non_looping_animation() {
        let mut control = AnimationControl::new();
        control.set_state(AnimationState::Attack);

        assert_eq!(control.current_state.frame_count(), 6);
        assert!(!control.current_state.is_looping());
    }
}
