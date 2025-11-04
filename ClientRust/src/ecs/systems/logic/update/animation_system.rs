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
    fn update_animation_frame(control: &mut AnimationControl, delta_time: f32) {
        let frame_count = control.current_state.frame_count();
        let base_frame_interval = control.current_state.frame_interval() as f32 / 60.0; // 转换为秒
        
        // 🎯 关键修复：根据速度缩放调整帧间隔
        // speed_scale越大，帧间隔越短，动画播放越快
        let frame_interval = if control.speed_scale > 0.01 {
            base_frame_interval / control.speed_scale
        } else {
            base_frame_interval
        };

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

        // 🎯 关键修复：根据移动状态调整动画速度
        // 注意：不能用actual_speed，因为碰撞时velocity=0但我们希望动画继续播放
        use crate::ecs::components::movement::MovementVelocity;
        use crate::ecs::components::Player;
        
        for (_, (control, velocity, player)) in ctx.world.query_mut::<(
            &mut AnimationControl,
            &MovementVelocity,
            Option<&Player>,
        )>() {
            // 🎯 根据动画状态设置速度缩放
            let speed_scale = match control.current_state {
                AnimationState::Walk => {
                    // 走路动画：基础速度
                    1.0
                }
                AnimationState::Run => {
                    // 跑步动画：根据跑步/走路速度比例加速
                    // 一般跑步速度是走路的1.5-2倍，所以动画也应该快1.5-2倍
                    if velocity.walk_speed > 0.01 {
                        (velocity.run_speed / velocity.walk_speed).clamp(1.0, 2.5)
                    } else {
                        1.5 // 默认1.5倍速度
                    }
                }
                _ => {
                    // 其他动画保持正常速度
                    1.0
                }
            };
            
            // 🎯 更新速度缩放因子
            control.speed_scale = speed_scale;
        }

        // 更新所有动画帧
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
