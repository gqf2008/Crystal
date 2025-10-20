// ============================================================================
// Components - ECS 组件定义
// 参考 C# Client/MirObjects/ 的对象属性
// ============================================================================

use mir2_shared::Point;
pub use mir2_shared::{MirDirection, MirAction, MirClass, MirGender};

// ============================================================================
// 核心组件 (所有实体都有)
// ============================================================================

/// 位置组件 - 所有实体必备
#[derive(Debug, Clone, Copy)]
pub struct Position {
    pub x: i32,      // 地图格子坐标
    pub y: i32,
    pub offset_x: i32, // 像素偏移 (用于移动插值)
    pub offset_y: i32,
}

impl Position {
    pub fn new(x: i32, y: i32) -> Self {
        Self { x, y, offset_x: 0, offset_y: 0 }
    }

    pub fn with_offset(x: i32, y: i32, offset_x: i32, offset_y: i32) -> Self {
        Self { x, y, offset_x, offset_y }
    }
}

/// 速度组件 - 移动实体必备
#[derive(Debug, Clone, Copy)]
pub struct Velocity {
    pub dx: f32,
    pub dy: f32,
}

impl Velocity {
    pub fn new(dx: f32, dy: f32) -> Self {
        Self { dx, dy }
    }

    pub fn zero() -> Self {
        Self { dx: 0.0, dy: 0.0 }
    }
}

/// 方向组件
#[derive(Debug, Clone, Copy)]
pub struct DirectionComp {
    pub current: MirDirection,
    pub target: MirDirection,
}

impl DirectionComp {
    pub fn new(dir: MirDirection) -> Self {
        Self { current: dir, target: dir }
    }
}

/// 精灵渲染组件 - 可渲染实体必备
#[derive(Debug, Clone)]
pub struct SpriteComp {
    pub library: i32,      // MLibrary 索引 (0=Tiles, 1=SmTiles, 2=Objects, etc.)
    pub index: i32,        // 贴图索引
    pub frame: i32,        // 当前帧
    pub blend_mode: BlendModeComp, // 混合模式
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum BlendModeComp {
    Alpha,
    Add,    // ⭐ ADD 混合 (技能特效)
    Multiply,
}

impl SpriteComp {
    pub fn new(library: i32, index: i32) -> Self {
        Self {
            library,
            index,
            frame: 0,
            blend_mode: BlendModeComp::Alpha,
        }
    }

    pub fn with_blend(library: i32, index: i32, blend_mode: BlendModeComp) -> Self {
        Self { library, index, frame: 0, blend_mode }
    }
}

// ============================================================================
// 动画组件
// ============================================================================

/// 动画状态组件
#[derive(Debug, Clone)]
pub struct AnimationComp {
    pub action: MirAction,
    pub frame_count: u8,
    pub frame_index: u8,
    pub frame_interval: u32, // 毫秒
    pub frame_timer: u32,
    pub loop_animation: bool,
}

impl AnimationComp {
    pub fn new(action: MirAction, frame_count: u8, frame_interval: u32) -> Self {
        Self {
            action,
            frame_count,
            frame_index: 0,
            frame_interval,
            frame_timer: 0,
            loop_animation: true,
        }
    }

    /// 更新动画 (返回 true 表示播放完成)
    pub fn update(&mut self, delta_ms: u32) -> bool {
        self.frame_timer += delta_ms;
        if self.frame_timer >= self.frame_interval {
            self.frame_timer = 0;
            self.frame_index += 1;

            if self.frame_index >= self.frame_count {
                if self.loop_animation {
                    self.frame_index = 0;
                } else {
                    self.frame_index = self.frame_count - 1;
                    return true; // 动画完成
                }
            }
        }
        false
    }
}

// ============================================================================
// 战斗相关组件
// ============================================================================

/// 生命值组件
#[derive(Debug, Clone, Copy)]
pub struct Health {
    pub current: i32,
    pub max: i32,
}

impl Health {
    pub fn new(max: i32) -> Self {
        Self { current: max, max }
    }

    pub fn is_alive(&self) -> bool {
        self.current > 0
    }

    pub fn take_damage(&mut self, damage: i32) {
        self.current = (self.current - damage).max(0);
    }

    pub fn heal(&mut self, amount: i32) {
        self.current = (self.current + amount).min(self.max);
    }
}

/// 战斗属性组件 (玩家/怪物)
#[derive(Debug, Clone)]
pub struct CombatStats {
    pub level: u16,
    pub attack_min: i32,
    pub attack_max: i32,
    pub defense: i32,
    pub magic_defense: i32,
    pub accuracy: u8,
    pub agility: u8,
}

// ============================================================================
// 玩家专用组件
// ============================================================================

/// 玩家数据组件 (标记这是玩家实体)
#[derive(Debug, Clone)]
pub struct PlayerComp {
    pub id: u32,
    pub name: String,
    pub class: MirClass,
    pub gender: MirGender,
    pub exp: i64,
    pub gold: u32,
}

/// 本地玩家标记 (只有一个)
#[derive(Debug, Clone, Copy)]
pub struct LocalPlayer;

/// 远程玩家标记 (网络同步)
#[derive(Debug, Clone, Copy)]
pub struct RemotePlayer {
    pub id: u32,
}

// ============================================================================
// 怪物专用组件
// ============================================================================

/// 怪物数据组件
#[derive(Debug, Clone)]
pub struct MonsterComp {
    pub id: u32,
    pub name: String,
    pub monster_index: u16,
    pub ai_mode: u8,
}

/// AI 状态组件
#[derive(Debug, Clone)]
pub struct AIState {
    pub mode: AIMode,
    pub target_entity: Option<hecs::Entity>, // 目标实体
    pub last_action_time: u64,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum AIMode {
    Idle,
    Patrol,
    Chase,
    Attack,
    Retreat,
}

// ============================================================================
// NPC 组件
// ============================================================================

/// NPC 数据组件
#[derive(Debug, Clone)]
pub struct NPCComp {
    pub id: u32,
    pub name: String,
    pub npc_index: u16,
    pub dialogue_id: u32,
}

// ============================================================================
// 技能/特效组件
// ============================================================================

/// 技能数据组件
#[derive(Debug, Clone)]
pub struct SpellComp {
    pub spell_id: u16,
    pub caster_id: u32,
    pub target_pos: Point,
    pub power: i32,
}

/// 生命周期组件 (技能特效/掉落物等有时间限制的实体)
#[derive(Debug, Clone, Copy)]
pub struct Lifetime {
    pub remaining_ms: u32,
}

impl Lifetime {
    pub fn new(duration_ms: u32) -> Self {
        Self { remaining_ms: duration_ms }
    }

    pub fn update(&mut self, delta_ms: u32) -> bool {
        if self.remaining_ms > delta_ms {
            self.remaining_ms -= delta_ms;
            false
        } else {
            self.remaining_ms = 0;
            true // 生命周期结束
        }
    }
}

// ============================================================================
// 物品组件
// ============================================================================

/// 地面物品组件
#[derive(Debug, Clone)]
pub struct ItemDrop {
    pub item_id: u32,
    pub item_index: u16,
    pub count: u32,
    pub owner_id: Option<u32>, // 归属玩家 (拾取保护)
}

// ============================================================================
// 网络同步组件
// ============================================================================

/// 网络同步标记 (需要同步的实体)
#[derive(Debug, Clone, Copy)]
pub struct NetworkSync {
    pub last_sync_time: u64,
}

// ============================================================================
// 渲染层级组件
// ============================================================================

/// 渲染层级 (用于排序)
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum RenderLayer {
    Ground = 0,      // 地面层
    GroundItem = 1,  // 地面物品
    Shadow = 2,      // 阴影
    Object = 3,      // 游戏对象 (玩家/怪物/NPC)
    Effect = 4,      // 特效 (技能/爆炸)
    UI = 5,          // UI元素
}

#[derive(Debug, Clone, Copy)]
pub struct RenderOrder {
    pub layer: RenderLayer,
    pub z_order: i32, // 同层内的排序 (Y坐标)
}

impl RenderOrder {
    pub fn new(layer: RenderLayer, z_order: i32) -> Self {
        Self { layer, z_order }
    }
}

