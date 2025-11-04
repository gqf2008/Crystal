// ============================================================================
// Layer 5: State Update - CharacterAnimationSystem
// Priority: 500
// ============================================================================
//
// **职责**：
// - 角色动画状态机更新
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

/// 角色动画系统 (处理所有角色的动画更新)
pub struct CharacterAnimationSystem {
    /// 累积时间(秒)
    accumulated_time: f32,
}

impl CharacterAnimationSystem {
    pub fn new() -> Self {
        Self {
            accumulated_time: 0.0,
        }
    }

    /// 更新动画帧
    /// 
    /// 🎯 **设计原则**：
    /// - 动画播放速度**固定不变**,由AnimationState.frame_interval()决定
    /// - 不受velocity影响,不需要speed_scale计算
    /// - Walk: 3帧间隔 → 3/60s = 0.05s/帧 (匹配96px/s)
    /// - Run: 1帧间隔 → 1/60s = 0.017s/帧 (匹配144px/s)
    fn update_animation_frame(control: &mut AnimationControl, delta_time: f32, is_first_frame: bool) {
        let frame_count = control.current_state.frame_count();
        
        // 🎯 固定帧间隔：直接使用AnimationState定义的值,不做任何调整
        let frame_interval_ticks = control.current_state.frame_interval();
        let frame_interval = frame_interval_ticks as f32 / 60.0; // 转换为秒
        
        // 🎯 添加日志查看实际读取的值
        if is_first_frame && matches!(control.current_state, AnimationState::Walk | AnimationState::Run) {
            tracing::info!("🎬 动画系统读取: {:?} - frame_interval_ticks={}, frame_interval={:.3}s", 
                control.current_state, frame_interval_ticks, frame_interval);
        }

        // 🎯 累积帧时间
        control.frame_timer += delta_time;

        // 🎯 检查是否该切换帧
        if control.frame_timer >= frame_interval {
            // 重置计时器（保留余数以保持精确）
            control.frame_timer -= frame_interval;
            
            // 切换到下一帧
            control.current_frame = (control.current_frame + 1) % frame_count;

            // 如果动画播放完毕（回到第0帧）
            if control.current_frame == 0 && !control.loop_animation {
                // 非循环动画结束,切换到Idle
                control.set_state(AnimationState::Idle);
            }
        }
    }
}

impl Default for CharacterAnimationSystem {
    fn default() -> Self {
        Self::new()
    }
}

impl System for CharacterAnimationSystem {
    fn priority(&self) -> u32 {
        priority::ANIMATION
    }

    fn update(&mut self, ctx: &mut GameContext, delay_time: f32) -> GameResult {
        self.accumulated_time += delay_time;

        // 🎯 **动画播放速度完全固定,不受velocity影响**
        // - Walk: 3帧间隔, 6帧/循环, 共0.3s (匹配96px/s速度)
        // - Run: 1帧间隔, 6帧/循环, 共0.1s (匹配144px/s速度)
        // - 动画由PlayerState控制类型(走/跑/站立)
        // - 速度由CollisionSystem控制velocity(碰撞时归零)
        // - 两者完全解耦,互不影响
        
        // 更新所有动画帧
        let mut is_first = true;
        for (_, control) in ctx.world.query_mut::<&mut AnimationControl>() {
            Self::update_animation_frame(control, delay_time, is_first);
            is_first = false;
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
