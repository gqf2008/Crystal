// ============================================================================
// Network System - ECS网络同步系统
// ============================================================================
//
// 功能：
// - 将网络事件转换为ECS实体
// - 同步其他玩家、NPC、怪物的状态
// - 管理实体的生命周期（创建/更新/删除）
//
// ============================================================================

use hecs::{World, Entity};
use std::collections::HashMap;
use crate::network::game_client::GameEvent;
use crate::ecs::components::*;
use mir2_shared::enums::MirDirection;

/// 网络系统 - 管理网络同步的实体
pub struct NetworkSystem {
    /// 对象ID -> Entity 映射表
    object_map: HashMap<u32, Entity>,
}

impl NetworkSystem {
    pub fn new() -> Self {
        Self {
            object_map: HashMap::new(),
        }
    }

    /// 处理网络事件，更新World中的实体
    pub fn process_event(&mut self, world: &mut World, event: &GameEvent) {
        tracing::debug!("🔄 NetworkSystem::process_event - 事件: {:?}", event);
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
                tracing::info!("🚶 收到 PlayerMoved 事件: {:?}", location);
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
            GameEvent::ObjectAttacked { object_id, direction, location, spell: _ } => {
                self.handle_object_attacked(world, *object_id, *direction, location);
            }
            GameEvent::ObjectPushed { object_id, direction, location } => {
                self.handle_object_pushed(world, *object_id, *direction, location);
            }
            GameEvent::ItemSpawned { object_id, item, location } => {
                self.handle_item_spawned(world, *object_id, item, location);
            }
            _ => {
                // 其他事件由UISystem处理
            }
        }
        
        // 将事件传递给UISystem处理UI更新
        use crate::ecs::systems::UISystem;
        UISystem::process_event(world, event);
    }
    
    /// 处理玩家移动事件 - UserLocation 包确认位置
    fn handle_player_moved(&mut self, world: &mut World, location: &mir2_shared::Point) {
        tracing::info!("🎯 handle_player_moved 被调用: grid=({}, {})", location.x, location.y);
        
        // 查找本地玩家实体
        let mut player_entity = None;
        let mut should_sync = false;
        
        {
            let mut query = world.query::<(&LocalPlayer, &mut Position, &mut Player)>();
            
            for (entity, (_local, position, player)) in query.iter() {
                tracing::info!("🔍 找到本地玩家实体: entity={:?}", entity);
                // 将格子坐标转换为世界坐标 (格子中心)
                let (world_x, world_y) = crate::ecs::coordinates::Coordinates::grid_to_world_center(location.x, location.y);
                
                // 🎯 使用统一的坐标转换函数 (避免 round vs floor 的差异)
                let (current_grid_x, current_grid_y) = crate::ecs::coordinates::Coordinates::world_to_grid(position.x, position.y);
                
                tracing::info!("📊 位置对比: 客户端=({}, {}) world=({:.1}, {:.1}), 服务器=({}, {}) world=({:.1}, {:.1})",
                    current_grid_x, current_grid_y, position.x, position.y,
                    location.x, location.y, world_x, world_y);
                
                // 检查格子位置偏差
                let grid_diff_x = (current_grid_x - location.x).abs();
                let grid_diff_y = (current_grid_y - location.y).abs();
                
                // 🎯 清除等待服务器确认标志
                player.waiting_server_confirm = false;
                
                if grid_diff_x > 0 || grid_diff_y > 0 {
                    // 🎯 格子位置不同 - 同步到服务器位置
                    tracing::info!("✅ 同步到服务器格子: ({}, {}) -> ({}, {})", 
                        current_grid_x, current_grid_y, location.x, location.y);
                    
                    position.x = world_x;
                    position.y = world_y;
                    player.target_x = world_x;
                    player.target_y = world_y;
                    
                    // 🎯 如果在自动寻路模式,更新路径索引到服务器位置
                    if player.move_mode == crate::ecs::components::MoveMode::AutoPathfinding 
                        && !player.path.is_empty() {
                        
                        // 查找服务器位置在路径中的索引
                        let mut found_index = None;
                        for (i, &(path_x, path_y)) in player.path.iter().enumerate() {
                            if path_x == location.x && path_y == location.y {
                                found_index = Some(i);
                                break;
                            }
                        }
                        
                        if let Some(index) = found_index {
                            // 找到了,设置到下一个waypoint
                            player.path_index = index + 1;
                            tracing::info!("🎯 路径同步: 服务器在索引 {}, 下一个目标索引 {}", index, player.path_index);
                            
                            if player.path_index >= player.path.len() {
                                // 到达终点
                                player.move_mode = crate::ecs::components::MoveMode::Idle;
                                player.is_moving = false;
                                tracing::info!("✅ 到达目的地");
                            }
                        } else {
                            // 服务器位置不在路径上,可能是旧位置或有偏差
                            // 如果偏差很大(>2格),清除移动状态
                            if grid_diff_x > 2 || grid_diff_y > 2 {
                                tracing::warn!("⚠️ 位置偏差过大! 客户端:({}, {}) 服务器:({}, {}) - 停止移动", 
                                    current_grid_x, current_grid_y, location.x, location.y);
                                player.move_mode = crate::ecs::components::MoveMode::Idle;
                                player.is_moving = false;
                                player.path.clear();
                                player.path_index = 0;
                            } else {
                                // 偏差不大,可能是网络延迟,保持当前路径继续
                                tracing::debug!("📍 服务器位置不在路径上,但偏差可接受,继续移动");
                            }
                        }
                    }
                } else {
                    // ✅ 同一个格子 - 服务器确认位置,允许客户端继续
                    tracing::debug!("✅ 服务器确认当前格子: ({}, {}) - 保持客户端插值", location.x, location.y);
                }
                
                player_entity = Some(entity);
                should_sync = true;
                break;
            }
        }
        
        // 🔒 添加 NetworkSync 组件,标记已收到服务器位置
        if let Some(entity) = player_entity {
            if should_sync {
                // 检查是否已有 NetworkSync 组件
                let has_sync = world.get::<&crate::ecs::components::NetworkSync>(entity).is_ok();
                
                if !has_sync {
                    // 首次收到服务器位置,添加 NetworkSync 组件
                    if let Err(e) = world.insert_one(entity, crate::ecs::components::NetworkSync {
                        object_id: 0, // 本地玩家的 object_id
                        last_update: std::time::Instant::now(),
                        object_type: crate::ecs::components::NetworkObjectType::Player,
                    }) {
                        tracing::error!("❌ 添加 NetworkSync 组件失败: {:?}", e);
                    } else {
                        tracing::info!("🔒 已添加 NetworkSync 组件 - 玩家纹理现在可以渲染");
                    }
                } else {
                    // 更新现有的 NetworkSync 组件
                    if let Ok(mut sync) = world.get::<&mut crate::ecs::components::NetworkSync>(entity) {
                        sync.last_update = std::time::Instant::now();
                    }
                }
            }
        }
    }
    
    /// 处理地图信息事件 - 从服务器加载地图
    fn handle_map_information(&mut self, world: &mut World, map_index: i32, file_name: &str, title: &str) {
        tracing::info!("🗺️ 收到地图信息: {} ({}) - 索引: {}", title, file_name, map_index);
        
        // 构建地图文件路径
        let map_path = format!("Map/{}.map", file_name);
        
        tracing::info!("📂 正在加载地图文件: {}", map_path);
        
        // 加载地图数据
        use crate::objects::MapReader;
        use crate::ecs::map_loader::MapLoader;
        
        match MapReader::new(&map_path) {
            Ok(reader) => {
                tracing::info!("✅ 地图文件加载成功: {}x{}", reader.width, reader.height);
                
                // 清除旧的地图瓦片
                // TODO: 实现清除旧地图瓦片的逻辑
                
                // 加载新地图瓦片到 ECS
                match MapLoader::load_map(world, reader) {
                    Ok(_) => {
                        tracing::info!("✅ 地图数据已加载到 ECS");
                    }
                    Err(e) => {
                        tracing::error!("❌ 地图数据加载到 ECS 失败: {}", e);
                    }
                }
            }
            Err(e) => {
                tracing::error!("❌ 地图文件加载失败: {} - {}", map_path, e);
            }
        }
    }
    
    /// 处理用户信息更新事件
    fn handle_user_information(&mut self, world: &mut World, user_info: &mir2_shared::packets::server::UserInformation) {
        tracing::debug!("📊 User info updated - Object ID: {}", user_info.object_id);
        
        // 查找本地玩家实体（通过 LocalPlayer 标记）
        let mut query = world.query::<(&LocalPlayer, &mut Position, &mut Player, &mut PlayerData)>();
        
        for (_entity, (_local_player, position, player, player_comp)) in query.iter() {
            // 更新位置（从格子坐标转换为世界坐标）
            let (world_x, world_y) = crate::ecs::coordinates::Coordinates::grid_to_world_center(user_info.location_x, user_info.location_y);
            position.x = world_x;
            position.y = world_y;
            
            // 更新玩家目标位置（防止寻路到错误位置）
            player.target_x = world_x;
            player.target_y = world_y;
            
            // 更新玩家信息
            player_comp.id = user_info.object_id;
            player_comp.name = user_info.name.clone();
            player_comp.gold = user_info.gold;
            player_comp.class = user_info.class;
            player_comp.exp = user_info.experience; // ✅ i64类型，直接赋值
            
            tracing::info!("✅ Updated local player state: pos=({}, {}) grid=({}, {}), hp={}, gold={}", 
                world_x, world_y, user_info.location_x, user_info.location_y,
                user_info.hp, user_info.gold);
            
            println!("✅ 本地玩家位置已更新: 格子({}, {}) -> 世界({:.1}, {:.1})", 
                user_info.location_x, user_info.location_y, world_x, world_y);
            break;
        }
    }

    /// 处理对象生成事件
    fn handle_object_spawned(&mut self, world: &mut World, object: &crate::network::game_client::GameObject) {
        use crate::network::game_client::GameObject;

        let (object_id, object_type) = match object {
            GameObject::Player { id, .. } => (*id, "Player"),
            GameObject::Monster { id, .. } => (*id, "Monster"),
            GameObject::Npc { id, .. } => (*id, "Npc"),
            GameObject::Item { id, .. } => (*id, "Item"),
        };

        tracing::info!("🌟 生成网络对象: ID={}, 类型={}", object_id, object_type);

        // 检查是否已存在
        if self.object_map.contains_key(&object_id) {
            tracing::warn!("⚠️ 对象已存在: ID={}", object_id);
            return;
        }

        // 根据对象类型创建实体
        let entity = match object {
            GameObject::Player { id, name, location, class, gender, level, direction, .. } => {
                self.create_other_player(world, *id, name, location, *class, *gender, *level, *direction)
            }
            GameObject::Npc { id, name, location, image, direction } => {
                self.create_npc(world, *id, name, location, *image, *direction)
            }
            GameObject::Monster { id, name, location, image, direction, .. } => {
                self.create_monster(world, *id, name, location, *image, *direction)
            }
            GameObject::Item { .. } => {
                // Item通过ItemSpawned事件处理,不在这里创建
                tracing::debug!("Item对象由ItemSpawned事件处理");
                return;
            }
        };

        // 记录映射
        self.object_map.insert(object_id, entity);
    }

    /// 处理对象移除事件
    fn handle_object_removed(&mut self, world: &mut World, object_id: u32) {
        if let Some(entity) = self.object_map.remove(&object_id) {
            tracing::info!("🗑️ 移除网络对象: ID={}", object_id);
            if let Err(e) = world.despawn(entity) {
                tracing::warn!("⚠️ 删除实体失败: {:?}", e);
            }
        } else {
            tracing::warn!("⚠️ 尝试删除不存在的对象: ID={}", object_id);
        }
    }

    /// 创建其他玩家实体
    fn create_other_player(
        &self, 
        world: &mut World, 
        object_id: u32, 
        name: &str, 
        location: &mir2_shared::Point,
        class: MirClass,
        gender: MirGender,
        level: u16,
        direction: MirDirection,
    ) -> Entity {
        let (world_x, world_y) = crate::ecs::coordinates::Coordinates::grid_to_world_center(location.x, location.y);
        
        world.spawn((
            Position::new(world_x, world_y),
            Direction::new(direction),
            Animation::new(MirAction::Standing, 4, 200),
            NetworkSync::new(object_id, NetworkObjectType::Player),
            OtherPlayer::new(
                name.to_string(),
                class,
                gender,
                level,
            ),
            Health::new(100), // TODO: 需要从ObjectHealth packet获取
        ))
    }

    /// 创建NPC实体
    fn create_npc(
        &self, 
        world: &mut World, 
        object_id: u32, 
        name: &str, 
        location: &mir2_shared::Point,
        image: u16,
        direction: MirDirection,
    ) -> Entity {
        let (world_x, world_y) = crate::ecs::coordinates::Coordinates::grid_to_world_center(location.x, location.y);
        
        world.spawn((
            Position::new(world_x, world_y),
            Direction::new(direction),
            Animation::new(MirAction::Standing, 4, 200),
            NetworkSync::new(object_id, NetworkObjectType::NPC),
            NPC::new(
                name.to_string(),
                format!("NPC#{}", image), // 使用image作为NPC类型标识
            ),
        ))
    }

    /// 创建怪物实体
    fn create_monster(&self, world: &mut World, object_id: u32, name: &str, location: &mir2_shared::Point, image: u16, direction: MirDirection) -> Entity {
        let (world_x, world_y) = crate::ecs::coordinates::Coordinates::grid_to_world_center(location.x, location.y);
        
        tracing::info!("👹 创建怪物实体: ID={}, name={}, image={}, pos=({}, {})", object_id, name, image, world_x, world_y);
        
        world.spawn((
            Position::new(world_x, world_y),
            Direction::new(direction),
            Animation::new(MirAction::Standing, 4, 200),
            NetworkSync::new(object_id, NetworkObjectType::Monster),
            Monster::new(
                name.to_string(),
                image, // 使用服务器传来的怪物图像索引
            ),
            Health::new(100), // TODO: 从对象数据获取
            CombatStats {
                level: 1,
                attack_min: 1,
                attack_max: 5,
                defense: 1,
                magic_defense: 0,
                accuracy: 10,
                agility: 10,
            },
        ))
    }

    /// 处理地面物品生成事件
    fn handle_item_spawned(&mut self, world: &mut World, object_id: u32, item: &mir2_shared::UserItem, location: &mir2_shared::Point) {
        let (world_x, world_y) = crate::ecs::coordinates::Coordinates::grid_to_world_center(location.x, location.y);
        
        let item_name = item.info.as_ref()
            .map(|i| i.name.as_str())
            .unwrap_or("Unknown");
        
        tracing::info!("💎 创建地面物品: ID={}, name={}, pos=({}, {})", object_id, item_name, world_x, world_y);
        
        // 创建地面物品实体
        let entity = world.spawn((
            Position::new(world_x, world_y),
            NetworkSync::new(object_id, NetworkObjectType::Item),
            // TODO: 添加 ItemDrop 组件用于渲染物品图标
            // ItemDrop { item: item.clone() },
        ));
        
        // 记录映射
        self.object_map.insert(object_id, entity);
    }

    /// 处理对象转向事件
    fn handle_object_turned(&self, world: &mut World, object_id: u32, direction: mir2_shared::enums::MirDirection, _location: &mir2_shared::Point) {
        if let Some(&entity) = self.object_map.get(&object_id) {
            if let Ok(mut dir_comp) = world.get::<&mut Direction>(entity) {
                dir_comp.current = direction;
                dir_comp.target = direction;
                tracing::debug!("🔄 Object {} turned to {:?}", object_id, direction);
            }
        }
    }
    
    /// 处理对象移动事件（走路或跑步）
    fn handle_object_moved(&self, world: &mut World, object_id: u32, direction: mir2_shared::enums::MirDirection, location: &mir2_shared::Point, action: MirAction) {
        if let Some(&entity) = self.object_map.get(&object_id) {
            let (world_x, world_y) = crate::ecs::coordinates::Coordinates::grid_to_world_center(location.x, location.y);
            
            // 分别获取各个组件
            if let Ok(mut pos) = world.get::<&mut Position>(entity) {
                pos.x = world_x;
                pos.y = world_y;
            }
            if let Ok(mut dir_comp) = world.get::<&mut Direction>(entity) {
                dir_comp.current = direction;
                dir_comp.target = direction;
            }
            if let Ok(mut anim) = world.get::<&mut Animation>(entity) {
                anim.action = action;
            }
            
            let action_str = if action == MirAction::Walking { "walking" } else { "running" };
            tracing::debug!("🚶 Object {} {} to ({}, {}) facing {:?}", 
                object_id, action_str, location.x, location.y, direction);
        }
    }
    
    /// 处理对象攻击事件
    fn handle_object_attacked(&self, world: &mut World, object_id: u32, direction: mir2_shared::enums::MirDirection, location: &mir2_shared::Point) {
        if let Some(&entity) = self.object_map.get(&object_id) {
            let (world_x, world_y) = crate::ecs::coordinates::Coordinates::grid_to_world_center(location.x, location.y);
            
            // 分别获取各个组件
            if let Ok(mut pos) = world.get::<&mut Position>(entity) {
                pos.x = world_x;
                pos.y = world_y;
            }
            if let Ok(mut dir_comp) = world.get::<&mut Direction>(entity) {
                dir_comp.current = direction;
                dir_comp.target = direction;
            }
            if let Ok(mut anim) = world.get::<&mut Animation>(entity) {
                anim.action = MirAction::Attack1;
            }
            
            tracing::debug!("⚔️ Object {} attacking at ({}, {}) facing {:?}", 
                object_id, location.x, location.y, direction);
        }
    }
    
    /// 处理对象被推动事件
    fn handle_object_pushed(&self, world: &mut World, object_id: u32, direction: mir2_shared::enums::MirDirection, location: &mir2_shared::Point) {
        if let Some(&entity) = self.object_map.get(&object_id) {
            let (world_x, world_y) = crate::ecs::coordinates::Coordinates::grid_to_world_center(location.x, location.y);
            
            // 分别获取各个组件
            if let Ok(mut pos) = world.get::<&mut Position>(entity) {
                pos.x = world_x;
                pos.y = world_y;
            }
            if let Ok(mut dir_comp) = world.get::<&mut Direction>(entity) {
                dir_comp.current = direction;
                dir_comp.target = direction;
            }
            if let Ok(mut anim) = world.get::<&mut Animation>(entity) {
                anim.action = MirAction::Pushed;
            }
            
            tracing::debug!("💨 Object {} pushed to ({}, {})", 
                object_id, location.x, location.y);
        }
    }

    /// 获取对象ID对应的实体
    pub fn get_entity(&self, object_id: u32) -> Option<Entity> {
        self.object_map.get(&object_id).copied()
    }

    /// 清除所有网络对象
    pub fn clear(&mut self, world: &mut World) {
        for (_, entity) in self.object_map.drain() {
            let _ = world.despawn(entity);
        }
    }
}

impl Default for NetworkSystem {
    fn default() -> Self {
        Self::new()
    }
}

// ============================================================================
// 辅助函数
// ============================================================================

// ❌ 已删除重复函数 - 使用 CoordinateSystem::grid_to_world_center() 替代



