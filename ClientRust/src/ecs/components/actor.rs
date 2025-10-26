// ============================================================================
// 怪物/NPC 等游戏角色组件
// ============================================================================

pub use mir2_shared::{MirClass, MirGender};

/// 怪物数据组件
#[derive(Debug, Clone)]
pub struct MonsterData {
    pub id: u32,
    pub name: String,
    pub monster_index: u16,
    pub ai_mode: u8,
    pub ai_type: u8,         // AI 类型 (0=无, 1=近战, 2=远程, 3=巡逻)
    pub spawn_x: f32,        // 出生点 X
    pub spawn_y: f32,        // 出生点 Y
}

/// AI 状态组件
#[derive(Debug, Clone)]
pub struct AIState {
    pub mode: AIMode,
    pub current_action: AIAction,
    pub target_entity: Option<hecs::Entity>, // 目标实体
    pub target_pos: Option<(f32, f32)>,      // 目标位置
    pub last_action_time: u64,
    pub patrol_points: Vec<(f32, f32)>,      // 巡逻路径点
    pub current_patrol_index: usize,         // 当前巡逻点索引
}

impl Default for AIState {
    fn default() -> Self {
        Self {
            mode: AIMode::Idle,
            current_action: AIAction::Idle,
            target_entity: None,
            target_pos: None,
            last_action_time: 0,
            patrol_points: Vec::new(),
            current_patrol_index: 0,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum AIMode {
    Idle,
    Patrol,
    Chase,
    Attack,
    Retreat,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum AIAction {
    Idle,      // 闲置
    Patrol,    // 巡逻
    Chase,     // 追击
    Attack,    // 攻击
    Retreat,   // 后退
}

/// NPC 数据组件
#[derive(Debug, Clone)]
pub struct NPCData {
    pub id: u32,
    pub name: String,
    pub npc_index: u16,
    pub dialogue_id: u32,
    pub colour: i32,  // NPC颜色染色 (ARGB格式)
    pub action_timer: u32,  // 动作切换计时器(毫秒)
    pub next_action_delay: u32,  // 下次切换延迟(毫秒)
}

/// NPC任务标识
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum QuestIcon {
    None,           // 无任务
    Available,      // 可接任务(黄色感叹号)
    Complete,       // 可交任务(黄色问号)
    Incomplete,     // 进行中任务(灰色问号)
}

/// 任务标记组件
#[derive(Debug, Clone, Copy)]
pub struct QuestMarker {
    pub icon: QuestIcon,
}

impl QuestMarker {
    pub fn new(icon: QuestIcon) -> Self {
        Self { icon }
    }
}

/// NPC组件
#[derive(Debug, Clone)]
pub struct NPC {
    pub name: String,
    pub npc_type: String,
    pub can_interact: bool,
}

impl NPC {
    pub fn new(name: String, npc_type: String) -> Self {
        Self {
            name,
            npc_type,
            can_interact: true,
        }
    }
}

/// 怪物组件
#[derive(Debug, Clone)]
pub struct Monster {
    pub name: String,
    pub monster_type: u16,
    pub ai_state: MonsterAIState,
    pub target_id: Option<u32>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MonsterAIState {
    Idle,
    Patrol,
    Chase,
    Attack,
    Dead,
}

impl Monster {
    pub fn new(name: String, monster_type: u16) -> Self {
        Self {
            name,
            monster_type,
            ai_state: MonsterAIState::Idle,
            target_id: None,
        }
    }
}
