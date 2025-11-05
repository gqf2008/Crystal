// ============================================================================
// GameWorld - hecs ECS 世界管理
// ============================================================================

use hecs::World;
use std::ops::{Deref, DerefMut};
use std::time::Instant;

use super::components::*;
use crate::network::NetContext;
use crate::ClientSettings;
use ggez::mint::Point2;
use mir2_shared::Point;

// 使用有效的Entity ID (高32位为generation=1, 低32位为index)
const SETTING_ENTITY: hecs::Entity = hecs::Entity::from_bits(0x100000001).unwrap();
const NETWORK_ENTITY: hecs::Entity = hecs::Entity::from_bits(0x100000002).unwrap();

/// 游戏世界 - 管理所有实体
/// 
/// ## 功能
/// 
/// 1. **单例组件管理** - Settings 和 Network
/// 2. **实体工厂方法** - 统一创建玩家、怪物、NPC 等
/// 3. **查询工具** - 快速查找特定实体
/// 4. **生命周期管理** - 自动清理死亡实体
/// 
/// ## 坐标系统
/// 
/// `GameWorld` 支持两种坐标类型：
/// - **`Point2<f32>`** (ggez/mint) - 屏幕/渲染坐标，高精度
/// - **`Point` (i32)** (mir2_shared) - 地图网格坐标，用于网络同步
/// 
/// 每个实体工厂方法都有两个版本：
/// - `spawn_xxx()` - 接受 `Point` (i32) 地图坐标
/// - `spawn_xxx_at()` - 接受 `Point2<f32>` 屏幕坐标
/// 
/// ## 示例
/// 
/// ```rust
/// use mir2_client::ecs::GameWorld;
/// use ggez::mint::Point2;
/// use mir2_shared::Point;
/// 
/// let mut game_world = GameWorld::new();
/// 
/// // 1. 初始化单例组件
/// game_world.spawn_settings(settings);
/// game_world.spawn_network(net_ctx);
/// 
/// // 2. 使用 ggez 标准坐标（屏幕/渲染）
/// let player = game_world.spawn_local_player_at(
///     "Hero".to_string(),
///     MirClass::Warrior,
///     MirGender::Male,
///     Point2 { x: 100.5, y: 200.3 },  // f32 精度
/// );
/// 
/// // 3. 使用网络协议坐标（地图网格）
/// let monster = game_world.spawn_monster(
///     1001,
///     "Deer".to_string(),
///     0,
///     Point { x: 110, y: 210 },  // i32 网格坐标
/// );
/// 
/// // 4. 透明访问内部 World（通过 Deref）
/// for (id, pos) in game_world.query::<&Position>().iter() {
///     println!("Entity {:?} at {:?}", id, pos);
/// }
/// 
/// // 5. 使用查询工具
/// if let Some(pos) = game_world.get_local_player_position() {
///     println!("Player at grid ({}, {})", pos.x, pos.y);
/// }
/// 
/// // 6. 清理死亡实体
/// game_world.cleanup_dead_entities();
/// ```
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
    // 单例组件管理 (Settings, Network)
    // ========================================================================

    /// 初始化客户端设置
    pub fn spawn_settings(&mut self, settings: ClientSettings) -> &mut Self {
        self.world.spawn_at(SETTING_ENTITY, (settings,));
        self
    }

    /// 初始化网络上下文
    pub fn spawn_network(&mut self, net_ctx: NetContext) -> &mut Self {
        self.world.spawn_at(NETWORK_ENTITY, (net_ctx,));
        self
    }

    /// 获取客户端设置
    pub fn settings(&self) -> hecs::Ref<'_, ClientSettings> {
        self.world
            .get::<&ClientSettings>(SETTING_ENTITY)
            .expect("GameWorld ClientSettings not found")
    }

    /// 获取网络上下文
    pub fn network(&self) -> hecs::Ref<'_, NetContext> {
        self.world
            .get::<&NetContext>(NETWORK_ENTITY)
            .expect("GameWorld NetContext not found")
    }

    // ========================================================================
    // 实体创建工厂方法
    // ========================================================================
    
    /// 创建本地玩家（使用屏幕坐标）
    /// 
    /// 便捷方法：接受 `mint::Point2<f32>` 屏幕坐标
    pub fn spawn_local_player_at(
        &mut self,
        name: String,
        class: MirClass,
        gender: MirGender,
        position: Point2<f32>,
    ) -> hecs::Entity {
        let grid_pos = Point::new(position.x as i32, position.y as i32);
        self.spawn_local_player(name, class, gender, grid_pos)
    }

    /// 创建本地玩家（使用地图网格坐标）
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
            Mana::new(100),
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
            },
            Experience::new(1),
            Currency::new(),
            LocalPlayer,
            NetworkSync::new(0, NetworkObjectType::Player),
            RenderOrder::new(RenderLayer::Object, position.y),
            AnimationFrame::new(), // 🆕 动画帧组件 (AnimationSystem 更新, RenderSystem 读取)
        ))
    }

    /// 创建完整的本地玩家（包含移动、动画、输入等所有组件）
    /// 
    /// 此方法适用于需要完整功能的场景（如地图查看器、游戏场景）
    pub fn spawn_local_player_full(
        &mut self,
        class: MirClass,
        gender: MirGender,
        position: Point,
        hair: u8,
        weapon: i16,
        armour: i16,
    ) -> hecs::Entity {
        self.world.spawn((
            // 位置组件（世界坐标，单位：像素）
            Position::new((position.x * 48) as f32, (position.y * 32) as f32),
            // 移动速度组件
            movement::MovementVelocity::with_speeds(120.0, 60.0, 120.0),
            // 路径组件（寻路系统需要）
            movement::Path::new(),
            // 玩家核心状态
            Player {
                direction: mir2_shared::MirDirection::Down,
                action: PlayerAction::Stand,
            },
            // 玩家外观
            PlayerAppearance {
                class,
                gender,
                hair,
                weapon,
                armour,
                weapon_effect: 0,
                wing_effect: 0,
            },
            // 玩家输入组件
            PlayerInput::default(),
            // 本地玩家标记
            LocalPlayer,
            // 动画帧组件（AnimationSystem 更新, RenderSystem 读取）
            AnimationFrame::new(),
        ))
    }

    /// 创建远程玩家（使用屏幕坐标）
    pub fn spawn_remote_player_at(
        &mut self,
        id: u32,
        name: String,
        class: MirClass,
        gender: MirGender,
        position: Point2<f32>,
    ) -> hecs::Entity {
        let grid_pos = Point::new(position.x as i32, position.y as i32);
        self.spawn_remote_player(id, name, class, gender, grid_pos)
    }

    /// 创建远程玩家（使用地图网格坐标）
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
            Mana::new(100),
            PlayerData {
                id,
                name,
                class,
                gender,
                level: 1,
            },
            Experience::new(1),
            Currency::new(),
            RemotePlayer { id },
            RenderOrder::new(RenderLayer::Object, position.y),
        ))
    }

    /// 创建怪物（使用屏幕坐标）
    pub fn spawn_monster_at(
        &mut self,
        id: u32,
        name: String,
        monster_index: u16,
        position: Point2<f32>,
    ) -> hecs::Entity {
        let grid_pos = Point::new(position.x as i32, position.y as i32);
        self.spawn_monster(id, name, monster_index, grid_pos)
    }

    /// 创建怪物（使用地图网格坐标）
    /// 
    /// 基础版本：用于网络同步等简单场景
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

    /// 创建完整的怪物实体（包含移动、动画等所有组件）
    /// 
    /// 完整版本：用于需要完整功能的场景（AI、动画、移动等）
    pub fn spawn_monster_full(
        &mut self,
        id: u32,
        name: String,
        monster_index: u16,
        position: Point,
        max_hp: i32,
        stats: CombatStats,
    ) -> hecs::Entity {
        self.world.spawn((
            // 位置组件（世界坐标，单位：像素）
            Position::new((position.x * 48) as f32, (position.y * 32) as f32),
            // 移动速度组件
            movement::MovementVelocity::with_speeds(100.0, 50.0, 100.0),
            // 路径组件（寻路系统需要）
            movement::Path::new(),
            // 怪物核心数据
            MonsterData {
                id,
                name,
                monster_index,
                ai_mode: 0,
                ai_type: 0,
                spawn_x: position.x as f32,
                spawn_y: position.y as f32,
                direction: 0,
            },
            // 生命值
            Health::new(max_hp),
            // 战斗属性
            stats,
            // AI 状态
            AIState {
                current_action: crate::ecs::components::AIAction::Idle,
                mode: AIMode::Idle,
                target_entity: None,
                target_pos: None,
                patrol_points: Vec::new(),
                current_patrol_index: 0,
                last_action_time: 0,
            },
            // 网络同步
            NetworkSync::new(id, NetworkObjectType::Monster),
            // 渲染层级
            RenderOrder::new(RenderLayer::Object, position.y),
            // 动画帧组件（AnimationSystem 更新, RenderSystem 读取）
            AnimationFrame::new(),
        ))
    }

    /// 创建 NPC（使用屏幕坐标）
    pub fn spawn_npc_at(
        &mut self,
        id: u32,
        name: String,
        npc_index: u16,
        position: Point2<f32>,
    ) -> hecs::Entity {
        let grid_pos = Point::new(position.x as i32, position.y as i32);
        self.spawn_npc(id, name, npc_index, grid_pos)
    }

    /// 创建 NPC（使用地图网格坐标）
    /// 
    /// 基础版本：用于网络同步等简单场景
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

    /// 创建完整的 NPC 实体（包含动画等所有组件）
    /// 
    /// 完整版本：用于需要完整功能的场景（动画、对话等）
    pub fn spawn_npc_full(
        &mut self,
        id: u32,
        name: String,
        npc_index: u16,
        position: Point,
        dialogue_id: u32,
    ) -> hecs::Entity {
        self.world.spawn((
            // 位置组件（世界坐标，单位：像素）
            Position::new((position.x * 48) as f32, (position.y * 32) as f32),
            // NPC 核心数据
            NPCData {
                id,
                name,
                npc_index,
                dialogue_id,
                colour: 0,
                action_timer: 0,
                next_action_delay: rand::random::<u32>() % 5000 + 3000,
                direction: 0,
            },
            // 渲染层级
            RenderOrder::new(RenderLayer::Object, position.y),
            // 动画帧组件（AnimationSystem 更新, RenderSystem 读取）
            AnimationFrame::new(),
        ))
    }

    /// 创建技能特效（使用屏幕坐标）
    pub fn spawn_spell_effect_at(
        &mut self,
        spell_id: u16,
        caster_id: u32,
        position: Point2<f32>,
        target_pos: Point2<f32>,
        duration_ms: u32,
    ) -> hecs::Entity {
        let start = Point::new(position.x as i32, position.y as i32);
        let end = Point::new(target_pos.x as i32, target_pos.y as i32);
        self.spawn_spell_effect(spell_id, caster_id, start, end, duration_ms)
    }

    /// 创建技能特效 (ADD 混合，使用地map网格坐标)
    /// 
    /// 基础版本：用于简单的技能特效
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

    /// 创建完整的技能特效（包含动画帧等所有组件）
    /// 
    /// 完整版本：用于需要完整动画的技能特效
    pub fn spawn_spell_effect_full(
        &mut self,
        spell_id: u16,
        caster_id: u32,
        position: Point,
        target_pos: Point,
        duration_ms: u32,
        blend_mode: SpriteBlendMode,
    ) -> hecs::Entity {
        // 计算速度 (从起点到终点)
        let dx = (target_pos.x - position.x) as f32;
        let dy = (target_pos.y - position.y) as f32;
        let distance = (dx * dx + dy * dy).sqrt();
        let speed = 300.0; // 像素/秒
        let vx = if distance > 0.0 { dx / distance * speed } else { 0.0 };
        let vy = if distance > 0.0 { dy / distance * speed } else { 0.0 };

        self.world.spawn((
            // 位置组件
            Position::new((position.x * 48) as f32, (position.y * 32) as f32),
            // 速度组件
            Velocity::new(vx, vy),
            // 精灵组件（使用指定的混合模式）
            Sprite::with_blend(6, spell_id as i32, blend_mode),
            // 技能数据
            SpellData {
                spell_id,
                caster_id,
                target_pos,
                power: 10,
            },
            // 生命周期
            Lifetime::new(duration_ms),
            // 渲染层级
            RenderOrder::new(RenderLayer::Effect, position.y),
            // 动画帧组件（AnimationSystem 更新, RenderSystem 读取）
            AnimationFrame::new(),
        ))
    }

    /// 创建地面物品（使用屏幕坐标）
    pub fn spawn_item_drop_at(
        &mut self,
        item_id: u32,
        item_index: u16,
        count: u32,
        position: Point2<f32>,
        owner_id: Option<u32>,
    ) -> hecs::Entity {
        let grid_pos = Point::new(position.x as i32, position.y as i32);
        self.spawn_item_drop(item_id, item_index, count, grid_pos, owner_id)
    }

    /// 创建地面物品（使用地图网格坐标）
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

    // ========================================================================
    // 统计和批量操作
    // ========================================================================

    /// 获取所有远程玩家数量
    pub fn count_remote_players(&self) -> usize {
        self.world.query::<&RemotePlayer>().iter().count()
    }

    /// 获取所有怪物数量
    pub fn count_monsters(&self) -> usize {
        self.world.query::<&MonsterData>().iter().count()
    }

    /// 获取所有 NPC 数量
    pub fn count_npcs(&self) -> usize {
        self.world.query::<&NPCData>().iter().count()
    }

    /// 获取世界中所有实体数量
    pub fn entity_count(&self) -> u32 {
        self.world.len()
    }

    /// 清除所有远程玩家
    pub fn clear_remote_players(&mut self) {
        let entities: Vec<_> = self
            .world
            .query::<&RemotePlayer>()
            .iter()
            .map(|(e, _)| e)
            .collect();
        
        for entity in entities {
            self.despawn(entity);
        }
    }

    /// 清除所有怪物
    pub fn clear_monsters(&mut self) {
        let entities: Vec<_> = self
            .world
            .query::<&MonsterData>()
            .iter()
            .map(|(e, _)| e)
            .collect();
        
        for entity in entities {
            self.despawn(entity);
        }
    }

    /// 清除所有特效
    pub fn clear_effects(&mut self) {
        let entities: Vec<_> = self
            .world
            .query::<&SpellData>()
            .iter()
            .map(|(e, _)| e)
            .collect();
        
        for entity in entities {
            self.despawn(entity);
        }
    }
}

impl Default for GameWorld {
    fn default() -> Self {
        Self::new()
    }
}

// ============================================================================
// Deref 实现 - 透明访问内部 World
// ============================================================================

impl Deref for GameWorld {
    type Target = World;

    fn deref(&self) -> &Self::Target {
        &self.world
    }
}

impl DerefMut for GameWorld {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.world
    }
}



