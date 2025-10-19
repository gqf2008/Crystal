// Debug系统 - 打印Transform信息

use bevy::prelude::*;
use super::camera::GameCamera;

/// 调试系统：打印摄像机和白色方块的Transform
pub fn debug_transforms_system(
    camera_query: Query<(Entity, &Transform, &GameCamera, Option<&Name>), With<Camera2d>>,
    sprite_query: Query<(Entity, &Transform, Option<&Name>), With<Sprite>>,
    mut frame_count: Local<u32>,
) {
    *frame_count += 1;
    
    // 每60帧打印一次
    if *frame_count % 60 != 1 {
        return;
    }
    
    info!("🔍 ===== Transform调试信息 (Frame {}) =====", *frame_count);
    
    // 打印摄像机信息
    for (entity, transform, game_camera, name) in camera_query.iter() {
        info!(
            "📷 摄像机 {:?} ({}): Translation=({:.1}, {:.1}, {:.1}), Target=({:.1}, {:.1})",
            entity,
            name.map(|n| n.as_str()).unwrap_or("Unknown"),
            transform.translation.x,
            transform.translation.y,
            transform.translation.z,
            game_camera.target.x,
            game_camera.target.y,
        );
    }
    
    // 打印前3个Sprite
    let mut count = 0;
    for (entity, transform, name) in sprite_query.iter() {
        if count >= 3 {
            break;
        }
        info!(
            "🎨 Sprite {:?} ({}): Translation=({:.1}, {:.1}, {:.1})",
            entity,
            name.map(|n| n.as_str()).unwrap_or("Unknown"),
            transform.translation.x,
            transform.translation.y,
            transform.translation.z,
        );
        count += 1;
    }
    
    info!("🔍 ===== 总共 {} 个Sprite =====", sprite_query.iter().count());
}
