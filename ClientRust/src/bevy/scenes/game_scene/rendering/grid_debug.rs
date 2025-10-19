// Grid Debug - 地图网格调试绘制系统
//
// 功能说明:
// - 按G键切换网格显示
// - 绘制绿色的48x32网格线
// - 帮助调试瓦片对齐问题

use bevy::prelude::*;
use super::map_renderer::MapRenderData;
use super::camera::GameCamera;

/// 网格线标记组件
#[derive(Component)]
pub struct GridLines;

/// 处理G键切换网格显示
pub fn toggle_grid_system(
    keyboard: Res<ButtonInput<KeyCode>>,
    mut map_data: ResMut<MapRenderData>,
) {
    if keyboard.just_pressed(KeyCode::KeyG) {
        map_data.show_grid = !map_data.show_grid;
        if map_data.show_grid {
            info!("🟢 开启地图网格显示");
        } else {
            info!("⚫ 关闭地图网格显示");
        }
    }
}

/// 渲染地图网格线（简化版 - 使用Gizmos）
pub fn render_grid_system(
    mut gizmos: Gizmos,
    map_data: Res<MapRenderData>,
    camera_query: Query<&Transform, With<GameCamera>>,
) {
    // 如果不显示网格，直接返回
    if !map_data.show_grid {
        return;
    }
    
    // 获取摄像机位置计算可见区域
    let Ok(camera_transform) = camera_query.single() else {
        return;
    };
    
    let camera_x = camera_transform.translation.x;
    let camera_y = camera_transform.translation.y;
    
    // 计算可见区域（留一些边距）
    let visible_width = 1920.0;  // 假设屏幕宽度
    let visible_height = 1080.0; // 假设屏幕高度
    let margin = 500.0;
    
    let min_x = camera_x - visible_width / 2.0 - margin;
    let max_x = camera_x + visible_width / 2.0 + margin;
    let min_y = camera_y - visible_height / 2.0 - margin;
    let max_y = camera_y + visible_height / 2.0 + margin;
    
    // 格子大小
    const CELL_WIDTH: f32 = 48.0;
    const CELL_HEIGHT: f32 = 32.0;
    
    // 绿色（半透明）
    let color = Color::srgba(0.0, 1.0, 0.0, 0.5);
    
    // 创建垂直线（沿X轴）
    let grid_min_x = (min_x / CELL_WIDTH).floor() * CELL_WIDTH;
    let grid_max_x = (max_x / CELL_WIDTH).ceil() * CELL_WIDTH;
    let grid_min_y = (min_y / CELL_HEIGHT).floor() * CELL_HEIGHT;
    let grid_max_y = (max_y / CELL_HEIGHT).ceil() * CELL_HEIGHT;
    
    // 绘制垂直线
    let mut x = grid_min_x;
    while x <= grid_max_x {
        gizmos.line(
            Vec3::new(x, grid_min_y, 0.5),
            Vec3::new(x, grid_max_y, 0.5),
            color
        );
        x += CELL_WIDTH;
    }
    
    // 绘制水平线
    let mut y = grid_min_y;
    while y <= grid_max_y {
        gizmos.line(
            Vec3::new(grid_min_x, y, 0.5),
            Vec3::new(grid_max_x, y, 0.5),
            color
        );
        y += CELL_HEIGHT;
    }
}
