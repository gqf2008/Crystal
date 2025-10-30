// GameEventSystem - 处理网络游戏事件并同步到 ECS World
// 
// 职责:
// 1. 从 GlobalEvents.game_events 读取事件
// 2. 根据事件类型创建/更新 ECS 实体和组件
// 3. 替代 GameClient 的游戏状态管理功能
//
// 优先级: Stage 5 (State Update) - 500 系列
// 在网络同步系统之后执行,在渲染之前执行

use ggez::GameResult;
use hecs::World;

use crate::ecs::components::{
    GlobalEvents, Position, Sprite, Health, AnimationState,
    PlayerData, MonsterData, NPCData, ItemDrop,
    NetworkSync, NetworkObjectType,
};
use crate::ecs::systems::System;
use crate::network::GameEvent;
use mir2_shared::enums::*;

pub struct GameEventSystem {
    /// 事件计数统计
    events_processed: u64,
    
    /// 启用调试日志
    debug_logging: bool,
}

impl GameEventSystem {
    pub fn new() -> Self {
        Self {
            events_processed: 0,
            debug_logging: false,
        }
    }
    
    pub fn with_logging(mut self, enabled: bool) -> Self {
        self.debug_logging = enabled;
        self
    }
    
    /// 处理 ObjectSpawned 事件 - 生成游戏对象实体
    fn handle_object_spawned(&mut self, world: &mut World, object: crate::network::GameObject) {
        use crate::network::GameObject;
        
        match object {
            GameObject::Player { id, name, location, class, gender, level, direction, hair, weapon, armour, dead, hidden } => {
                if self.debug_logging {
                    tracing::debug!("👤 Spawning player entity: {} (id={}, level={})", name, id, level);
                }
                
                // 创建玩家实体
                world.spawn((
                    NetworkSync::new(id, NetworkObjectType::Player),
                    Position { 
                        x: location.x as f32, 
                        y: location.y as f32 
                    },
                    PlayerData {
                        name: name.clone(),
                        level,
                        class,
                        gender,
                        direction,
                        hair,
                        weapon,
                        armour,
                    },
                    Sprite {
                        image_index: Self::get_player_sprite_index(class, gender),
                        frame: 0,
                        visible: !hidden,
                    },
                    Health {
                        current: if dead { 0 } else { 100 }, // 默认值,等待 UserInformation 更新
                        max: 100,
                    },
                    AnimationState::Idle,
                ));
            }
            
            GameObject::Monster { id, name, location, image, direction, dead, hidden } => {
                if self.debug_logging {
                    tracing::debug!("👹 Spawning monster entity: {} (id={}, image={})", name, id, image);
                }
                
                // 创建怪物实体
                world.spawn((
                    NetworkSync::new(id, NetworkObjectType::Monster),
                    Position { 
                        x: location.x as f32, 
                        y: location.y as f32 
                    },
                    MonsterData {
                        id,
                        name: name.clone(),
                        image,
                        ai_type: 0, // TODO: 从服务器获取
                    },
                    Sprite {
                        image_index: image,
                        frame: 0,
                        visible: !hidden,
                    },
                    Health {
                        current: if dead { 0 } else { 100 },
                        max: 100,
                    },
                    AnimationState::Idle,
                ));
            }
            
            GameObject::Npc { id, name, location, image, colour, direction } => {
                if self.debug_logging {
                    tracing::debug!("🏪 Spawning NPC entity: {} (id={}, image={})", name, id, image);
                }
                
                // 创建 NPC 实体
                world.spawn((
                    NetworkSync::new(id, NetworkObjectType::Npc),
                    Position { 
                        x: location.x as f32, 
                        y: location.y as f32 
                    },
                    NPCData {
                        id,
                        name: name.clone(),
                        image,
                        npc_index: 0, // TODO: 从服务器获取
                    },
                    Sprite {
                        image_index: image,
                        frame: 0,
                        visible: true,
                    },
                    AnimationState::Idle,
                ));
            }
            
            GameObject::Item { id, location, item } => {
                if self.debug_logging {
                    tracing::debug!("📦 Spawning item entity: id={}, location=({}, {})", id, location.x, location.y);
                }
                
                // 创建地面物品实体
                world.spawn((
                    NetworkSync::new(id, NetworkObjectType::Item),
                    Position { 
                        x: location.x as f32, 
                        y: location.y as f32 
                    },
                    ItemDrop {
                        item_id: item.unique_id as u32,
                        item_index: item.index,
                        count: item.count as u32,
                        owner_id: None,
                    },
                    Sprite {
                        image_index: item.index,
                        frame: 0,
                        visible: true,
                    },
                ));
            }
        }
    }
    
    /// 处理 ObjectRemoved 事件 - 移除游戏对象
    fn handle_object_removed(&mut self, world: &mut World, object_id: u32) {
        if self.debug_logging {
            tracing::debug!("🗑️ Removing object: id={}", object_id);
        }
        
        // 查找并删除对应的实体
        let mut to_remove = None;
        for (entity, network_sync) in world.query_mut::<&NetworkSync>() {
            if network_sync.object_id == object_id {
                to_remove = Some(entity);
                break;
            }
        }
        
        if let Some(entity) = to_remove {
            if let Err(e) = world.despawn(entity) {
                tracing::error!("Failed to despawn entity for object_id={}: {:?}", object_id, e);
            }
        } else {
            tracing::warn!("⚠️ ObjectRemoved: entity not found for object_id={}", object_id);
        }
    }
    
    /// 处理 ObjectTurned 事件 - 更新对象朝向
    fn handle_object_turned(&mut self, world: &mut World, object_id: u32, direction: MirDirection, location: mir2_shared::Point) {
        for (_, (network_sync, pos, player_data)) in world.query_mut::<(&NetworkSync, &mut Position, Option<&mut PlayerData>)>() {
            if network_sync.object_id == object_id {
                pos.x = location.x as f32;
                pos.y = location.y as f32;
                
                if let Some(player) = player_data {
                    player.direction = direction;
                }
                return;
            }
        }
    }
    
    /// 处理 ObjectWalked/ObjectRan 事件 - 更新对象移动
    fn handle_object_moved(&mut self, world: &mut World, object_id: u32, _direction: MirDirection, location: mir2_shared::Point) {
        for (_, (network_sync, pos)) in world.query_mut::<(&NetworkSync, &mut Position)>() {
            if network_sync.object_id == object_id {
                pos.x = location.x as f32;
                pos.y = location.y as f32;
                return;
            }
        }
    }
    
    /// 处理 ObjectHealthChanged 事件 - 更新对象血量
    fn handle_object_health_changed(&mut self, world: &mut World, object_id: u32, percent: u8) {
        for (_, (network_sync, health)) in world.query_mut::<(&NetworkSync, &mut Health)>() {
            if network_sync.object_id == object_id {
                health.current = (health.max * percent as u32) / 100;
                return;
            }
        }
    }
    
    /// 处理 MapChanged 事件 - 切换地图
    fn handle_map_changed(&mut self, world: &mut World, file_name: String, location: mir2_shared::Point) {
        if self.debug_logging {
            tracing::info!("🗺️ Map changed: {} at ({}, {})", file_name, location.x, location.y);
        }
        
        // 清理旧地图上的所有实体 (除了 GlobalEvents 和 Camera)
        let mut entities_to_remove = Vec::new();
        for (entity, _) in world.query_mut::<&NetworkSync>() {
            entities_to_remove.push(entity);
        }
        
        for entity in entities_to_remove {
            let _ = world.despawn(entity);
        }
        
        // 更新地图信息 (假设有 MapInfo 组件)
        // TODO: 需要创建或更新 MapInfo 组件
        tracing::info!("✅ Map cleared, ready to load: {}", file_name);
    }
    
    /// 获取玩家精灵图像索引 (根据职业和性别)
    fn get_player_sprite_index(class: MirClass, gender: MirGender) -> u16 {
        // 简化版本 - 实际应该从资源配置中读取
        match (class, gender) {
            (MirClass::Warrior, MirGender::Male) => 0,
            (MirClass::Warrior, MirGender::Female) => 1,
            (MirClass::Wizard, MirGender::Male) => 2,
            (MirClass::Wizard, MirGender::Female) => 3,
            (MirClass::Taoist, MirGender::Male) => 4,
            (MirClass::Taoist, MirGender::Female) => 5,
            _ => 0,
        }
    }
}

impl System for GameEventSystem {
    fn update(&mut self, world: &mut World, _delay_time: f32) -> GameResult {
        // 1. 获取全局事件组件
        let game_events = {
            let mut events_to_process = Vec::new();
            
            for (_, global_events) in world.query_mut::<&mut GlobalEvents>() {
                // 取出所有游戏事件 (drain 会清空原队列)
                events_to_process = global_events.game_events.drain(..).collect();
                break;
            }
            
            events_to_process
        };
        
        // 2. 处理所有事件
        for event in game_events {
            self.events_processed += 1;
            
            match event {
                // ============================================================
                // 对象生成/移除
                // ============================================================
                GameEvent::ObjectSpawned { object } => {
                    self.handle_object_spawned(world, object);
                }
                
                GameEvent::ObjectRemoved { object_id } => {
                    self.handle_object_removed(world, object_id);
                }
                
                // ============================================================
                // 对象移动
                // ============================================================
                GameEvent::ObjectTurned { object_id, direction, location } => {
                    self.handle_object_turned(world, object_id, direction, location);
                }
                
                GameEvent::ObjectWalked { object_id, direction, location } => {
                    self.handle_object_moved(world, object_id, direction, location);
                }
                
                GameEvent::ObjectRan { object_id, direction, location } => {
                    self.handle_object_moved(world, object_id, direction, location);
                }
                
                // ============================================================
                // 对象状态更新
                // ============================================================
                GameEvent::ObjectHealthChanged { object_id, percent } => {
                    self.handle_object_health_changed(world, object_id, percent);
                }
                
                // ============================================================
                // 地图事件
                // ============================================================
                GameEvent::MapChanged { file_name, location } => {
                    self.handle_map_changed(world, file_name, location);
                }
                
                // ============================================================
                // 玩家状态更新 (暂时忽略,等待完整实现)
                // ============================================================
                GameEvent::PlayerSpawned { .. } => {
                    // PlayerSpawned 已经通过 ObjectSpawned 处理
                    // 这里可以做额外的玩家特定初始化
                }
                
                GameEvent::UserInformation { user_info } => {
                    // TODO: 更新玩家详细信息 (属性、装备、技能等)
                    if self.debug_logging {
                        tracing::debug!("📊 UserInformation received, full update needed");
                    }
                }
                
                // ============================================================
                // 其他事件 (暂时记录日志)
                // ============================================================
                GameEvent::Connected => {
                    tracing::info!("🌐 Connected to server");
                }
                
                GameEvent::Disconnected { reason } => {
                    tracing::warn!("🔌 Disconnected from server: {}", reason);
                }
                
                GameEvent::SystemMessage { message } => {
                    tracing::info!("💬 System: {}", message);
                }
                
                // 其他事件暂不处理 (UI、聊天、交易等由其他系统处理)
                _ => {
                    // 静默忽略其他事件
                }
            }
        }
        
        Ok(())
    }
    
    fn priority(&self) -> i32 {
        510 // Stage 5: State Update - 在网络同步(150)之后,渲染(1000)之前
    }
}

impl Default for GameEventSystem {
    fn default() -> Self {
        Self::new()
    }
}
