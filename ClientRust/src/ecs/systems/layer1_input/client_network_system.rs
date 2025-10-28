// ============================================================================
// Client Network System - 客户端网络系统
// ============================================================================
//
// 职责（Layer 1: 输入与网络层）：
// 1. 发送：读取 PlayerInputComponent，序列化后发给服务器
// 2. 接收：接收服务器权威数据，写入 ServerStateComponent
//
// 不负责：
// - ❌ 游戏逻辑（由其他系统处理）
// - ❌ 预测和插值（由 Layer 2 处理）
//
// ============================================================================

use hecs::{World, Entity};
use std::collections::HashMap;
use tokio::sync::mpsc;

use crate::network::{NetworkCommand, game_client::GameEvent};
use crate::ecs::components::*;
use crate::ecs::Coordinates;
use mir2_shared::enums::MirDirection;

/// 客户端网络系统
pub struct ClientNetworkSystem {
    /// 对象ID -> Entity 映射表
    object_map: HashMap<u32, Entity>,
}

impl ClientNetworkSystem {
    pub fn new() -> Self {
        Self {
            object_map: HashMap::new(),
        }
    }
    
    /// 🎯 发送命令到服务器（读取 PlayerInputComponent）
    pub fn send_commands(
        world: &mut World,
        network_tx: Option<&mpsc::UnboundedSender<NetworkCommand>>,
    ) {
        let network_tx = match network_tx {
            Some(tx) => tx,
            None => return,
        };
        
        // 查找本地玩家
        for (_entity, (_, player_input, player)) in world.query_mut::<(
            &LocalPlayer,
            &PlayerInputComponent,
            &mut Player,
        )>() {
            // 1. 处理移动命令
            if let Some(move_to) = player_input.move_to {
                // 检查跑步限制
                let mut is_running = player_input.is_running;
                let now = std::time::Instant::now();
                
                if is_running {
                    if !player.can_run || now.duration_since(player.last_run_time) > player.run_cooldown {
                        is_running = false;
                        player.can_run = false;
                    }
                }
                
                // 发送移动命令到服务器
                // 注意：这里只发送命令，实际移动由 LocalPredictionSystem 立即执行
                let (target_grid_x, target_grid_y) = Coordinates::world_to_grid(move_to.0, move_to.1);
                
                tracing::debug!("🌐 发送移动命令: target=({}, {}) running={}", 
                    target_grid_x, target_grid_y, is_running);
                
                // TODO: 实现网络发送（需要定义 NetworkCommand::Walk 等）
                // let cmd = if is_running {
                //     NetworkCommand::Run { x: target_grid_x, y: target_grid_y }
                // } else {
                //     NetworkCommand::Walk { x: target_grid_x, y: target_grid_y }
                // };
                // let _ = network_tx.send(cmd);
            }
            
            // 2. 处理攻击命令
            if let Some(_target) = player_input.attack_target {
                // TODO: 实现攻击命令发送
                // let _ = network_tx.send(NetworkCommand::Attack { target_id: target });
            }
            
            // 3. 处理施法命令
            if let Some(_spell) = player_input.cast_spell {
                // TODO: 实现施法命令发送
                // let _ = network_tx.send(NetworkCommand::Magic { spell_id: spell.id, target: spell.target });
            }
        }
    }
    
    /// 🎯 处理服务器事件（写入 ServerStateComponent）
    pub fn process_event(&mut self, world: &mut World, event: &GameEvent) {
        match event {
            GameEvent::MapInformation { map_index, file_name, title } => {
                self.handle_map_information(world, *map_index, file_name, title);
            }
            GameEvent::ObjectSpawned { object } => {
                self.handle_object_spawned(world, object);
            }
            GameEvent::ObjectRemoved { object_id } => {
                self.handle_object_removed(world, *object_id);
            }
            GameEvent::PlayerMoved { location } => {
                // 🎯 服务器确认玩家移动
                self.handle_player_moved(world, location);
            }
            GameEvent::UserInformation { user_info } => {
                self.handle_user_information(world, user_info);
            }
            GameEvent::ObjectTurned { object_id, direction, location } => {
                self.handle_object_turned(world, *object_id, *direction, location);
            }
            GameEvent::ObjectWalked { object_id, direction, location } => {
                self.handle_object_moved(world, *object_id, *direction, location, MirAction::Walking);
            }
            GameEvent::ObjectRan { object_id, direction, location } => {
                self.handle_object_moved(world, *object_id, *direction, location, MirAction::Running);
            }
            _ => {
                // 其他事件委托给 UISystem
                use crate::ecs::systems::UISystem;
                UISystem::process_event(world, event);
            }
        }
    }
    
    /// 处理玩家移动事件 - 服务器权威位置
    fn handle_player_moved(&mut self, world: &mut World, location: &mir2_shared::Point) {
        tracing::info!("🌐 收到服务器位置确认: grid=({}, {})", location.x, location.y);
        
        // 查找本地玩家
        for (_entity, (_, player, server_state)) in world.query_mut::<(
            &LocalPlayer,
            &mut Player,
            Option<&mut ServerStateComponent>,
        )>() {
            // 将格子坐标转换为世界坐标
            let (world_x, world_y) = Coordinates::grid_to_world_center(location.x, location.y);
            let server_position = Position { x: world_x, y: world_y };
            
            // 写入服务器状态组件
            if let Some(server_state) = server_state {
                server_state.update(server_position, player.direction, 0);
            }
            
            // 解除等待服务器确认的标志
            if player.waiting_server_confirm {
                player.waiting_server_confirm = false;
                player.path_index += 1;
                
                tracing::info!("✅ 服务器确认，path_index 递增到 {}", player.path_index);
            }
        }
    }
    
    /// 处理其他对象移动（非本地玩家）
    fn handle_object_moved(
        &mut self,
        world: &mut World,
        object_id: u32,
        direction: MirDirection,
        location: &mir2_shared::Point,
        action: MirAction,
    ) {
        if let Some(&entity) = self.object_map.get(&object_id) {
            // 更新其他玩家/怪物的服务器状态和插值
            if let Ok((current_pos, mut server_state, mut interpolation)) = world.query_one_mut::<(
                &Position,
                Option<&mut ServerStateComponent>,
                Option<&mut InterpolationComponent>,
            )>(entity) {
                let (world_x, world_y) = Coordinates::grid_to_world_center(location.x, location.y);
                let server_position = Position { x: world_x, y: world_y };
                
                // 写入服务器状态
                if let Some(server_state) = server_state.as_deref_mut() {
                    server_state.update(server_position.clone(), direction as u8, 0);
                }
                
                // 启动插值（平滑移动）
                if let Some(interpolation) = interpolation.as_deref_mut() {
                    interpolation.start_interpolation(
                        current_pos.clone(),
                        server_position,
                        0.1, // 100ms插值
                    );
                }
            }
        }
    }
    
    // ============================================================================
    // 以下方法保持与原 NetworkSystem 兼容
    // ============================================================================
    
    fn handle_map_information(&mut self, world: &mut World, map_index: i32, file_name: &str, title: &str) {
        tracing::info!("🗺️ 地图信息: {} - {}", file_name, title);
        // TODO: 实现地图加载逻辑
        // 1. 调用 MapLoader::load(file_name)
        // 2. 更新 world 中的 MapData 组件
        // 3. 清理旧地图实体（瓦片、对象）
    }
    
    fn handle_object_spawned(&mut self, world: &mut World, object: &crate::network::game_client::GameObject) {
        // 根据GameObject类型提取对象名称
        let object_name = match object {
            crate::network::game_client::GameObject::Player { name, .. } => name,
            crate::network::game_client::GameObject::Monster { name, .. } => name,
            crate::network::game_client::GameObject::Npc { name, .. } => name,
            crate::network::game_client::GameObject::Item { .. } => "Item",
        };
        tracing::info!("👤 对象生成: {}", object_name);
        // TODO: 根据 GameObject 类型创建对应实体
        // match object {
        //     GameObject::Player { id, name, location, direction, .. } => {
        //         let entity = world.spawn((
        //             Position { x: location.x, y: location.y },
        //             Player { id: *id, name: name.clone(), direction: *direction, ... },
        //             ServerStateComponent::default(),
        //             InterpolationComponent::default(),
        //         ));
        //         self.object_map.insert(*id, entity);
        //     }
        //     GameObject::Monster { ... } => { ... }
        //     GameObject::Npc { ... } => { ... }
        //     GameObject::Item { ... } => { ... }
        // }
    }
    
    fn handle_object_removed(&mut self, world: &mut World, object_id: u32) {
        if let Some(entity) = self.object_map.remove(&object_id) {
            let _ = world.despawn(entity);
            tracing::info!("🗑️ 对象移除: {}", object_id);
        }
    }
    
    fn handle_user_information(&mut self, world: &mut World, user_info: &Box<mir2_shared::packets::server::UserInformation>) {
        tracing::info!("ℹ️ 用户信息: {}", user_info.name);
        // TODO: 更新本地玩家信息
        // for (entity, (_, mut player)) in world.query_mut::<(&LocalPlayer, &mut Player)>() {
        //     player.level = user_info.level;
        //     player.experience = user_info.experience;
        //     player.hp = user_info.hp;
        //     player.mp = user_info.mp;
        //     // ... 更新其他属性
        // }
    }
    
    fn handle_object_turned(
        &mut self,
        world: &mut World,
        object_id: u32,
        direction: MirDirection,
        location: &mir2_shared::Point,
    ) {
        if let Some(&entity) = self.object_map.get(&object_id) {
            if let Ok(mut player) = world.get::<&mut Player>(entity) {
                player.direction = direction as u8;
            }
        }
    }
}
