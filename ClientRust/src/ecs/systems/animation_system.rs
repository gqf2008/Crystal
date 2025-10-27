// ============================================================================
// Animation & Door Systems - 动画和门系统
// ============================================================================

use hecs::World;
use std::time::Instant;
use crate::ecs::components::{MapTile, AnimatedTile, Door, DoorState, Animation};

/// 动画系统 - 统一管理地图瓦片和实体动画
pub struct AnimationSystem;

impl AnimationSystem {
    /// 更新地图瓦片动画 (水流、火焰等)
    pub fn update_tiles(world: &mut World, animation_count: i32) {
        for (_entity, (tile, anim)) in world.query_mut::<(&mut MapTile, &AnimatedTile)>() {
            let total_frames = anim.frame_count as i32 + (anim.frame_count as i32 * anim.frame_interval as i32);
            let frame_offset = (animation_count % total_frames) / (1 + anim.frame_interval as i32);
            tile.image_index = anim.base_image_index + frame_offset;
        }
    }
    
    /// 更新实体动画 (Monster/NPC/Player等)
    /// 
    /// # 参数
    /// - world: ECS世界
    /// - delta_ms: 距上一帧的时间差(毫秒)
    pub fn update_entities(world: &mut World, delta_ms: u32) {
        for (_entity, anim) in world.query_mut::<&mut Animation>() {
            anim.update(delta_ms);
        }
    }
    
    /// 🎬 更新角色移动动画插值 - 实现原版C#的OffSetMove机制
    /// 
    /// 参考: Client/MirObjects/PlayerObject.cs Line 864-1000
    /// 
    /// 核心原理:
    /// - 根据动画帧进度计算offset_move
    /// - 更新movement_grid指向目标格子
    /// - Position在移动逻辑中逐帧更新
    pub fn update_movement_animation(world: &mut World) {
        use crate::ecs::components::{MovementAnimation, Player, Position, MoveMode, PlayerAction};
        use crate::ecs::Coordinates;
        
        for (_, (player, pos, movement_anim)) in world.query_mut::<(&Player, &Position, &mut MovementAnimation)>() {
            // 只有在移动时才更新插值
            if !player.is_moving {
                // 停止移动时清零偏移
                movement_anim.offset_move = (0.0, 0.0);
                movement_anim.move_distance = 0;
                // 同步current_grid和movement_grid到当前位置
                let (grid_x, grid_y) = Coordinates::world_to_grid(pos.x, pos.y);
                movement_anim.current_grid = (grid_x, grid_y);
                movement_anim.movement_grid = (grid_x, grid_y);
                continue;
            }
            
            // 获取当前动画帧信息
            let frame_count = player.action.frame_count();
            let frame_index = player.frame_index;
            
            // 计算移动距离（格子数）
            let move_distance = match player.action {
                PlayerAction::Walk => 1,
                PlayerAction::Run => 2,  // 原版跑步移动2格
                _ => 0,
            };
            
            // 如果有移动距离，更新offset_move和movement_grid
            if move_distance > 0 {
                movement_anim.move_distance = move_distance;
                movement_anim.update_offset(frame_index, frame_count, player.direction);
                
                // 🎯 根据移动模式更新movement_grid
                match player.move_mode {
                    MoveMode::AutoPathfinding => {
                        // 寻路模式：使用path中的目标格子
                        if player.path_index < player.path.len() {
                            let (target_grid_x, target_grid_y) = player.path[player.path_index];
                            movement_anim.movement_grid = (target_grid_x, target_grid_y);
                        }
                    }
                    MoveMode::DirectFollow => {
                        // 直接跟随模式：根据target位置计算目标格子
                        let (target_grid_x, target_grid_y) = Coordinates::world_to_grid(player.target_x, player.target_y);
                        movement_anim.movement_grid = (target_grid_x, target_grid_y);
                    }
                    MoveMode::Idle => {
                        // 空闲状态：movement_grid = current_grid
                        let (grid_x, grid_y) = Coordinates::world_to_grid(pos.x, pos.y);
                        movement_anim.movement_grid = (grid_x, grid_y);
                    }
                }
                
                // 更新current_grid为当前Position所在格子
                let (grid_x, grid_y) = Coordinates::world_to_grid(pos.x, pos.y);
                movement_anim.current_grid = (grid_x, grid_y);
            } else {
                movement_anim.offset_move = (0.0, 0.0);
                movement_anim.move_distance = 0;
            }
        }
    }
}

/// NPC动作系统 - 管理NPC动作切换
pub struct NPCActionSystem;

impl NPCActionSystem {
    /// 更新NPC动作,实现智能切换Standing/Harvest
    /// 
    /// # 特性
    /// - 动作权重: Standing 70%, Harvest 30%
    /// - 完整播放: 等待当前动画循环完成后再切换
    /// - 随机延迟: 3-8秒随机间隔,避免所有NPC同步
    /// 
    /// # 参数
    /// - world: ECS世界
    /// - delta_ms: 距上一帧的时间差(毫秒)
    pub fn update(world: &mut World, delta_ms: u32) {
        use crate::ecs::components::NPCData;
        use crate::objects::frames::{DEFAULT_NPC_FRAMES, get_frame};
        use mir2_shared::MirAction;
        use rand::Rng;
        
        for (_entity, (npc, anim)) in world.query_mut::<(&mut NPCData, &mut Animation)>() {
            // 只在Standing和Harvest之间切换
            if anim.action != MirAction::Standing && anim.action != MirAction::Harvest {
                continue;
            }
            
            // 累积计时器
            npc.action_timer += delta_ms;
            
            // 检查是否到达切换时间
            if npc.action_timer >= npc.next_action_delay {
                // 根据权重选择新动作: Standing 70%, Harvest 30%
                let roll = rand::rng().random_range(0..100);
                let new_action = if roll < 70 {
                    MirAction::Standing
                } else {
                    MirAction::Harvest
                };
                
                // 只有在真正需要切换动作时才处理
                if new_action != anim.action {
                    // 检查当前动画是否接近循环点(最后一帧) - 只有切换时才需要检查
                    let is_near_loop = anim.frame_index >= anim.frame_count.saturating_sub(1);
                    
                    if is_near_loop || anim.frame_count == 0 {
                        // 从FrameSet读取新动作的配置
                        if let Some(frame) = get_frame(&DEFAULT_NPC_FRAMES, new_action) {
                            tracing::debug!("🏪 NPC {} 切换动作: {:?} -> {:?}", npc.name, anim.action, new_action);
                            
                            anim.action = new_action;
                            anim.frame_count = frame.count as u8;
                            anim.frame_interval = frame.interval as u32;
                            anim.frame_index = 0;
                            anim.frame_timer = 0;
                        }
                    }
                }
                
                // 无论是否切换，都重置计时器以避免重复触发
                npc.action_timer = 0;
                npc.next_action_delay = rand::rng().random_range(3000..8000);
            }
        }
    }
}

/// 门系统
pub struct DoorSystem;

impl DoorSystem {
    pub fn update(world: &mut World) {
        for (_entity, (tile, door)) in world.query_mut::<(&mut MapTile, &mut Door)>() {
            match door.state {
                DoorState::Opening => {
                    if door.last_tick.elapsed().as_millis() > 100 {
                        door.current_frame += 1;
                        if door.current_frame >= 8 {
                            door.current_frame = 8;
                            door.state = DoorState::Open;
                        }
                        door.last_tick = Instant::now();
                    }
                }
                DoorState::Closing => {
                    if door.last_tick.elapsed().as_millis() > 100 {
                        door.current_frame -= 1;
                        if door.current_frame <= 0 {
                            door.current_frame = 0;
                            door.state = DoorState::Closed;
                        }
                        door.last_tick = Instant::now();
                    }
                }
                _ => {}
            }
            
            // 更新瓦片图像索引
            if door.current_frame > 0 {
                tile.image_index += (door.current_frame + 1) * door.door_offset;
            }
        }
    }
}
