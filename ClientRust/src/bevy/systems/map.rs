// Map Rendering System - 地图渲染系统
use bevy::prelude::*;
use crate::bevy::resources::GameConfig;

/// 地图组件
#[derive(Component)]
pub struct Map {
    pub width: i32,
    pub height: i32,
    pub name: String,
}

/// 地图瓦片组件
#[derive(Component)]
pub struct MapTile {
    pub tile_x: i32,
    pub tile_y: i32,
    pub layer: i32, // 0=地面, 1=物体, 2=顶层
}

/// 地图初始化系统
pub fn setup_map_system(
    mut commands: Commands,
) {
    // 创建一个简单的测试地图
    commands.spawn(Map {
        width: 100,
        height: 100,
        name: "测试地图".to_string(),
    });
    
    println!("🗺️ 地图系统初始化完成 (100×100)");
}

/// 地图可见性裁剪系统
/// 只渲染摄像机可见范围内的瓦片
pub fn map_culling_system(
    camera_query: Query<&Transform, With<Camera2d>>,
    mut tile_query: Query<(&MapTile, &mut Visibility)>,
    config: Res<GameConfig>,
) {
    // 获取第一个摄像机
    let Some(camera_transform) = camera_query.iter().next() else {
        return;
    };
    
    // 计算可见范围 (简化版)
    let camera_x = camera_transform.translation.x;
    let camera_y = camera_transform.translation.y;
    
    const VISIBLE_RANGE: f32 = 1000.0;
    
    for (tile, mut visibility) in tile_query.iter_mut() {
        let tile_world_x = tile.tile_x as f32 * config.cell_width;
        let tile_world_y = tile.tile_y as f32 * config.cell_height;
        
        let distance = ((tile_world_x - camera_x).powi(2) + (tile_world_y - camera_y).powi(2)).sqrt();
        
        *visibility = if distance < VISIBLE_RANGE {
            Visibility::Visible
        } else {
            Visibility::Hidden
        };
    }
}
