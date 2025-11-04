// ============================================================================
// GameWorld - hecs ECS 世界管理
// ============================================================================

use hecs::World;
use std::time::Instant;

use super::components::*;
use mir2_shared::{enums::*, Point};

/// 游戏世界 - 管理所有实体
pub struct GameWorld {
    pub world: World,
    pub start_time: Instant,
}

impl GameWorld {
    pub fn new() -> Self {
        Self {
            world: World::new(),
            start_time: Instant::now(),
        }
    }

    // ========================================================================
    // 实体创建工厂方法
    // ========================================================================

    /// 创建本地玩家
    pub fn spawn_local_player(
        &mut self,
        name: String,
        class: MirClass,
        gender: MirGender,
        position: Point,
    ) -> hecs::Entity {
        self.world.spawn((
            Position::new(position.x as f32, position.y as f32),
            Velocity::zero(),
            Sprite::new(2, 0), // Objects.wil
            Health::new(100),
            CombatStats {
                level: 1,
                attack_min: 5,
                attack_max: 10,
                defense: 5,
                magic_defense: 5,
                accuracy: 10,
                agility: 10,
            },
            PlayerData {
                id: 0,
                name,
                class,
                gender,
                level: 1,
                exp: 0,
                max_experience: 100,
                gold: 0,
                credit: 0,
            },
            LocalPlayer,
            NetworkSync::new(0, NetworkObjectType::Player),
            RenderOrder::new(RenderLayer::Object, position.y),
        ))
    }

    /// 创建远程玩家
    pub fn spawn_remote_player(
        &mut self,
        id: u32,
        name: String,
        class: MirClass,
        gender: MirGender,
        position: Point,
    ) -> hecs::Entity {
        self.world.spawn((
            Position::new(position.x as f32, position.y as f32),
            Velocity::zero(),
            Sprite::new(2, 0),
            Health::new(100),
            PlayerData {
                id,
                name,
                class,
                gender,
                level: 1,
                exp: 0,
                max_experience: 100,
                gold: 0,
                credit: 0,
            },
            RemotePlayer { id },
            RenderOrder::new(RenderLayer::Object, position.y),
        ))
    }

    /// 创建怪物
    pub fn spawn_monster(
        &mut self,
        id: u32,
        name: String,
        monster_index: u16,
        position: Point,
    ) -> hecs::Entity {
        self.world.spawn((
            Position::new(position.x as f32, position.y as f32),
            Velocity::zero(),
            Sprite::new(4, monster_index as i32), // Monsters.wil
            Health::new(50),
            CombatStats {
                level: 1,
                attack_min: 3,
                attack_max: 6,
                defense: 3,
                magic_defense: 2,
                accuracy: 8,
                agility: 5,
            },
            MonsterData {
                id,
                name,
                monster_index,
                ai_mode: 0,
                ai_type: 0,        // 无AI
                spawn_x: position.x as f32,
                spawn_y: position.y as f32,
                direction: 0,      // 默认朝向
            },
            AIState {
                current_action: crate::ecs::components::AIAction::Idle,
                mode: AIMode::Idle,
                target_entity: None,
                target_pos: None,
                patrol_points: Vec::new(),
                current_patrol_index: 0,
                last_action_time: 0,
            },
            NetworkSync::new(id, NetworkObjectType::Monster),
            RenderOrder::new(RenderLayer::Object, position.y),
        ))
    }

    /// 创建 NPC
    pub fn spawn_npc(
        &mut self,
        id: u32,
        name: String,
        npc_index: u16,
        position: Point,
    ) -> hecs::Entity {
        self.world.spawn((
            Position::new(position.x as f32, position.y as f32),
            Sprite::new(3, npc_index as i32), // NPCs.wil
            NPCData {
                id,
                name,
                npc_index,
                dialogue_id: 0,
                colour: 0, // 默认白色(无染色)
                action_timer: 0,
                next_action_delay: rand::random::<u32>() % 5000 + 3000,
                direction: 0, // 默认朝向
            },
            RenderOrder::new(RenderLayer::Object, position.y),
        ))
    }

    /// 创建技能特效 (ADD 混合)
    pub fn spawn_spell_effect(
        &mut self,
        spell_id: u16,
        caster_id: u32,
        position: Point,
        target_pos: Point,
        duration_ms: u32,
    ) -> hecs::Entity {
        // 计算速度 (从起点到终点)
        let dx = (target_pos.x - position.x) as f32;
        let dy = (target_pos.y - position.y) as f32;
        let distance = (dx * dx + dy * dy).sqrt();
        let speed = 10.0; // 像素/帧
        let vx = if distance > 0.0 { dx / distance * speed } else { 0.0 };
        let vy = if distance > 0.0 { dy / distance * speed } else { 0.0 };

        self.world.spawn((
            Position::new(position.x as f32, position.y as f32),
            Velocity::new(vx, vy),
            Sprite::with_blend(6, spell_id as i32, SpriteBlendMode::Additive), // Magic.wil + ADD混合
            SpellData {
                spell_id,
                caster_id,
                target_pos,
                power: 10,
            },
            Lifetime::new(duration_ms),
            RenderOrder::new(RenderLayer::Effect, position.y),
        ))
    }

    /// 创建地面物品
    pub fn spawn_item_drop(
        &mut self,
        item_id: u32,
        item_index: u16,
        count: u32,
        position: Point,
        owner_id: Option<u32>,
    ) -> hecs::Entity {
        self.world.spawn((
            Position::new(position.x as f32, position.y as f32),
            Sprite::new(5, item_index as i32), // Items.wil
            ItemDrop {
                item_id,
                item_index,
                count,
                owner_id,
            },
            Lifetime::new(60000), // 60秒后消失
            RenderOrder::new(RenderLayer::GroundItem, position.y),
        ))
    }

    // ========================================================================
    // 查询方法
    // ========================================================================

    /// 获取本地玩家实体
    pub fn get_local_player(&self) -> Option<hecs::Entity> {
        self.world
            .query::<&LocalPlayer>()
            .iter()
            .next()
            .map(|(entity, _)| entity)
    }

    /// 获取本地玩家位置
    pub fn get_local_player_position(&self) -> Option<Point> {
        let entity = self.get_local_player()?;
        let pos = self.world.get::<&Position>(entity).ok()?;
        Some(Point::new(pos.x as i32, pos.y as i32))
    }

    /// 根据 ID 查找远程玩家
    pub fn find_remote_player(&self, id: u32) -> Option<hecs::Entity> {
        self.world
            .query::<&RemotePlayer>()
            .iter()
            .find(|(_, remote)| remote.id == id)
            .map(|(entity, _)| entity)
    }

    /// 根据 ID 查找怪物
    pub fn find_monster(&self, id: u32) -> Option<hecs::Entity> {
        self.world
            .query::<&MonsterData>()
            .iter()
            .find(|(_, monster)| monster.id == id)
            .map(|(entity, _)| entity)
    }

    /// 获取某个位置的所有实体
    pub fn get_entities_at(&self, pos: Point) -> Vec<hecs::Entity> {
        self.world
            .query::<&Position>()
            .iter()
            .filter(|(_, position)| position.x as i32 == pos.x && position.y as i32 == pos.y)
            .map(|(entity, _)| entity)
            .collect()
    }

    // ========================================================================
    // 实体移除
    // ========================================================================

    /// 移除实体
    pub fn despawn(&mut self, entity: hecs::Entity) {
        let _ = self.world.despawn(entity);
    }

    /// 清理所有死亡实体
    pub fn cleanup_dead_entities(&mut self) {
        let mut to_remove = Vec::new();

        // 收集需要移除的实体
        for (entity, health) in self.world.query::<&Health>().iter() {
            if !health.is_alive() {
                to_remove.push(entity);
            }
        }

        // 收集生命周期结束的实体
        for (entity, lifetime) in self.world.query::<&Lifetime>().iter() {
            if lifetime.remaining_ms == 0 {
                to_remove.push(entity);
            }
        }

        // 移除
        for entity in to_remove {
            self.despawn(entity);
        }
    }
}

impl Default for GameWorld {
    fn default() -> Self {
        Self::new()
    }
}



