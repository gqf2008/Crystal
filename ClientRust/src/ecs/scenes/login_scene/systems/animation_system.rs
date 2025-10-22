//! 动画系统 - 更新所有动画精灵

use hecs::World;
use super::super::components::*;

/// 更新所有动画精灵
pub fn update_animations(world: &mut World, delta_time: f32) {
    for (_entity, anim_sprite) in world.query_mut::<&mut AnimatedSprite>() {
        if anim_sprite.paused {
            continue;
        }

        anim_sprite.timer += delta_time;
        if anim_sprite.timer >= anim_sprite.frame_duration {
            anim_sprite.timer = 0.0;
            anim_sprite.current_frame += 1;

            if anim_sprite.current_frame >= anim_sprite.frame_count {
                if anim_sprite.loop_animation {
                    anim_sprite.current_frame = 0;
                } else {
                    anim_sprite.current_frame = anim_sprite.frame_count - 1;
                    anim_sprite.paused = true;
                    tracing::debug!("🎬 动画播放完成");
                }
            }
        }
    }
}

/// 检查动画是否完成
pub fn is_animation_complete(world: &World) -> bool {
    for (_entity, anim_sprite) in world.query::<&AnimatedSprite>().iter() {
        if !anim_sprite.paused && anim_sprite.current_frame < anim_sprite.frame_count - 1 {
            return false;
        }
    }
    true
}
