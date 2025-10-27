// ============================================================================
// Pathfinding System - 寻路系统
// ============================================================================
//
// 职责：
// - A* 算法封装和路径计算
// - 路径存储和管理
// - 路径有效性检查
// - 避障路径重新计算
// - 处理输入事件触发的寻路
//
// ============================================================================

use hecs::World;
use tokio::sync::mpsc;
use crate::ecs::components::{
    Position, Player, MoveMode, PlayerAction, MouseInput, MapData, Camera,
};
use crate::ecs::{Coordinates, MapUtils};
use crate::network::NetworkCommand;
use crate::objects::pathfinder::PathFinder;
use mir2_shared::Point;

/// 寻路系统
pub struct PathfindingSystem;

impl PathfindingSystem {
    /// 屏幕坐标转世界坐标
    pub fn screen_to_world(mouse_x: f32, mouse_y: f32, camera_pos: &Position, camera: &Camera) -> (f32, f32) {
        let world_x = camera_pos.x + (mouse_x - camera.screen_width / 2.0) / camera.zoom;
        let world_y = camera_pos.y + (mouse_y - camera.screen_height / 2.0) / camera.zoom;
        (world_x, world_y)
    }
    
    /// 处理输入事件，更新寻路路径
    pub fn update(
        world: &mut World,
        _network_tx: Option<&mpsc::UnboundedSender<NetworkCommand>>,
    ) {
        // 获取鼠标输入
        let mouse_input = world.query_mut::<&MouseInput>()
            .into_iter()
            .next()
            .map(|(_, input)| input.clone());
        
        let mouse_input = match mouse_input {
            Some(input) => input,
            None => return,
        };
        
        // 获取相机信息
        let (camera_pos, camera) = world.query_mut::<(&Position, &Camera)>()
            .into_iter()
            .next()
            .map(|(_, (pos, cam))| (pos.clone(), cam.clone()))
            .unwrap_or((Position { x: 0.0, y: 0.0 }, Camera { zoom: 1.0, screen_width: 1280.0, screen_height: 720.0 }));
        
        // 获取地图数据
        let map_data = world.query_mut::<&MapData>()
            .into_iter()
            .next()
            .map(|(_, data)| data.clone());
        
        let map_data = match map_data {
            Some(data) => data,
            None => return,
        };
        
        // 计算鼠标指向的世界坐标
        let (mouse_world_x, mouse_world_y) = Self::screen_to_world(
            mouse_input.x, 
            mouse_input.y, 
            &camera_pos, 
            &camera
        );
        
        // 获取玩家当前位置（在循环外获取以避免借用冲突）
        let player_pos = world.query_mut::<(&Player, &Position)>()
            .into_iter()
            .next()
            .map(|(_, (_, pos))| (pos.x, pos.y))
            .unwrap_or((0.0, 0.0));
        
        let (start_grid_x, start_grid_y) = Coordinates::world_to_grid(player_pos.0, player_pos.1);
        
        // 处理所有玩家的寻路请求
        for (_entity, player) in world.query_mut::<&mut Player>() {
            // 检查是否需要触发寻路（双击或碰撞避障）
            let should_pathfind = mouse_input.left_double_clicked 
                || mouse_input.right_double_clicked
                || (player.collision_detected && player.move_mode == MoveMode::DirectFollow);
            
            if !should_pathfind {
                continue;
            }
            
            // 判断是否为跑步
            let mut is_run = mouse_input.right_double_clicked;
            
            // 跑步限制检查
            let now = std::time::Instant::now();
            if is_run {
                if !player.can_run {
                    tracing::info!("🚫 不能跑步: can_run=false, 强制改为走路");
                    is_run = false;
                } else if now.duration_since(player.last_run_time) > player.run_cooldown {
                    tracing::info!("⏰ 跑步冷却超时，强制改为走路");
                    player.can_run = false;
                    is_run = false;
                }
            }
            
            // 确定目标位置
            let (target_grid_x, target_grid_y) = if player.collision_detected {
                // 避障模式：目标是鼠标最后跟随的目标
                Coordinates::world_to_grid(player.target_x, player.target_y)
            } else {
                // 双击模式：目标是鼠标点击位置
                Coordinates::world_to_grid(mouse_world_x, mouse_world_y)
            };
            
            // 检查起点和终点是否相同
            if start_grid_x == target_grid_x && start_grid_y == target_grid_y {
                tracing::info!("⚠️ 起点==终点，忽略寻路");
                continue;
            }
            
            // 执行 A* 寻路
            let path = Self::find_path(
                &map_data,
                start_grid_x,
                start_grid_y,
                target_grid_x,
                target_grid_y,
            );
            
            if let Some(path) = path {
                if path.len() < 2 {
                    tracing::warn!("⚠️ 路径太短 ({}个点)，忽略", path.len());
                    continue;
                }
                
                // 更新玩家路径
                player.path = path;
                player.path_index = 1; // 跳过起点
                player.is_moving = true;
                player.action = if is_run { PlayerAction::Run } else { PlayerAction::Walk };
                player.speed = if is_run { 2.5 } else { 1.8 };
                player.move_mode = MoveMode::AutoPathfinding;
                player.waiting_server_confirm = false;
                
                tracing::info!("🗺️ 寻路成功: {} 个路径点 ({}), 起点=({}, {}), 第一个目标=({}, {})", 
                    player.path.len(), if is_run { "跑" } else { "走" },
                    player.path[0].0, player.path[0].1,
                    player.path[1].0, player.path[1].1);
                
                // 清除碰撞标记
                if player.collision_detected {
                    player.collision_detected = false;
                    player.collision_target_grid = None;
                }
            } else {
                tracing::warn!("❌ 寻路失败: 无法到达目标 ({}, {})", target_grid_x, target_grid_y);
                
                // 如果是避障寻路失败，停止移动
                if player.collision_detected {
                    player.is_moving = false;
                    player.action = PlayerAction::Stand;
                }
            }
        }
    }
    
    /// 执行 A* 寻路算法
    pub fn find_path(
        map_data: &MapData,
        start_x: i32,
        start_y: i32,
        target_x: i32,
        target_y: i32,
    ) -> Option<Vec<(i32, i32)>> {
        let map_data_clone = map_data.clone();
        let pathfinder = PathFinder::new(
            map_data.width,
            map_data.height,
            Box::new(move |p: Point| !MapUtils::is_walkable(&map_data_clone, p.x, p.y))
        );
        
        let start_point = Point::new(start_x, start_y);
        let target_point = Point::new(target_x, target_y);
        
        pathfinder.find_path(start_point, target_point)
            .map(|path| path.iter().map(|p| (p.x, p.y)).collect())
    }
    
    /// 检查路径是否仍然有效（没有被障碍物阻挡）
    pub fn is_path_valid(map_data: &MapData, path: &[(i32, i32)]) -> bool {
        for &(x, y) in path {
            if !MapUtils::is_walkable(map_data, x, y) {
                return false;
            }
        }
        true
    }
    
    /// 清除玩家的路径
    pub fn clear_path(world: &mut World) {
        for (_entity, player) in world.query_mut::<&mut Player>() {
            player.path.clear();
            player.path_index = 0;
            if player.move_mode == MoveMode::AutoPathfinding {
                player.move_mode = MoveMode::Idle;
                player.is_moving = false;
                player.action = PlayerAction::Stand;
            }
        }
    }
}
