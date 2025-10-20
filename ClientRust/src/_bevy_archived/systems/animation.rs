// Animation System - 动画系统
use bevy::prelude::*;
use crate::bevy::components::AnimationState;

/// 动画更新系统
pub fn animation_system(
    time: Res<Time>,
    mut query: Query<&mut AnimationState>,
) {
    let delta = time.delta_secs();
    
    for mut anim in query.iter_mut() {
        anim.update(delta);
    }
}
