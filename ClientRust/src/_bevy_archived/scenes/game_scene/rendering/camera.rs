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
use bevy::input::{
    mouse::{MouseWheel, MouseScrollUnit, MouseMotion},
    keyboard::KeyCode,
};

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
    /// 是否是第一帧 (用于立即跳到玩家位置)
    pub first_frame: bool,
    /// 手动控制模式（拖拽时禁用自动跟随）
    pub manual_control: bool,
}

impl Default for GameCamera {
    fn default() -> Self {
        Self {
            target: Vec2::ZERO,
            smoothness: 0.2,
            zoom: 1.0,
            map_bounds: None,
            first_frame: true, // 第一帧立即跳到目标
            manual_control: false, // 默认自动跟随
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
        // X轴：正坐标系统 (0 到 map_bounds.x)
        let min_x = half_width.max(0.0);
        let max_x = (map_bounds.x - half_width).max(min_x);
        
        // Y轴：负坐标系统 (-map_bounds.y 到 0)
        // 地图顶部Y=0，地图底部Y=-map_bounds.y
        // 摄像机不能超出地图边界（考虑半屏高度）
        let max_y = -half_height; // 上边界（接近0，最大Y）
        let min_y = -(map_bounds.y - half_height); // 下边界（最负，最小Y）

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
        // 🖱️ 如果处于手动控制模式（拖拽），跳过自动跟随
        if camera.manual_control {
            continue;
        }
        
        // 步骤1: 更新摄像机目标位置为玩家位置
        if let Ok(player_transform) = player_query.single() {
            // 玩家的世界坐标 (直接使用，不转换Y轴)
            let player_world_x = player_transform.translation.x;
            let player_world_y = player_transform.translation.y; // 玩家已经在负Y，直接使用
            
            // 🔧 第一帧：立即打印玩家坐标
            if camera.first_frame {
                error!("📷 摄像机第一帧找到玩家: ({:.1}, {:.1})", player_world_x, player_world_y);
            }
            
            camera.target = Vec2::new(player_world_x, player_world_y);
            
            // 🔧 调试：每60帧打印一次摄像机跟随信息
            if (time.elapsed_secs() * 60.0) as u32 % 60 == 0 {
                debug!("📷 摄像机跟随玩家 | 玩家世界坐标:({:.1}, {:.1}) | 摄像机目标:({:.1}, {:.1})", 
                    player_world_x, player_world_y, camera.target.x, camera.target.y);
            }
        } else {
            // 🔧 调试：找不到玩家时警告
            if camera.first_frame {
                error!("❌ 摄像机第一帧找不到玩家！Query失败");
            }
            if (time.elapsed_secs() * 60.0) as u32 % 180 == 0 {
                warn!("⚠️ 摄像机找不到玩家实体 (Player组件)");
            }
        }
        
        // 步骤2: 计算限制后的目标位置
        let clamped_target = camera.clamp_target(screen_width, screen_height);
        
        // 🔧 调试：第一帧显示详细信息
        if camera.first_frame {
            error!("📷 摄像机第一帧调试:");
            error!("   - 原始target: ({:.1}, {:.1})", camera.target.x, camera.target.y);
            error!("   - 钳制后target: ({:.1}, {:.1})", clamped_target.x, clamped_target.y);
            error!("   - 屏幕尺寸: {:.0}x{:.0}", screen_width, screen_height);
            if let Some(bounds) = camera.map_bounds {
                error!("   - 地图边界: ({:.0}, {:.0})", bounds.x, bounds.y);
            }
        }

        // 步骤3: 平滑插值到目标位置 (lerp)
        let current = Vec2::new(transform.translation.x, transform.translation.y); // 也不转换
        
        let new_pos = if camera.first_frame {
            // 🔧 第一帧：立即跳到目标位置，让玩家显示在屏幕中央
            camera.first_frame = false;
            info!("📷 摄像机第一帧，立即跳到玩家位置: ({:.1}, {:.1})", clamped_target.x, clamped_target.y);
            clamped_target
        } else {
            // 之后的帧：平滑跟随
            let lerp_factor = (camera.smoothness * time.delta_secs() * 60.0).min(1.0);
            current.lerp(clamped_target, lerp_factor)
        };

        transform.translation.x = new_pos.x;
        transform.translation.y = new_pos.y; // 直接设置，不取负
    }
}

/// 摄像机缩放系统 (可选) - 已禁用
/// 
/// 注意: Bevy 0.17 中,OrthographicProjection 不再是组件
/// 摄像机缩放和拖拽控制系统
pub fn camera_zoom_system(
    mut mouse_wheel_events: EventReader<MouseWheel>,
    mouse_button: Res<ButtonInput<MouseButton>>,
    keyboard: Res<ButtonInput<KeyCode>>,
    mut mouse_motion_events: EventReader<MouseMotion>,
    mut camera_query: Query<(&mut GameCamera, &mut Transform), With<Camera2d>>,
    windows: Query<&Window>,
) {
    let Ok(window) = windows.single() else {
        return;
    };
    
    let Ok((mut game_camera, mut transform)) = camera_query.single_mut() else {
        return;
    };
    
    // 1. 滚轮缩放
    let mut zoomed = false;
    for event in mouse_wheel_events.read() {
        let zoom_delta = match event.unit {
            MouseScrollUnit::Line => event.y * 0.1,      // 行滚动
            MouseScrollUnit::Pixel => event.y * 0.001,   // 像素滚动
        };
        
        // 限制缩放范围: 0.5x ~ 3.0x
        let old_zoom = game_camera.zoom;
        game_camera.zoom = (game_camera.zoom + zoom_delta).clamp(0.5, 3.0);
        
        if (game_camera.zoom - old_zoom).abs() > 0.001 {
            info!("🔍 摄像机缩放: {:.2}x", game_camera.zoom);
            zoomed = true;
        }
    }
    
    // 2. 鼠标中键/右键拖拽地图
    let is_dragging = mouse_button.pressed(MouseButton::Middle) || mouse_button.pressed(MouseButton::Right);
    
    if is_dragging {
        // 🖱️ 拖拽时进入手动控制模式
        if !game_camera.manual_control {
            game_camera.manual_control = true;
            info!("🖱️ 进入手动拖拽模式（保持到按空格键恢复自动跟随）");
        }
        
        let mut dragged = false;
        for event in mouse_motion_events.read() {
            // 鼠标移动的像素转换为世界坐标移动（考虑缩放）
            let delta_x = -event.delta.x / game_camera.zoom;
            let delta_y = event.delta.y / game_camera.zoom; // Y轴翻转（屏幕Y向下，世界Y向上）
            
            // 更新摄像机目标位置
            game_camera.target.x += delta_x;
            game_camera.target.y += delta_y;
            
            // 立即更新Transform（无平滑）
            transform.translation.x = game_camera.target.x;
            transform.translation.y = game_camera.target.y;
            
            dragged = true;
        }
        
        // 拖拽时打印调试信息
        if dragged {
            info!("🖱️ 拖拽地图: 目标({:.1}, {:.1})", game_camera.target.x, game_camera.target.y);
        }
    }
    
    // 清空未使用的鼠标移动事件
    if !is_dragging {
        mouse_motion_events.clear();
    }
    
    // 3. 按空格键恢复自动跟随玩家
    if keyboard.just_pressed(KeyCode::Space) && game_camera.manual_control {
        game_camera.manual_control = false;
        info!("🔄 按空格键恢复自动跟随玩家");
    }
}

