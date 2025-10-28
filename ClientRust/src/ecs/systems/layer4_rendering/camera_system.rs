// ============================================================================
// Camera System - 摄像机系统
// ============================================================================
//
// 功能:
// - 屏幕/世界坐标转换
// - 鼠标拖拽
// - 缩放控制
// - 边缘滚屏
//
// ============================================================================

use hecs::World;
use crate::ecs::components::{Position, Camera, Draggable};

/// 相机系统
pub struct CameraSystem;

impl CameraSystem {
    /// 屏幕坐标转世界坐标
    pub fn screen_to_world(pos: &Position, camera: &Camera, screen_x: f32, screen_y: f32) -> (f32, f32) {
        (
            pos.x + (screen_x - camera.screen_width / 2.0) / camera.zoom,
            pos.y + (screen_y - camera.screen_height / 2.0) / camera.zoom,
        )
    }

    /// 世界坐标转屏幕坐标
    pub fn world_to_screen(pos: &Position, camera: &Camera, world_x: f32, world_y: f32) -> (f32, f32) {
        (
            (world_x - pos.x) * camera.zoom + camera.screen_width / 2.0,
            (world_y - pos.y) * camera.zoom + camera.screen_height / 2.0,
        )
    }

    /// 开始拖拽
    pub fn start_drag(draggable: &mut Draggable, pos: &Position, mouse_x: f32, mouse_y: f32) {
        draggable.is_dragging = true;
        draggable.drag_start_x = mouse_x;
        draggable.drag_start_y = mouse_y;
        draggable.drag_start_pos_x = pos.x;
        draggable.drag_start_pos_y = pos.y;
    }

    /// 更新拖拽
    pub fn update_drag(draggable: &Draggable, pos: &mut Position, camera: &Camera, mouse_x: f32, mouse_y: f32) {
        if draggable.is_dragging {
            let dx = mouse_x - draggable.drag_start_x;
            let dy = mouse_y - draggable.drag_start_y;
            pos.x = draggable.drag_start_pos_x - dx / camera.zoom;
            pos.y = draggable.drag_start_pos_y - dy / camera.zoom;
        }
    }

    /// 结束拖拽
    pub fn end_drag(draggable: &mut Draggable) {
        draggable.is_dragging = false;
    }

    /// 缩放
    pub fn zoom(_pos: &mut Position, camera: &mut Camera, delta: f32, _mouse_x: f32, _mouse_y: f32) {
        camera.zoom = (camera.zoom * (1.0 + delta * 0.1)).clamp(0.5, 3.0);
    }

    /// 🎯 更新摄像机系统 - 直接跟随玩家
    pub fn update(world: &mut World) {
        Self::update_camera_follow(world);
    }
    
    /// 摄像机直接跟随玩家（居中显示）
    pub fn update_camera_follow(world: &mut World) {
        use crate::ecs::components::Player;
        
        let player_pos = world.query_mut::<(&Player, &Position)>()
            .into_iter()
            .next()
            .map(|(_, (_, pos))| (pos.x, pos.y));
        
        let Some((target_x, target_y)) = player_pos else { return };
        
        // 直接将相机位置设置为玩家位置（角色始终居中）
        for (_entity, (camera_pos, _camera)) in world.query_mut::<(&mut Position, &Camera)>() {
            let old_cam_x = camera_pos.x;
            let old_cam_y = camera_pos.y;
            
            camera_pos.x = target_x;
            camera_pos.y = target_y;
            
            // 只在摄像机移动时输出
            if (old_cam_x - target_x).abs() > 0.1 || (old_cam_y - target_y).abs() > 0.1 {
                println!("[CameraSystem] 📷 摄像机跟随: ({:.1}, {:.1}) -> ({:.1}, {:.1})",
                    old_cam_x, old_cam_y, target_x, target_y);
            }
        }
    }
    
    /// 智能相机跟随：只在角色接近边缘或离开屏幕时才移动相机
    pub fn update_smart_camera_follow(world: &mut World) {
        use crate::ecs::components::Player;
        
        // 获取玩家位置
        let player_pos = world.query_mut::<(&Player, &Position)>()
            .into_iter()
            .next()
            .map(|(_, (_, pos))| (pos.x, pos.y));
        
        let Some((player_x, player_y)) = player_pos else { return };
        
        // 获取相机信息
        for (_entity, (camera_pos, camera)) in world.query_mut::<(&mut Position, &Camera)>() {
            // 计算玩家在屏幕上的位置
            let screen_x = (player_x - camera_pos.x) * camera.zoom + camera.screen_width / 2.0;
            let screen_y = (player_y - camera_pos.y) * camera.zoom + camera.screen_height / 2.0;
            
            // 定义安全区域（距离屏幕边缘的距离）
            const EDGE_MARGIN: f32 = 300.0;  // 边缘安全距离
            const STOP_THRESHOLD: f32 = 400.0; // 停止跟随阈值
            
            // 检查玩家是否超出安全区域
            let too_left = screen_x < EDGE_MARGIN;
            let too_right = screen_x > camera.screen_width - EDGE_MARGIN;
            let too_top = screen_y < EDGE_MARGIN;
            let too_bottom = screen_y > camera.screen_height - EDGE_MARGIN;
            
            // 只有当玩家确实接近边缘时才跟随
            if too_left || too_right || too_top || too_bottom {
                // 计算目标位置（将玩家居中）
                let target_cam_x = player_x;
                let target_cam_y = player_y;
                
                let dx = target_cam_x - camera_pos.x;
                let dy = target_cam_y - camera_pos.y;
                let distance = (dx * dx + dy * dy).sqrt();
                
                // 如果距离很近，直接跳转避免抖动
                if distance < 50.0 {
                    camera_pos.x = target_cam_x;
                    camera_pos.y = target_cam_y;
                } else if distance < STOP_THRESHOLD {
                    // 在停止阈值内，使用较慢的速度
                    camera_pos.x += dx * 0.02;
                    camera_pos.y += dy * 0.02;
                } else {
                    // 距离较远时快速跟随
                    camera_pos.x += dx * 0.08;
                    camera_pos.y += dy * 0.08;
                }
            }
        }
    }
}
