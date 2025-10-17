// Camera System - 摄像机跟随系统
use bevy::prelude::*;
use crate::bevy::components::Player;

/// 摄像机跟随玩家系统
pub fn camera_follow_system(
    player_query: Query<&Transform, With<Player>>,
    mut camera_query: Query<&mut Transform, (With<Camera2d>, Without<Player>)>,
) {
    // Get player transform
    let mut player_iter = player_query.iter();
    let Some(player_transform) = player_iter.next() else {
        return;
    };
    
    // Get camera transform
    let mut camera_iter = camera_query.iter_mut();
    let Some(mut camera_transform) = camera_iter.next() else {
        return;
    };
    
    // 平滑跟随
    const LERP_FACTOR: f32 = 0.2;
    camera_transform.translation.x += (player_transform.translation.x - camera_transform.translation.x) * LERP_FACTOR;
    camera_transform.translation.y += (player_transform.translation.y - camera_transform.translation.y) * LERP_FACTOR;
}
