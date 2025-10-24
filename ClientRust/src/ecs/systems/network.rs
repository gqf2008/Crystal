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
        match event {
            GameEvent::ObjectSpawned { object } => {
                self.handle_object_spawned(world, object);
            }
            GameEvent::ObjectRemoved { object_id } => {
                self.handle_object_removed(world, *object_id);
            }
            GameEvent::PlayerMoved { location } => {
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
            _ => {
                // 其他事件由UISystem处理
            }
        }
        
        // 将事件传递给UISystem处理UI更新
        use crate::ecs::systems::UISystem;
        UISystem::process_event(world, event);
    }
    
    /// 处理玩家移动事件
    fn handle_player_moved(&mut self, world: &mut World, location: &mir2_shared::Point) {
        tracing::debug!("🚶 Player moved to: {:?}", location);
        
        // TODO: 这里需要知道是哪个玩家移动了
        // 可能需要从 UserInformation 中获取当前玩家的 object_id
        // 或者服务器发送 ObjectWalked/ObjectRan 事件包含 object_id
    }
    
    /// 处理用户信息更新事件
    fn handle_user_information(&mut self, world: &mut World, user_info: &mir2_shared::packets::server::UserInformation) {
        tracing::debug!("📊 User info updated - Object ID: {}", user_info.object_id);
        
        // 查找本地玩家实体（通过 LocalPlayer 标记）
        let mut query = world.query::<(&LocalPlayer, &mut Position, &mut Player, &mut PlayerComp)>();
        
        for (_entity, (_local_player, position, player, player_comp)) in query.iter() {
            // 更新位置（从格子坐标转换为世界坐标）
            let (world_x, world_y) = grid_to_world(user_info.location_x, user_info.location_y);
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
            GameObject::Player { id, name, location } => {
                self.create_other_player(world, *id, name, location)
            }
            GameObject::Npc { id, name, location } => {
                self.create_npc(world, *id, name, location)
            }
            GameObject::Monster { id, name, location, image, direction } => {
                self.create_monster(world, *id, name, location, *image, *direction)
            }
            GameObject::Item { .. } => {
                // TODO: 创建地面物品
                tracing::debug!("暂不处理地面物品");
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
    fn create_other_player(&self, world: &mut World, object_id: u32, name: &str, location: &mir2_shared::Point) -> Entity {
        let (world_x, world_y) = grid_to_world(location.x, location.y);
        
        world.spawn((
            Position::new(world_x, world_y),
            DirectionComp::new(MirDirection::Up),
            AnimationComp::new(MirAction::Standing, 4, 200),
            NetworkSync::new(object_id, NetworkObjectType::Player),
            OtherPlayer::new(
                name.to_string(),
                MirClass::Warrior, // TODO: 从对象数据获取
                MirGender::Male,   // TODO: 从对象数据获取
                1,                 // TODO: 从对象数据获取
            ),
            Health::new(100), // TODO: 从对象数据获取
        ))
    }

    /// 创建NPC实体
    fn create_npc(&self, world: &mut World, object_id: u32, name: &str, location: &mir2_shared::Point) -> Entity {
        let (world_x, world_y) = grid_to_world(location.x, location.y);
        
        world.spawn((
            Position::new(world_x, world_y),
            DirectionComp::new(MirDirection::Up),
            AnimationComp::new(MirAction::Standing, 4, 200),
            NetworkSync::new(object_id, NetworkObjectType::NPC),
            NPC::new(
                name.to_string(),
                "Unknown".to_string(), // TODO: 从对象数据获取NPC类型
            ),
        ))
    }

    /// 创建怪物实体
    fn create_monster(&self, world: &mut World, object_id: u32, name: &str, location: &mir2_shared::Point, image: u16, direction: MirDirection) -> Entity {
        let (world_x, world_y) = grid_to_world(location.x, location.y);
        
        tracing::info!("👹 创建怪物实体: ID={}, name={}, image={}, pos=({}, {})", object_id, name, image, world_x, world_y);
        
        world.spawn((
            Position::new(world_x, world_y),
            DirectionComp::new(direction),
            AnimationComp::new(MirAction::Standing, 4, 200),
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

    /// 处理对象转向事件
    fn handle_object_turned(&self, world: &mut World, object_id: u32, direction: mir2_shared::enums::MirDirection, _location: &mir2_shared::Point) {
        if let Some(&entity) = self.object_map.get(&object_id) {
            if let Ok(mut dir_comp) = world.get::<&mut DirectionComp>(entity) {
                dir_comp.current = direction;
                dir_comp.target = direction;
                tracing::debug!("🔄 Object {} turned to {:?}", object_id, direction);
            }
        }
    }
    
    /// 处理对象移动事件（走路或跑步）
    fn handle_object_moved(&self, world: &mut World, object_id: u32, direction: mir2_shared::enums::MirDirection, location: &mir2_shared::Point, action: MirAction) {
        if let Some(&entity) = self.object_map.get(&object_id) {
            let (world_x, world_y) = grid_to_world(location.x, location.y);
            
            // 分别获取各个组件
            if let Ok(mut pos) = world.get::<&mut Position>(entity) {
                pos.x = world_x;
                pos.y = world_y;
            }
            if let Ok(mut dir_comp) = world.get::<&mut DirectionComp>(entity) {
                dir_comp.current = direction;
                dir_comp.target = direction;
            }
            if let Ok(mut anim) = world.get::<&mut AnimationComp>(entity) {
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
            let (world_x, world_y) = grid_to_world(location.x, location.y);
            
            // 分别获取各个组件
            if let Ok(mut pos) = world.get::<&mut Position>(entity) {
                pos.x = world_x;
                pos.y = world_y;
            }
            if let Ok(mut dir_comp) = world.get::<&mut DirectionComp>(entity) {
                dir_comp.current = direction;
                dir_comp.target = direction;
            }
            if let Ok(mut anim) = world.get::<&mut AnimationComp>(entity) {
                anim.action = MirAction::Attack1;
            }
            
            tracing::debug!("⚔️ Object {} attacking at ({}, {}) facing {:?}", 
                object_id, location.x, location.y, direction);
        }
    }
    
    /// 处理对象被推动事件
    fn handle_object_pushed(&self, world: &mut World, object_id: u32, direction: mir2_shared::enums::MirDirection, location: &mir2_shared::Point) {
        if let Some(&entity) = self.object_map.get(&object_id) {
            let (world_x, world_y) = grid_to_world(location.x, location.y);
            
            // 分别获取各个组件
            if let Ok(mut pos) = world.get::<&mut Position>(entity) {
                pos.x = world_x;
                pos.y = world_y;
            }
            if let Ok(mut dir_comp) = world.get::<&mut DirectionComp>(entity) {
                dir_comp.current = direction;
                dir_comp.target = direction;
            }
            if let Ok(mut anim) = world.get::<&mut AnimationComp>(entity) {
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

/// 格子坐标转世界坐标
fn grid_to_world(grid_x: i32, grid_y: i32) -> (f32, f32) {
    (grid_x as f32 * 48.0, grid_y as f32 * 32.0)
}
