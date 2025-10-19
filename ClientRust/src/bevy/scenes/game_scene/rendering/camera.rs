// Camera - 摄像机系统 (Bevy 版本)
//
// 功能说明:
// - 2D 摄像机,跟随玩家移动
// - 坐标转换 (世界坐标 ↔ 屏幕坐标)
// - 地图边界限制
// - 平滑跟随
//
// 移植自: ClientRust/src/scenes/game_scene/camera.rs

use bevy::prelude::*;
use bevy::window::PrimaryWindow;

/// 地图常量
const CELL_WIDTH: f32 = 48.0;
const CELL_HEIGHT: f32 = 32.0;

/// 游戏摄像机组件
///
/// 与 Bevy 的 Camera2d 配合使用,提供额外的游戏逻辑功能:
/// - 跟随玩家
/// - 地图边界限制
/// - 坐标转换工具
#[derive(Component, Debug)]
pub struct GameCamera {
    /// 目标位置 (世界坐标,像素)
    pub target: Vec2,
    /// 平滑跟随速度 (0-1, 默认 0.2)
    pub smoothness: f32,
    /// 缩放级别 (1.0 = 正常)
    pub zoom: f32,
    /// 地图边界 (像素)
    pub map_bounds: Option<Vec2>,
}

impl Default for GameCamera {
    fn default() -> Self {
        Self {
            target: Vec2::ZERO,
            smoothness: 0.2,
            zoom: 1.0,
            map_bounds: None,
        }
    }
}

impl GameCamera {
    /// 创建新摄像机
    pub fn new() -> Self {
        Self::default()
    }

    /// 设置地图边界 (像素)
    pub fn set_map_bounds(&mut self, width: f32, height: f32) {
        self.map_bounds = Some(Vec2::new(width, height));
    }

    /// 设置跟随目标 (世界坐标,像素)
    pub fn follow_target(&mut self, world_x: f32, world_y: f32) {
        self.target = Vec2::new(world_x, world_y);
    }

    /// 设置跟随目标 (地图格子坐标)
    pub fn follow_target_grid(&mut self, grid_x: i32, grid_y: i32) {
        let world_x = grid_x as f32 * CELL_WIDTH;
        let world_y = grid_y as f32 * CELL_HEIGHT;
        self.target = Vec2::new(world_x, world_y);
    }

    /// 计算限制后的目标位置 (带地图边界)
    pub fn clamp_target(&self, screen_width: f32, screen_height: f32) -> Vec2 {
        let Some(map_bounds) = self.map_bounds else {
            return self.target;
        };

        // 计算可视区域的半宽和半高
        let half_width = screen_width / (2.0 * self.zoom);
        let half_height = screen_height / (2.0 * self.zoom);

        // 限制摄像机位置，确保不超出地图边界
        let min_x = half_width.max(0.0);
        let max_x = (map_bounds.x - half_width).max(min_x);
        let min_y = half_height.max(0.0);
        let max_y = (map_bounds.y - half_height).max(min_y);

        Vec2::new(
            self.target.x.clamp(min_x, max_x),
            self.target.y.clamp(min_y, max_y),
        )
    }

    /// 世界坐标转屏幕坐标
    pub fn world_to_screen(&self, world_pos: Vec2, camera_pos: Vec2, screen_size: Vec2) -> Vec2 {
        (world_pos - camera_pos) * self.zoom + screen_size / 2.0
    }

    /// 屏幕坐标转世界坐标
    pub fn screen_to_world(&self, screen_pos: Vec2, camera_pos: Vec2, screen_size: Vec2) -> Vec2 {
        camera_pos + (screen_pos - screen_size / 2.0) / self.zoom
    }

    /// 获取可见区域 (世界坐标)
    pub fn get_visible_rect(&self, camera_pos: Vec2, screen_size: Vec2) -> (Vec2, Vec2) {
        let half_size = screen_size / (2.0 * self.zoom);
        let min = camera_pos - half_size;
        let max = camera_pos + half_size;
        (min, max)
    }

    /// 计算可见的地图格子范围
    pub fn get_visible_tiles(&self, camera_pos: Vec2, screen_size: Vec2, map_width: i32, map_height: i32) -> (i32, i32, i32, i32) {
        let (min, max) = self.get_visible_rect(camera_pos, screen_size);

        let start_x = ((min.x / CELL_WIDTH).floor() as i32 - 2).max(0);
        let end_x = ((max.x / CELL_WIDTH).ceil() as i32 + 2).min(map_width - 1);
        let start_y = ((min.y / CELL_HEIGHT).floor() as i32 - 2).max(0);
        let end_y = ((max.y / CELL_HEIGHT).ceil() as i32 + 2).min(map_height - 1);

        (start_x, end_x, start_y, end_y)
    }
}

/// 创建游戏摄像机
pub fn spawn_camera(mut commands: Commands) {
    commands.spawn((
        Camera2d,
        GameCamera::default(),
        Transform::default(),
        Name::new("GameCamera"),
    ));
    info!("✅ GameCamera 已创建");
}

/// 摄像机平滑跟随系统
///
/// 包含两个步骤:
/// 1. 更新摄像机目标位置 (跟随玩家)
/// 2. 平滑插值到目标位置
pub fn camera_follow_system(
    mut camera_query: Query<(&mut Transform, &mut GameCamera), With<Camera2d>>,
    player_query: Query<&Transform, (With<crate::bevy::components::Player>, Without<Camera2d>)>,
    window_query: Query<&Window, With<PrimaryWindow>>,
    time: Res<Time>,
) {
    let Ok(window) = window_query.single() else {
        return;
    };

    let screen_width = window.width();
    let screen_height = window.height();

    for (mut transform, mut camera) in camera_query.iter_mut() {
        // 步骤1: 更新摄像机目标位置为玩家位置
        if let Ok(player_transform) = player_query.single() {
            // 玩家的世界坐标 (Y轴已翻转)
            let player_world_x = player_transform.translation.x;
            let player_world_y = -player_transform.translation.y; // 转回正Y轴
            
            camera.target = Vec2::new(player_world_x, player_world_y);
            
            // 🔧 调试：每60帧打印一次摄像机跟随信息
            if (time.elapsed_secs() * 60.0) as u32 % 60 == 0 {
                debug!("📷 摄像机跟随玩家 | 玩家世界坐标:({:.1}, {:.1}) | 摄像机目标:({:.1}, {:.1})", 
                    player_world_x, player_world_y, camera.target.x, camera.target.y);
            }
        } else {
            // 🔧 调试：找不到玩家时警告
            if (time.elapsed_secs() * 60.0) as u32 % 180 == 0 {
                warn!("⚠️ 摄像机找不到玩家实体 (Player组件)");
            }
        }
        
        // 步骤2: 计算限制后的目标位置
        let clamped_target = camera.clamp_target(screen_width, screen_height);

        // 步骤3: 平滑插值到目标位置 (lerp)
        let current = Vec2::new(transform.translation.x, -transform.translation.y);
        
        // 🔧 修复抖动：减小lerp_factor，让跟随更平滑
        // smoothness = 0.2, delta = 0.016s (60fps) => lerp_factor ≈ 0.192
        let lerp_factor = (camera.smoothness * time.delta_secs() * 60.0).min(1.0);
        
        let new_pos = current.lerp(clamped_target, lerp_factor);

        transform.translation.x = new_pos.x;
        transform.translation.y = -new_pos.y; // Bevy Y轴向上,所以取负
    }
}

/// 摄像机缩放系统 (可选) - 已禁用
/// 
/// 注意: Bevy 0.17 中,OrthographicProjection 不再是组件
/// 缩放功能需要通过其他方式实现
#[allow(dead_code)]
pub fn camera_zoom_system(
    keyboard: Res<ButtonInput<KeyCode>>,
) {
    // TODO: 实现缩放功能
    // 可能需要使用 Camera2d 的其他配置
}
