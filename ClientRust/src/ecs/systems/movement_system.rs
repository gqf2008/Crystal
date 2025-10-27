// ============================================================================
// Movement System - 移动系统
// ============================================================================
//
// 职责：
// - 移动状态机管理（Idle/DirectFollow/AutoPathfinding）
// - Position 更新和速度计算
// - 碰撞检测和避障
// - 移动方向计算
// - 网络移动命令发送
//
// ============================================================================

use hecs::World;
use tokio::sync::mpsc;
use crate::ecs::components::{
    Position, Player, MoveMode, PlayerAction, MouseInput, MapData, Camera,
};
use crate::ecs::{Coordinates, MapUtils};
use crate::network::NetworkCommand;
use mir2_shared::enums::MirDirection;

/// 移动系统
pub struct MovementSystem;

impl MovementSystem {
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
    
    /// 更新移动系统
    pub fn update(world: &mut World, network_tx: Option<&mpsc::UnboundedSender<NetworkCommand>>) {
        let map_data = world.query_mut::<&MapData>()
            .into_iter()
            .next()
            .map(|(_, data)| data.clone());
        
        let map_data = match map_data {
            Some(data) => data,
            None => return,
        };
        
        // 更新所有玩家的移动
        for (_entity, (player, pos)) in world.query_mut::<(&mut Player, &mut Position)>() {
            // 检查跑步冷却
            let now = std::time::Instant::now();
            if now.duration_since(player.last_run_time) > player.run_cooldown {
                if player.can_run {
                    tracing::info!("⏰ 跑步冷却超时，重置 can_run=false");
                    player.can_run = false;
                }
            }
            
            // 记录移动前的状态
            let (old_grid_x, old_grid_y) = Coordinates::world_to_grid(pos.x, pos.y);
            let old_direction = player.direction;
            
            // 处理自动寻路移动
            if player.move_mode == MoveMode::AutoPathfinding && !player.path.is_empty() {
                Self::update_pathfinding_movement(player, pos, &map_data, network_tx);
            }
            // 处理直接跟随移动
            else if player.move_mode == MoveMode::DirectFollow && player.is_moving {
                Self::update_direct_follow_movement(player, pos, &map_data);
            }
            
            // 更新动画帧
            Self::update_animation_frame(player);
            
            // 处理转身（不移动格子，只改变方向）
            Self::handle_turn(player, pos, old_grid_x, old_grid_y, old_direction, network_tx);
        }
    }
    
    /// 更新自动寻路移动
    fn update_pathfinding_movement(
        player: &mut Player,
        pos: &mut Position,
        map_data: &MapData,
        network_tx: Option<&mpsc::UnboundedSender<NetworkCommand>>,
    ) {
        let (current_grid_x, current_grid_y) = Coordinates::world_to_grid(pos.x, pos.y);
        
        // 如果正在等待服务器确认，完全停止Position更新
        if !player.waiting_server_confirm && player.path_index < player.path.len() {
            let (target_grid_x, target_grid_y) = player.path[player.path_index];
            let (target_x, target_y) = Coordinates::grid_to_world_center(target_grid_x, target_grid_y);
            
            let dx = target_x - pos.x;
            let dy = target_y - pos.y;
            let distance = (dx * dx + dy * dy).sqrt();
            
            let now = std::time::Instant::now();
            let time_since_last_move = now.duration_since(player.last_move_time).as_millis();
            
            println!("  🎯 目标[{}]: grid=({},{}) dist={:.1} time_since_last={}ms", 
                player.path_index, target_grid_x, target_grid_y, distance, time_since_last_move);
            
            // 只有距离足够大才移动（避免微小抖动）
            if distance > player.speed {
                // 检查目标格子是否可行走
                if !MapUtils::is_walkable(map_data, target_grid_x, target_grid_y) {
                    println!("🚫 [AutoPath] 目标格子 ({},{}) 不可达！停止移动", target_grid_x, target_grid_y);
                    player.is_moving = false;
                    player.move_mode = MoveMode::Idle;
                    player.action = PlayerAction::Stand;
                    player.collision_detected = true;
                    player.collision_target_grid = Some((target_grid_x, target_grid_y));
                } else {
                    // 逐帧移动Position（朝目标格子移动）
                    pos.x += (dx / distance) * player.speed;
                    pos.y += (dy / distance) * player.speed;
                    
                    // 平滑移动方向
                    if distance > 10.0 {
                        let target_dir = Self::calculate_direction(dx, dy);
                        player.direction = Self::smooth_direction(player.direction, target_dir);
                    }
                    
                    // 清除碰撞标记
                    player.collision_detected = false;
                    player.collision_target_grid = None;
                }
            } else {
                // 到达格子中心
                pos.x = target_x;
                pos.y = target_y;
                
                println!("✅ [到达] grid=({}, {}) pos=({:.1}, {:.1})", 
                    target_grid_x, target_grid_y, pos.x, pos.y);
                
                // 检查是否还有下一个格子
                let next_index = player.path_index + 1;
                if next_index >= player.path.len() {
                    println!("✅ [寻路] 到达路径终点: grid=({}, {})", target_grid_x, target_grid_y);
                    player.is_moving = false;
                    player.move_mode = MoveMode::Idle;
                    player.action = PlayerAction::Stand;
                    player.path_index = player.path.len();
                    return;
                }
                
                // 发送下一个格子的移动命令
                if let Some(network_tx) = network_tx {
                    let now = std::time::Instant::now();
                    let elapsed = now.duration_since(player.last_move_time);
                    
                    if elapsed >= player.move_delay {
                        let (next_target_x, next_target_y) = player.path[next_index];
                        
                        // 计算从当前格子到下一个格子的方向
                        let grid_dx = next_target_x as i32 - current_grid_x as i32;
                        let grid_dy = next_target_y as i32 - current_grid_y as i32;
                        
                        let direction = Self::grid_offset_to_direction(grid_dx, grid_dy);
                        
                        if let Some(dir) = direction {
                            match player.action {
                                PlayerAction::Run => {
                                    let _ = network_tx.send(NetworkCommand::Run { direction: dir });
                                    println!("🌐 [网络] 发送Run命令: current=({},{}) → next=({},{}) dir={:?}", 
                                        current_grid_x, current_grid_y, next_target_x, next_target_y, dir);
                                    player.last_run_time = now;
                                }
                                PlayerAction::Walk => {
                                    let _ = network_tx.send(NetworkCommand::Walk { direction: dir });
                                    println!("🌐 [网络] 发送Walk命令: current=({},{}) → next=({},{}) dir={:?}", 
                                        current_grid_x, current_grid_y, next_target_x, next_target_y, dir);
                                    player.can_run = true;
                                    player.last_run_time = now;
                                }
                                _ => {}
                            }
                            
                            player.last_move_time = now;
                            player.waiting_server_confirm = true;
                            println!("⏸️ [网络] 设置waiting_server_confirm=true，等待服务器确认");
                        } else {
                            // 路径错误，停止移动
                            tracing::error!("❌ 路径错误! current=({}, {}), next=({}, {}), offset=({}, {})",
                                current_grid_x, current_grid_y, next_target_x, next_target_y, grid_dx, grid_dy);
                            player.is_moving = false;
                            player.move_mode = MoveMode::Idle;
                            player.action = PlayerAction::Stand;
                        }
                    }
                } else {
                    // 离线模式：直接递增 path_index
                    player.path_index += 1;
                    println!("✅ [离线模式] 到达格子 ({},{})，递增 path_index 到 {}/{}",
                        target_grid_x, target_grid_y, player.path_index, player.path.len());
                }
            }
        } else if player.path_index >= player.path.len() {
            player.is_moving = false;
            player.move_mode = MoveMode::Idle;
            player.action = PlayerAction::Stand;
        }
    }
    
    /// 更新直接跟随移动
    fn update_direct_follow_movement(
        player: &mut Player,
        pos: &mut Position,
        map_data: &MapData,
    ) {
        let dx = player.target_x - pos.x;
        let dy = player.target_y - pos.y;
        let distance = (dx * dx + dy * dy).sqrt();
        
        if distance < player.speed * 2.0 {
            // 检查最终位置是否可行走
            let (final_grid_x, final_grid_y) = Coordinates::world_to_grid(player.target_x, player.target_y);
            if MapUtils::is_walkable(map_data, final_grid_x, final_grid_y) {
                pos.x = player.target_x;
                pos.y = player.target_y;
                player.is_moving = false;
                player.action = PlayerAction::Stand;
            } else {
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
            let (next_grid_x, next_grid_y) = Coordinates::world_to_grid(next_x, next_y);
            let (current_grid_x, current_grid_y) = Coordinates::world_to_grid(pos.x, pos.y);
            let crossed_grid = next_grid_x != current_grid_x || next_grid_y != current_grid_y;
            
            if MapUtils::is_walkable(map_data, next_grid_x, next_grid_y) {
                pos.x = next_x;
                pos.y = next_y;
                player.collision_detected = false;
                player.collision_target_grid = None;
            } else if crossed_grid {
                // 碰到障碍物，触发寻路
                if player.collision_target_grid != Some((next_grid_x, next_grid_y)) {
                    println!("🚫 [DirectFollow] 碰到障碍物: ({},{})，触发寻路避障", next_grid_x, next_grid_y);
                    
                    // 触发寻路系统找到绕行路径
                    // 这部分将由 PathfindingSystem 处理
                    player.collision_detected = true;
                    player.collision_target_grid = Some((next_grid_x, next_grid_y));
                }
            }
        }
    }
    
    /// 更新动画帧
    fn update_animation_frame(player: &mut Player) {
        player.frame_time += 1;
        if player.frame_time >= player.action.frame_interval() {
            player.frame_time = 0;
            player.frame_index = (player.frame_index + 1) % player.action.frame_count();
        }
    }
    
    /// 处理转身
    fn handle_turn(
        player: &mut Player,
        pos: &Position,
        old_grid_x: i32,
        old_grid_y: i32,
        old_direction: u8,
        network_tx: Option<&mpsc::UnboundedSender<NetworkCommand>>,
    ) {
        if let Some(network_tx) = network_tx {
            let (new_grid_x, new_grid_y) = Coordinates::world_to_grid(pos.x, pos.y);
            
            if player.direction != old_direction 
                && new_grid_x == old_grid_x 
                && new_grid_y == old_grid_y
                && !player.waiting_server_confirm {
                
                let now = std::time::Instant::now();
                let elapsed = now.duration_since(player.last_move_time);
                
                if elapsed >= player.move_delay {
                    let direction = Self::direction_to_mir_direction(player.direction);
                    let _ = network_tx.send(NetworkCommand::Turn { direction });
                    tracing::info!("🌐 发送转身命令: direction={:?}", direction);
                    player.last_move_time = now;
                }
            }
        }
    }
    
    /// 格子偏移转换为方向
    fn grid_offset_to_direction(dx: i32, dy: i32) -> Option<MirDirection> {
        match (dx, dy) {
            (0, -1) => Some(MirDirection::Up),
            (1, -1) => Some(MirDirection::UpRight),
            (1, 0) => Some(MirDirection::Right),
            (1, 1) => Some(MirDirection::DownRight),
            (0, 1) => Some(MirDirection::Down),
            (-1, 1) => Some(MirDirection::DownLeft),
            (-1, 0) => Some(MirDirection::Left),
            (-1, -1) => Some(MirDirection::UpLeft),
            _ => None,
        }
    }
    
    /// 方向值转换为MirDirection
    fn direction_to_mir_direction(direction: u8) -> MirDirection {
        match direction {
            0 => MirDirection::Up,
            1 => MirDirection::UpRight,
            2 => MirDirection::Right,
            3 => MirDirection::DownRight,
            4 => MirDirection::Down,
            5 => MirDirection::DownLeft,
            6 => MirDirection::Left,
            7 => MirDirection::UpLeft,
            _ => MirDirection::Down,
        }
    }
}
