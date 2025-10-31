// Network Event System - 处理网络事件并更新 ECS 组件
//
// 职责:
// 1. 从队列中读取 GameEvent
// 2. 更新 ECS 组件 (PlayerState, Health, Mana, Inventory, etc.)
// 3. 创建/删除实体 (远程玩家, 怪物, NPC, 物品)

use hecs::{Entity, World};
use std::sync::Arc;
use mir2_shared::enums::MirAction;

use crate::network::{NetContext, handlers::GameEvent};
use crate::ecs::components::*;
use crate::ecs::world::GameWorld;

/// Network Event System - 消费 GameEvents 并同步到 ECS
pub struct NetworkEventSystem {
    /// 网络上下文（接收网络事件）
    net_ctx: Arc<NetContext>,
    
    /// 缓存本地玩家实体 (性能优化)
    local_player_entity: Option<Entity>,
    
    /// 缓存远程对象 ID -> Entity 映射
    object_map: std::collections::HashMap<u32, Entity>,
}

impl NetworkEventSystem {
    /// 创建新的 NetworkEventSystem
    pub fn new(net_ctx: Arc<NetContext>) -> Self {
        Self {
            net_ctx,
            local_player_entity: None,
            object_map: std::collections::HashMap::new(),
        }
    }
    
    /// 处理所有待处理的网络事件
    pub fn update(&mut self, game_world: &mut GameWorld) {
        // 批量接收所有待处理事件
        let events = self.net_ctx.recv_all();
        
        for event in events {
            if let Err(e) = self.handle_event(event, game_world) {
                tracing::error!("Failed to handle network event: {:?}", e);
            }
        }
    }
    
    /// 处理单个事件
    fn handle_event(&mut self, event: GameEvent, game_world: &mut GameWorld) -> anyhow::Result<()> {
        match event {
            // ========================================================================
            // 连接事件
            // ========================================================================
            GameEvent::Connected => {
                tracing::info!("✅ Connected to server");
                // 可以在这里显示"已连接"UI提示
                Ok(())
            }
            
            GameEvent::Disconnected { reason } => {
                tracing::warn!("❌ Disconnected from server (reason: {})", reason);
                self.cleanup_all_entities(game_world);
                Ok(())
            }
            
            // ========================================================================
            // 角色信息 (UserInformation)
            // ========================================================================
            GameEvent::PlayerLocationChanged { x, y } => {
                self.update_local_player_position(game_world, x, y)
            }
            
            GameEvent::HealthChanged { current, max } => {
                self.update_local_player_health(game_world, current as u32, max as u32)
            }
            
            GameEvent::ManaChanged { current, max } => {
                self.update_local_player_mana(game_world, current as u32, max as u32)
            }
            
            GameEvent::GoldChanged { amount } => {
                self.update_local_player_gold(game_world, amount)
            }
            
            // ========================================================================
            // 战斗事件
            // ========================================================================
            GameEvent::PlayerStruck { attacker_id, damage } => {
                self.handle_player_struck(game_world, attacker_id, damage)
            }
            
            GameEvent::PlayerDied => {
                tracing::warn!("💀 Local player died");
                self.handle_local_player_death(game_world)
            }
            
            GameEvent::ObjectStruck { object_id, attacker_id, damage } => {
                self.handle_object_struck(game_world, object_id, attacker_id, damage)
            }
            
            GameEvent::ObjectDied { object_id } => {
                self.handle_object_death(game_world, object_id)
            }
            
            GameEvent::ExperienceGained { amount } => {
                self.handle_experience_gained(game_world, amount)
            }
            
            GameEvent::LevelUp { new_level } => {
                self.handle_level_changed(game_world, new_level)
            }
            
            // ========================================================================
            // 聊天事件
            // ========================================================================
            GameEvent::SystemMessage { message } => {
                tracing::info!("💬 System: {}", message);
                // TODO: 添加到聊天历史UI
                Ok(())
            }
            
            GameEvent::ChatMessage { sender, message, chat_type } => {
                tracing::info!("💬 {} says: {}", sender, message);
                // TODO: 显示气泡对话
                Ok(())
            }
            
            // ========================================================================
            // 物品事件
            // ========================================================================
            GameEvent::ItemGained { item } => {
                tracing::info!("🎁 Gained item: {:?}", item);
                // TODO: 添加到背包
                Ok(())
            }
            
            GameEvent::ItemLost { unique_id } => {
                tracing::info!("� Lost item: ID={}", unique_id);
                // TODO: 从背包移除
                Ok(())
            }
            
            // ========================================================================
            // 组队事件
            // ========================================================================
            GameEvent::GroupInvite { inviter } => {
                tracing::info!("👥 Group invite from: {}", inviter);
                // TODO: 显示组队邀请UI
                Ok(())
            }
            
            GameEvent::GroupMemberAdded { name } => {
                tracing::info!("👥 {} joined the group", name);
                Ok(())
            }
            
            GameEvent::GroupMemberRemoved { name } => {
                tracing::info!("👥 {} left the group", name);
                Ok(())
            }
            
            GameEvent::GroupDisbanded => {
                tracing::info!("👥 Group disbanded");
                Ok(())
            }
            
            // ========================================================================
            // 公会事件
            // ========================================================================
            GameEvent::GuildInvite { inviter, guild_name } => {
                tracing::info!("🏰 Guild invite from {} to join {}", inviter, guild_name);
                // TODO: 显示公会邀请UI
                Ok(())
            }
            
            GameEvent::GuildJoined { guild_name } => {
                tracing::info!("🏰 Joined guild: {}", guild_name);
                self.update_local_player_guild(game_world, guild_name)
            }
            
            GameEvent::GuildLeft => {
                tracing::info!("🏰 Left guild");
                // TODO: 移除 GuildMembership 组件
                Ok(())
            }
            
            // ========================================================================
            // NPC 事件
            // ========================================================================
            GameEvent::NpcDialog { npc_id, dialog } => {
                tracing::info!("📜 NPC {} dialog: {}", npc_id, dialog);
                // TODO: 显示 NPC 对话框
                Ok(())
            }
            
            // ========================================================================
            // 游戏启动事件
            // ========================================================================
            GameEvent::StartGame { delay } => {
                tracing::info!("⏳ Game start delayed by {} ms", delay);
                Ok(())
            }
            
            GameEvent::CharacterCreated { name } => {
                tracing::info!("🎮 New character created: {}", name);
                Ok(())
            }
            
            GameEvent::CharacterDeleted { index } => {
                tracing::info!("🗑️ Character deleted: index {}", index);
                Ok(())
            }
            
            // ========================================================================
            // 其他默认处理
            // ========================================================================
            _ => {
                tracing::warn!("⚠️ Unhandled event: {:?}", event);
                Ok(())
            }
        }
    }
    
    // ========================================================================
    // 辅助方法 - 玩家状态更新
    // ========================================================================
    
    /// 获取或查找本地玩家实体
    fn get_local_player_entity(&mut self, world: &World) -> Option<Entity> {
        // 如果已缓存，验证实体仍然存在
        if let Some(entity) = self.local_player_entity {
            if world.contains(entity) {
                return Some(entity);
            }
        }
        
        // 重新查找本地玩家
        for (entity, _) in world.query::<&LocalPlayer>().iter() {
            self.local_player_entity = Some(entity);
            return Some(entity);
        }
        
        None
    }
    
    /// 更新本地玩家位置
    fn update_local_player_position(&mut self, game_world: &mut GameWorld, x: i32, y: i32) -> anyhow::Result<()> {
        let entity = self.get_local_player_entity(&game_world.world)
            .ok_or_else(|| anyhow::anyhow!("Local player not found"))?;
        
        if let Ok(mut pos) = game_world.world.get::<&mut Position>(entity) {
            pos.x = x as f32;
            pos.y = y as f32;
            tracing::debug!("📍 Player position updated: ({}, {})", x, y);
        }
        
        Ok(())
    }
    
    /// 更新本地玩家血量
    fn update_local_player_health(&mut self, game_world: &mut GameWorld, current: u32, max: u32) -> anyhow::Result<()> {
        let entity = self.get_local_player_entity(&game_world.world)
            .ok_or_else(|| anyhow::anyhow!("Local player not found"))?;
        
        if let Ok(mut health) = game_world.world.get::<&mut Health>(entity) {
            health.current = current as i32;
            health.max = max as i32;
            tracing::debug!("❤️ HP: {}/{}", current, max);
        }
        
        Ok(())
    }
    
    /// 更新本地玩家魔法值
    fn update_local_player_mana(&mut self, game_world: &mut GameWorld, current: u32, max: u32) -> anyhow::Result<()> {
        let entity = self.get_local_player_entity(&game_world.world)
            .ok_or_else(|| anyhow::anyhow!("Local player not found"))?;
        
        // 检查是否已有 Mana 组件
        let has_mana = game_world.world.get::<&Mana>(entity).is_ok();
        
        if has_mana {
            if let Ok(mut mana) = game_world.world.get::<&mut Mana>(entity) {
                mana.current = current as i32;
                mana.max = max as i32;
                tracing::debug!("💙 MP: {}/{}", current, max);
            }
        } else {
            // 添加 Mana 组件
            let _ = game_world.world.insert_one(entity, Mana::new(max as i32));
            tracing::debug!("💙 MP component added: {}/{}", current, max);
        }
        
        Ok(())
    }
    
    /// 更新本地玩家金币
    fn update_local_player_gold(&mut self, game_world: &mut GameWorld, amount: u32) -> anyhow::Result<()> {
        let entity = self.get_local_player_entity(&game_world.world)
            .ok_or_else(|| anyhow::anyhow!("Local player not found"))?;
        
        if let Ok(mut player_data) = game_world.world.get::<&mut PlayerData>(entity) {
            player_data.gold = amount;
            tracing::debug!("💰 Gold: {}", amount);
        }
        
        Ok(())
    }
    
    /// 更新本地玩家公会信息
    fn update_local_player_guild(&mut self, game_world: &mut GameWorld, guild_name: String) -> anyhow::Result<()> {
        let entity = self.get_local_player_entity(&game_world.world)
            .ok_or_else(|| anyhow::anyhow!("Local player not found"))?;
        
        // 检查是否已有 GuildMembership 组件
        if game_world.world.get::<&mut GuildMembership>(entity).is_ok() {
            if let Ok(mut guild) = game_world.world.get::<&mut GuildMembership>(entity) {
                guild.guild_name = guild_name;
            }
        } else {
            // 添加 GuildMembership 组件 (默认成员 rank=2)
            let _ = game_world.world.insert_one(entity, GuildMembership {
                guild_name,
                rank: 2,
            });
        }
        
        Ok(())
    }
    
    // ========================================================================
    // 辅助方法 - 战斗相关
    // ========================================================================
    
    /// 处理本地玩家被击中
    fn handle_player_struck(&mut self, game_world: &mut GameWorld, attacker_id: u32, damage: i32) -> anyhow::Result<()> {
        tracing::warn!("💥 Struck by attacker ID: {} for {} damage", attacker_id, damage);
        
        // TODO: 播放受击动画/音效
        // TODO: 血量由 HealthChanged 事件更新
        
        Ok(())
    }
    
    /// 处理本地玩家死亡
    fn handle_local_player_death(&mut self, game_world: &mut GameWorld) -> anyhow::Result<()> {
        let entity = self.get_local_player_entity(&game_world.world)
            .ok_or_else(|| anyhow::anyhow!("Local player not found"))?;
        
        // 更新动画状态为死亡
        if let Ok(mut anim) = game_world.world.get::<&mut Animation>(entity) {
            anim.action = MirAction::Dead;
            anim.frame_index = 0;
        }
        
        // TODO: 显示复活UI
        
        Ok(())
    }
    
    /// 处理远程对象被击中
    fn handle_object_struck(&mut self, game_world: &mut GameWorld, object_id: u32, attacker_id: u32, damage: i32) -> anyhow::Result<()> {
        if let Some(&entity) = self.object_map.get(&object_id) {
            tracing::debug!("💥 Object {} struck by {} for {} damage", object_id, attacker_id, damage);
            
            // TODO: 播放受击动画
            if let Ok(mut anim) = game_world.world.get::<&mut Animation>(entity) {
                anim.action = MirAction::Struck;
                anim.frame_index = 0;
            }
        }
        
        Ok(())
    }
    
    /// 处理远程对象死亡
    fn handle_object_death(&mut self, game_world: &mut GameWorld, object_id: u32) -> anyhow::Result<()> {
        if let Some(&entity) = self.object_map.get(&object_id) {
            tracing::info!("💀 Object {} died", object_id);
            
            // 播放死亡动画
            if let Ok(mut anim) = game_world.world.get::<&mut Animation>(entity) {
                anim.action = MirAction::Dead;
                anim.frame_index = 0;
            }
            
            // 移除实体 (延迟一段时间后)
            // TODO: 添加延迟删除逻辑
            game_world.world.despawn(entity).ok();
            self.object_map.remove(&object_id);
        }
        
        Ok(())
    }
    
    /// 处理经验获得
    fn handle_experience_gained(&mut self, game_world: &mut GameWorld, amount: i64) -> anyhow::Result<()> {
        let entity = self.get_local_player_entity(&game_world.world)
            .ok_or_else(|| anyhow::anyhow!("Local player not found"))?;
        
        if let Ok(mut player_data) = game_world.world.get::<&mut PlayerData>(entity) {
            player_data.exp += amount;
            tracing::info!("⭐ Gained {} EXP (total: {})", amount, player_data.exp);
        }
        
        Ok(())
    }
    
    /// 处理等级变化
    fn handle_level_changed(&mut self, game_world: &mut GameWorld, level: u16) -> anyhow::Result<()> {
        let entity = self.get_local_player_entity(&game_world.world)
            .ok_or_else(|| anyhow::anyhow!("Local player not found"))?;
        
        if let Ok(mut player_data) = game_world.world.get::<&mut PlayerData>(entity) {
            player_data.level = level;
            tracing::info!("🆙 Level up! Now level {}", level);
        }
        
        Ok(())
    }
    
    // ========================================================================
    // 辅助方法 - 清理
    // ========================================================================
    
    /// 清理所有实体 (断开连接时)
    fn cleanup_all_entities(&mut self, game_world: &mut GameWorld) {
        // 清除所有远程对象
        for (_, entity) in self.object_map.drain() {
            game_world.world.despawn(entity).ok();
        }
        
        // 清除本地玩家
        if let Some(entity) = self.local_player_entity {
            game_world.world.despawn(entity).ok();
            self.local_player_entity = None;
        }
        
        tracing::info!("🧹 Cleaned up all networked entities");
    }
}
