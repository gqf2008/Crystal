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
/// 1. 生成带有 GameCamera 组件的 2D 摄像机
/// 2. 请求加载初始地图
pub fn setup_game_rendering(
    mut commands: Commands,
    mut load_request: ResMut<MapLoadRequest>,
) {
    info!("🎨 初始化游戏渲染系统");
    
    // 生成游戏摄像机 (带 GameCamera 组件)
    commands.spawn((
        Camera2d::default(),
        GameCamera::new(),
        Name::new("GameCamera"),
    ));
    info!("📷 游戏摄像机已生成");
    
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
