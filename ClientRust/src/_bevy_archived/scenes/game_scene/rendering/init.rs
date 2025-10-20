// GameScene 渲染初始化系统
// 
// 功能说明:
// 1. 初始化游戏摄像机 (带 GameCamera 组件)
// 2. 加载初始地图
// 3. 设置摄像机初始位置

use bevy::prelude::*;
use crate::bevy::scenes::game_scene::{MapLoadRequest, GameCamera};

/// 初始化游戏渲染系统
/// 
/// 在进入游戏场景时调用,执行以下操作:
/// 1. 清理旧的Camera (防止多摄像机冲突)
/// 2. 清理所有旧的Sprite和ImageNode (防止纹理残留)
/// 3. 生成带有 GameCamera 组件的 2D 摄像机
/// 4. 请求加载初始地图
pub fn setup_game_rendering(
    mut commands: Commands,
    mut load_request: ResMut<MapLoadRequest>,
    mut clear_color: ResMut<ClearColor>, // Bevy 0.17使用全局ClearColor
    old_cameras: Query<Entity, (With<Camera2d>, Without<GameCamera>)>, // 查找旧的Camera
    old_sprites: Query<Entity, With<Sprite>>, // 查找所有旧的Sprite
    old_image_nodes: Query<Entity, With<ImageNode>>, // 查找所有旧的ImageNode (UI纹理)
) {
    info!("🎨 初始化游戏渲染系统");
    
    // 清理所有旧的Camera (防止多摄像机冲突)
    let old_camera_count = old_cameras.iter().count();
    if old_camera_count > 0 {
        info!("🧹 清理 {} 个旧摄像机", old_camera_count);
        for entity in old_cameras.iter() {
            commands.entity(entity).despawn();
        }
    }
    
    // 清理所有旧的Sprite (防止登录/选择场景的纹理残留)
    let old_sprite_count = old_sprites.iter().count();
    if old_sprite_count > 0 {
        info!("🧹 清理 {} 个旧Sprite", old_sprite_count);
        for entity in old_sprites.iter() {
            commands.entity(entity).despawn();
        }
    }
    
    // 清理所有旧的ImageNode (防止登录背景等UI纹理残留)
    let old_image_count = old_image_nodes.iter().count();
    if old_image_count > 0 {
        info!("🧹 清理 {} 个旧ImageNode (UI纹理)", old_image_count);
        for entity in old_image_nodes.iter() {
            commands.entity(entity).despawn();
        }
    }
    
    // 设置全局背景颜色为深蓝色
    *clear_color = ClearColor(Color::srgb(0.1, 0.1, 0.15));
    
    // 🔧 生成摄像机时，设置初始位置为地图中心 (700x700地图的中心是350, 350)
    // 这样玩家会显示在屏幕中央
    let player_grid_x = 350;
    let player_grid_y = 350;
    let player_world_x = player_grid_x as f32 * 48.0; // 16800.0
    let player_world_y = -(player_grid_y as f32 * 32.0); // -11200.0
    
    info!("🎯 摄像机初始位置设置为地图中心: 网格({}, {}) → 世界({:.1}, {:.1})", 
        player_grid_x, player_grid_y, player_world_x, player_world_y);
    
    let mut game_camera = GameCamera::new();
    game_camera.target = Vec2::new(player_world_x, player_world_y); // Target也用负Y
    
    // 生成游戏摄像机，并立即设置到玩家位置
    let camera_entity = commands.spawn((
        Camera2d,
        game_camera,
        Transform::from_xyz(player_world_x, player_world_y, 0.0), // Transform和Target一致
        Name::new("GameCamera"),
    )).id();
    info!("📷 游戏摄像机已生成 (Entity: {:?}, 初始位置: 地图中心({:.0}, {:.0}), 背景: 深蓝色)", 
        camera_entity, player_world_x, player_world_y);
    
    // 加载初始地图 (地图 "0")
    load_request.request("0".to_string());
    info!("🗺️ 已请求加载地图: 0");
}

/// 清理游戏渲染系统
/// 
/// 在退出游戏场景时调用,清理所有渲染相关实体
pub fn cleanup_game_rendering(
    mut commands: Commands,
    camera_query: Query<Entity, With<GameCamera>>,
) {
    info!("🧹 清理游戏渲染系统");
    
    // 移除游戏摄像机
    for entity in camera_query.iter() {
        commands.entity(entity).despawn();
    }
    info!("✅ 游戏摄像机已移除");
}
