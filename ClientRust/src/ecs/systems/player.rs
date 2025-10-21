// ============================================================================
// Player System - 角色系统
// ============================================================================
//
// 功能:
// - 角色移动控制
// - A* 寻路
// - 动画更新
// - 摄像机跟随
// - 状态机 (Idle/DirectFollow/AutoPathfinding)
//
// ============================================================================

use hecs::World;

// 🎯 从 components 导入统一的类型定义
use crate::ecs::components::{
    Position,
    Camera,
    Player,
    PlayerAction,
    MoveMode,
    MouseInput,
    MapData,
};

/// 角色系统
pub struct PlayerSystem;

impl PlayerSystem {
    /// 屏幕坐标转世界坐标
    pub fn screen_to_world(mouse_x: f32, mouse_y: f32, camera_pos: &Position, camera: &Camera) -> (f32, f32) {
        let world_x = camera_pos.x + (mouse_x - camera.screen_width / 2.0) / camera.zoom;
        let world_y = camera_pos.y + (mouse_y - camera.screen_height / 2.0) / camera.zoom;
        (world_x, world_y)
    }
    
    /// 计算两点间的方向（0-7，八方向）
    pub fn calculate_direction(dx: f32, dy: f32) -> u8 {
        let abs_dx = dx.abs();
        let abs_dy = dy.abs();
        
        let threshold = 0.1;
        
        if abs_dx < threshold && abs_dy < threshold {
            return 4; // 默认朝下
        }
        
        if abs_dx > abs_dy * 2.414 {
            if dx > 0.0 { 2 } else { 6 }  // Right / Left
        } else if abs_dy > abs_dx * 2.414 {
            if dy > 0.0 { 4 } else { 0 }  // Down / Up
        } else {
            if dx > 0.0 {
                if dy > 0.0 { 3 } else { 1 }  // DownRight / UpRight
            } else {
                if dy > 0.0 { 5 } else { 7 }  // DownLeft / UpLeft
            }
        }
    }
    
    /// 平滑方向转换
    pub fn smooth_direction(current: u8, target: u8) -> u8 {
        let diff = ((target as i32 - current as i32) + 8) % 8;
        if diff <= 1 || diff >= 7 {
            target
        } else if diff <= 4 {
            (current + 1) % 8
        } else {
            (current + 7) % 8
        }
    }
    
    /// 🎯 更新角色系统
    /// 
    /// 完整实现包含:
    /// - 双击检测 → 寻路
    /// - 长按检测 → 直接跟随
    /// - A* 寻路逻辑
    /// - 移动更新
    /// - 动画更新
    /// - 摄像机跟随
    pub fn update(world: &mut World) {
        use crate::objects::pathfinder::PathFinder;
        use mir2_shared::Point;
        use crate::ecs::map_helper::MapHelper;
        
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
        
        // 🎯 获取地图数据（用于寻路和碰撞检测）
        let map_data = world.query_mut::<&MapData>()
            .into_iter()
            .next()
            .map(|(_, data)| data.clone());
        
        let map_data = match map_data {
            Some(data) => data,
            None => return,
        };
        
        // 更新所有玩家
        for (_entity, (player, pos)) in world.query_mut::<(&mut Player, &mut Position)>() {
            // 📍 计算鼠标指向的世界坐标
            let (mouse_world_x, mouse_world_y) = PlayerSystem::screen_to_world(
                mouse_input.x, 
                mouse_input.y, 
                &camera_pos, 
                &camera
            );
            
            // 🎯 状态机：处理鼠标输入
            // 1. 双击事件 → 切换到自动寻路模式
            if mouse_input.left_double_clicked || mouse_input.right_double_clicked {
                let is_run = mouse_input.right_double_clicked;  // 右键=跑,左键=走
                
                match player.move_mode {
                    MoveMode::Idle => {
                        // 空闲状态 → 单击触发寻路
                        let (start_grid_x, start_grid_y) = MapHelper::world_to_grid(pos.x, pos.y);
                        
                        // ✅ 使用C#原版算法:屏幕坐标转地图坐标
                        const CELL_WIDTH: f32 = 48.0;
                        const CELL_HEIGHT: f32 = 32.0;
                        
                        let offset_x = (camera.screen_width / 2.0 / CELL_WIDTH) as i32;
                        let offset_y = (camera.screen_height / 2.0 / CELL_HEIGHT) as i32;
                        
                        let target_grid_x = (mouse_input.x / CELL_WIDTH) as i32 - offset_x + start_grid_x;
                        let target_grid_y = (mouse_input.y / CELL_HEIGHT) as i32 - offset_y + start_grid_y;
                        
                        // 使用 A* 寻路
                        let map_data_for_pathfinding = map_data.clone();
                        let pathfinder = PathFinder::new(
                            map_data.width,
                            map_data.height,
                            Box::new(move |p: Point| !MapHelper::is_walkable(&map_data_for_pathfinding, p.x, p.y))
                        );
                        
                        let start_point = Point::new(start_grid_x, start_grid_y);
                        let target_point = Point::new(target_grid_x, target_grid_y);
                        
                        if let Some(path) = pathfinder.find_path(start_point, target_point) {
                            player.path = path.iter().map(|p| (p.x, p.y)).collect();
                            player.path_index = 0;
                            player.is_moving = true;
                            player.action = if is_run { PlayerAction::Run } else { PlayerAction::Walk };
                            player.speed = if is_run { 1.6 } else { 1.33 };
                            player.move_mode = MoveMode::AutoPathfinding;
                            
                            println!("🗺️ 寻路成功: {} 个路径点 ({})", player.path.len(), if is_run { "跑" } else { "走" });
                        } else {
                            println!("❌ 寻路失败: 无法到达目标");
                        }
                    }
                    MoveMode::DirectFollow => {
                        // 直接跟随模式 → 单击切换到寻路
                        let (start_grid_x, start_grid_y) = MapHelper::world_to_grid(pos.x, pos.y);
                        let (target_grid_x, target_grid_y) = MapHelper::world_to_grid(mouse_world_x, mouse_world_y);
                        
                        let map_data_for_pathfinding = map_data.clone();
                        let pathfinder = PathFinder::new(
                            map_data.width,
                            map_data.height,
                            Box::new(move |p: Point| !MapHelper::is_walkable(&map_data_for_pathfinding, p.x, p.y))
                        );
                        
                        let start_point = Point::new(start_grid_x, start_grid_y);
                        let target_point = Point::new(target_grid_x, target_grid_y);
                        
                        if let Some(path) = pathfinder.find_path(start_point, target_point) {
                            player.path = path.iter().map(|p| (p.x, p.y)).collect();
                            player.path_index = 0;
                            player.is_moving = true;
                            player.action = if is_run { PlayerAction::Run } else { PlayerAction::Walk };
                            // 🎯 统一速度：走路1.8, 跑步2.5
                            player.speed = if is_run { 2.5 } else { 1.8 };
                            player.move_mode = MoveMode::AutoPathfinding;
                            println!("🎯 切换到寻路模式: {} 个路径点", player.path.len());
                        }
                    }
                    MoveMode::AutoPathfinding => {
                        // 自动寻路模式 → 双击更新寻路目标位置
                        let (start_grid_x, start_grid_y) = MapHelper::world_to_grid(pos.x, pos.y);
                        let (target_grid_x, target_grid_y) = MapHelper::world_to_grid(mouse_world_x, mouse_world_y);
                        
                        let map_data_for_pathfinding = map_data.clone();
                        let pathfinder = PathFinder::new(
                            map_data.width,
                            map_data.height,
                            Box::new(move |p: Point| !MapHelper::is_walkable(&map_data_for_pathfinding, p.x, p.y))
                        );
                        
                        let start_point = Point::new(start_grid_x, start_grid_y);
                        let target_point = Point::new(target_grid_x, target_grid_y);
                        
                        if let Some(path) = pathfinder.find_path(start_point, target_point) {
                            player.path = path.iter().map(|p| (p.x, p.y)).collect();
                            player.path_index = 0;
                            player.action = if is_run { PlayerAction::Run } else { PlayerAction::Walk };
                            // 🎯 统一速度：走路1.8, 跑步2.5
                            player.speed = if is_run { 2.5 } else { 1.8 };
                            println!("✅ 寻路目标已更新: {} 个路径点 ({})", player.path.len(), if is_run { "跑" } else { "走" });
                        } else {
                            println!("❌ 新目标寻路失败: 无法到达");
                        }
                    }
                }
            }
            // 2. 长按事件 → 直接跟随模式(不寻路，但需要检查碰撞)
            // 🎯 优化：降低延时从30帧→5帧，提升操作灵敏度
            else if (mouse_input.left_pressed && mouse_input.left_press_time >= 5) 
                  || (mouse_input.right_pressed && mouse_input.right_press_time >= 5) {
                let is_run = mouse_input.right_pressed;
                
                // 🎯 检查目标位置是否可行走
                let (target_grid_x, target_grid_y) = MapHelper::world_to_grid(mouse_world_x, mouse_world_y);
                let is_walkable = MapHelper::is_walkable(&map_data, target_grid_x, target_grid_y);
                
                if is_walkable {
                    match player.move_mode {
                        MoveMode::Idle | MoveMode::DirectFollow => {
                            player.target_x = mouse_world_x;
                            player.target_y = mouse_world_y;
                            player.is_moving = true;
                            player.action = if is_run { PlayerAction::Run } else { PlayerAction::Walk };
                            // 🎯 优化速度：走路1.5→1.8, 跑步1.6→2.5 (更快更流畅)
                            player.speed = if is_run { 2.5 } else { 1.8 };
                            player.move_mode = MoveMode::DirectFollow;
                            player.path.clear();
                        }
                        MoveMode::AutoPathfinding => {
                            player.target_x = mouse_world_x;
                            player.target_y = mouse_world_y;
                            player.is_moving = true;
                            player.action = if is_run { PlayerAction::Run } else { PlayerAction::Walk };
                            player.speed = if is_run { 2.5 } else { 1.8 };
                            player.move_mode = MoveMode::DirectFollow;
                            player.path.clear();
                            println!("🎯 切换到直接跟随模式");
                        }
                    }
                }
            }
            // 3. 松开鼠标 → 根据模式处理
            else {
                match player.move_mode {
                    MoveMode::DirectFollow => {
                        player.move_mode = MoveMode::Idle;
                        player.is_moving = false;
                        player.action = PlayerAction::Stand;
                    }
                    MoveMode::AutoPathfinding => {
                        // 继续寻路
                    }
                    MoveMode::Idle => {
                        // 保持空闲
                    }
                }
            }
            
            // 🎯 移动逻辑
            // 1. 自动寻路模式：沿路径移动
            if player.move_mode == MoveMode::AutoPathfinding && !player.path.is_empty() {
                if player.path_index < player.path.len() {
                    let (target_grid_x, target_grid_y) = player.path[player.path_index];
                    let (target_x, target_y) = MapHelper::grid_to_world(target_grid_x, target_grid_y);
                    
                    let dx = target_x - pos.x;
                    let dy = target_y - pos.y;
                    let distance = (dx * dx + dy * dy).sqrt();
                    
                    if distance < player.speed * 2.0 {
                        player.path_index += 1;
                        
                        if player.path_index >= player.path.len() {
                            player.move_mode = MoveMode::Idle;
                            player.is_moving = false;
                            player.action = PlayerAction::Stand;
                            player.path.clear();
                            println!("✅ 到达目的地");
                        }
                    } else {
                        if distance > 10.0 {
                            let target_dir = Self::calculate_direction(dx, dy);
                            player.direction = Self::smooth_direction(player.direction, target_dir);
                        }
                        pos.x += (dx / distance) * player.speed;
                        pos.y += (dy / distance) * player.speed;
                    }
                }
            }
            // 2. 直接跟随模式：直线移动到鼠标位置（带碰撞检测）
            else if player.move_mode == MoveMode::DirectFollow && player.is_moving {
                let dx = player.target_x - pos.x;
                let dy = player.target_y - pos.y;
                let distance = (dx * dx + dy * dy).sqrt();
                
                if distance < player.speed * 2.0 {
                    // 检查最终位置是否可行走
                    let (final_grid_x, final_grid_y) = MapHelper::world_to_grid(player.target_x, player.target_y);
                    if MapHelper::is_walkable(&map_data, final_grid_x, final_grid_y) {
                        pos.x = player.target_x;
                        pos.y = player.target_y;
                    } else {
                        // 目标不可行走，停止移动
                        player.is_moving = false;
                        player.action = PlayerAction::Stand;
                    }
                } else {
                    if distance > 10.0 {
                        let target_dir = Self::calculate_direction(dx, dy);
                        player.direction = Self::smooth_direction(player.direction, target_dir);
                    }
                    
                    // 计算下一步位置
                    let next_x = pos.x + (dx / distance) * player.speed;
                    let next_y = pos.y + (dy / distance) * player.speed;
                    
                    // 检查下一步是否可行走
                    let (next_grid_x, next_grid_y) = MapHelper::world_to_grid(next_x, next_y);
                    if MapHelper::is_walkable(&map_data, next_grid_x, next_grid_y) {
                        pos.x = next_x;
                        pos.y = next_y;
                    } else {
                        // 🎯 遇到障碍物，暂停移动但保持DirectFollow模式和当前动画
                        // 这样角色会继续播放走/跑动画（原地踏步效果）
                        player.is_moving = false;
                        // 不改变 action，保持走/跑动画
                        // 不改变 move_mode，保持 DirectFollow 状态
                    }
                }
            }
            
            // 🎯 更新动画帧（始终播放，即使碰到障碍物也保持动画）
            player.frame_time += 1;
            if player.frame_time >= player.action.frame_interval() {
                player.frame_time = 0;
                player.frame_index = (player.frame_index + 1) % player.action.frame_count();
            }
        }
        
        // 🔄 更新鼠标按下时间和清除双击事件
        if let Some((_, mouse_input)) = world.query_mut::<&mut MouseInput>().into_iter().next() {
            if mouse_input.left_pressed {
                mouse_input.left_press_time += 1;
            }
            if mouse_input.right_pressed {
                mouse_input.right_press_time += 1;
            }
            
            mouse_input.left_double_clicked = false;
            mouse_input.right_double_clicked = false;
        }
        
        // 📷 更新摄像机跟随玩家
        Self::update_camera_follow(world);
    }
    
    /// 摄像机跟随玩家
    pub fn update_camera_follow(world: &mut World) {
        let player_pos = world.query_mut::<(&Player, &Position)>()
            .into_iter()
            .next()
            .map(|(_, (_, pos))| (pos.x, pos.y));
        
        let Some((target_x, target_y)) = player_pos else { return };
        
        const CAMERA_SMOOTHING: f32 = 0.1;
        
        for (_entity, (camera_pos, _camera)) in world.query_mut::<(&mut Position, &Camera)>() {
            let dx = target_x - camera_pos.x;
            let dy = target_y - camera_pos.y;
            
            camera_pos.x += dx * CAMERA_SMOOTHING;
            camera_pos.y += dy * CAMERA_SMOOTHING;
        }
    }
}
