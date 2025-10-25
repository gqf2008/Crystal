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
use tokio::sync::mpsc;

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

use crate::network::NetworkCommand;
use mir2_shared::enums::MirDirection;

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
    /// - 网络同步：发送移动命令到服务器
    pub fn update(world: &mut World, network_tx: Option<&mpsc::UnboundedSender<NetworkCommand>>) {
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
            // 记录移动前的位置（用于检测是否真的发生了移动）
            let old_grid_x = (pos.x / 48.0) as i32;
            let old_grid_y = (pos.y / 32.0) as i32;
            let old_direction = player.direction;
            
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
                
                tracing::info!("🖱️ 检测到双击! left={}, right={}, is_run={}", 
                    mouse_input.left_double_clicked, mouse_input.right_double_clicked, is_run);
                
                match player.move_mode {
                    MoveMode::Idle => {
                        // 空闲状态 → 双击触发寻路
                        let (start_grid_x, start_grid_y) = MapHelper::world_to_grid(pos.x, pos.y);
                        
                        // ✅ 使用统一的屏幕坐标转世界坐标算法
                        let (mouse_world_x, mouse_world_y) = PlayerSystem::screen_to_world(
                            mouse_input.x, 
                            mouse_input.y, 
                            &camera_pos, 
                            &camera
                        );
                        let (target_grid_x, target_grid_y) = MapHelper::world_to_grid(mouse_world_x, mouse_world_y);
                        
                        tracing::info!("📍 寻路: 起点=({}, {}), 目标=({}, {})", 
                            start_grid_x, start_grid_y, target_grid_x, target_grid_y);
                        
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
                            // 🎯 跳过路径的第一个点(起点),从第二个点开始移动
                            player.path_index = if player.path.len() > 1 { 1 } else { 0 };
                            player.is_moving = true;
                            player.action = if is_run { PlayerAction::Run } else { PlayerAction::Walk };
                            player.speed = if is_run { 1.6 } else { 1.33 };
                            player.move_mode = MoveMode::AutoPathfinding;
                            player.waiting_server_confirm = false;  // 🔓 清除等待标志
                            
                            tracing::info!("🗺️ 寻路成功: {} 个路径点 ({}), 起点=({}, {}), 第一个目标=({}, {}), path_index={}", 
                                player.path.len(), if is_run { "跑" } else { "走" },
                                player.path[0].0, player.path[0].1,
                                if player.path.len() > 1 { player.path[1].0 } else { player.path[0].0 },
                                if player.path.len() > 1 { player.path[1].1 } else { player.path[0].1 },
                                player.path_index);
                        } else {
                            tracing::warn!("❌ 寻路失败: 无法到达目标 ({}, {})", target_grid_x, target_grid_y);
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
                            player.path_index = if player.path.len() > 1 { 1 } else { 0 };
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
                            player.path_index = if player.path.len() > 1 { 1 } else { 0 };
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
            // 1. 自动寻路模式：沿路径移动（等待服务器确认）
            if player.move_mode == MoveMode::AutoPathfinding && !player.path.is_empty() {
                // 📊 调试日志 (降低频率到每3秒一次)
                static mut DEBUG_COUNTER: u32 = 0;
                unsafe {
                    DEBUG_COUNTER += 1;
                    if DEBUG_COUNTER % 180 == 0 {  // 60fps * 3秒 = 180帧
                        tracing::info!("🎯 寻路状态: path_index={}/{}, waiting={}, pos=({:.1}, {:.1})", 
                            player.path_index, player.path.len(), player.waiting_server_confirm, pos.x, pos.y);
                        if player.path_index < player.path.len() {
                            let (target_grid_x, target_grid_y) = player.path[player.path_index];
                            tracing::info!("   当前目标格子: ({}, {})", target_grid_x, target_grid_y);
                        }
                    }
                }
                
                // ⚠️ 如果正在等待服务器确认，完全停止Position更新
                if !player.waiting_server_confirm && player.path_index < player.path.len() {
                    let (target_grid_x, target_grid_y) = player.path[player.path_index];
                    let (target_x, target_y) = MapHelper::grid_to_world(target_grid_x, target_grid_y);
                    
                    let dx = target_x - pos.x;
                    let dy = target_y - pos.y;
                    let distance = (dx * dx + dy * dy).sqrt();
                    
                    // 🎯 只有距离足够大才移动（避免微小抖动）
                    if distance > player.speed {
                        // 平滑移动到目标格子中心
                        if distance > 10.0 {
                            let target_dir = Self::calculate_direction(dx, dy);
                            player.direction = Self::smooth_direction(player.direction, target_dir);
                        }
                        pos.x += (dx / distance) * player.speed;
                        pos.y += (dy / distance) * player.speed;
                    } else {
                        // ✅ 到达格子中心：锁定位置,立即发送命令
                        pos.x = target_x;
                        pos.y = target_y;
                        
                        // 🌐 立即发送移动命令并等待服务器确认
                        if let Some(network_tx) = network_tx {
                            let now = std::time::Instant::now();
                            let elapsed = now.duration_since(player.last_move_time);
                            
                            if elapsed >= player.move_delay {
                                let direction = match player.direction {
                                    0 => MirDirection::Up,
                                    1 => MirDirection::UpRight,
                                    2 => MirDirection::Right,
                                    3 => MirDirection::DownRight,
                                    4 => MirDirection::Down,
                                    5 => MirDirection::DownLeft,
                                    6 => MirDirection::Left,
                                    7 => MirDirection::UpLeft,
                                    _ => MirDirection::Down,
                                };
                                
                                match player.action {
                                    PlayerAction::Run => {
                                        let _ = network_tx.send(NetworkCommand::Run { direction });
                                        tracing::info!("🌐 到达格子中心,发送跑步命令: direction={:?}, target=({}, {})", 
                                            direction, target_grid_x, target_grid_y);
                                    }
                                    PlayerAction::Walk => {
                                        let _ = network_tx.send(NetworkCommand::Walk { direction });
                                        tracing::info!("🌐 到达格子中心,发送行走命令: direction={:?}, target=({}, {})", 
                                            direction, target_grid_x, target_grid_y);
                                    }
                                    _ => {}
                                }
                                
                                player.last_move_time = now;
                                player.waiting_server_confirm = true;
                            }
                        }
                    }
                } else {
                    // 🚫 路径已走完或等待确认
                    if player.path_index >= player.path.len() {
                        tracing::info!("✅ 路径完成: path_index={} >= path.len()={}, 停止移动", 
                            player.path_index, player.path.len());
                        player.is_moving = false;
                        player.move_mode = MoveMode::Idle;
                        player.action = PlayerAction::Stand;
                    }
                }
                // ✅ waiting_server_confirm=true 时：Position完全不变，只播放动画
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
            
            // 🔄 处理转身（不移动格子，只改变方向）
            if let Some(network_tx) = network_tx {
                let new_grid_x = (pos.x / 48.0) as i32;
                let new_grid_y = (pos.y / 32.0) as i32;
                
                if player.direction != old_direction 
                    && new_grid_x == old_grid_x 
                    && new_grid_y == old_grid_y
                    && !player.waiting_server_confirm {
                    
                    let now = std::time::Instant::now();
                    let elapsed = now.duration_since(player.last_move_time);
                    
                    if elapsed >= player.move_delay {
                        let direction = match player.direction {
                            0 => MirDirection::Up,
                            1 => MirDirection::UpRight,
                            2 => MirDirection::Right,
                            3 => MirDirection::DownRight,
                            4 => MirDirection::Down,
                            5 => MirDirection::DownLeft,
                            6 => MirDirection::Left,
                            7 => MirDirection::UpLeft,
                            _ => MirDirection::Down,
                        };
                        
                        let _ = network_tx.send(NetworkCommand::Turn { direction });
                        tracing::info!("🌐 发送转身命令: direction={:?}", direction);
                        player.last_move_time = now;
                    }
                } else {
                    tracing::debug!("⏸️ 位置未变化,不发送命令");
                }
            } else {
                tracing::warn!("⚠️ network_tx 是 None,无法发送网络命令!");
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
        
        // 🎯 相机直接跟随角色（居中）
        Self::update_camera_follow(world);
    }
    
    /// 摄像机直接跟随玩家（居中显示）
    pub fn update_camera_follow(world: &mut World) {
        let player_pos = world.query_mut::<(&Player, &Position)>()
            .into_iter()
            .next()
            .map(|(_, (_, pos))| (pos.x, pos.y));
        
        let Some((target_x, target_y)) = player_pos else { return };
        
        // 直接将相机位置设置为玩家位置（角色始终居中）
        for (_entity, (camera_pos, _camera)) in world.query_mut::<(&mut Position, &Camera)>() {
            camera_pos.x = target_x;
            camera_pos.y = target_y;
        }
    }
    
    /// 智能相机跟随：只在角色接近边缘或离开屏幕时才移动相机（已禁用）
    #[allow(dead_code)]
    pub fn update_smart_camera_follow(world: &mut World) {
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
            const EDGE_MARGIN: f32 = 300.0;  // 边缘安全距离（从200增加到300，提前触发）
            const STOP_THRESHOLD: f32 = 400.0; // 停止跟随阈值（从250增加到400，增大滞后区间）
            
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
                    // 在停止阈值内，使用较慢的速度（从0.03降低到0.02）
                    camera_pos.x += dx * 0.02;
                    camera_pos.y += dy * 0.02;
                } else {
                    // 距离较远时快速跟随（从0.15降低到0.08）
                    camera_pos.x += dx * 0.08;
                    camera_pos.y += dy * 0.08;
                }
            }
        }
    }
}
