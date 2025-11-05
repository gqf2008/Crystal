// ============================================================================
// 玩家专用组件
// ============================================================================

pub use mir2_shared::{MirAction, MirClass, MirDirection, MirGender};

/// 玩家数据组件 (标记这是玩家实体 - 身份卡)
/// 
/// **设计原则**: 只包含玩家的身份识别信息 (不可变或较少变化的属性)
/// - ID、名字、职业、性别 - 角色的核心身份
/// - Level 保留用于UI显示和等级相关的逻辑判断
/// 
/// **已迁移到独立组件**:
/// - `exp`, `max_experience` → `Experience` 组件
/// - `gold`, `credit` → `Currency` 组件
/// - 生命值、魔法值 → `Health`, `Mana` 组件
/// - 战斗属性 → `CombatStats` 组件
/// 
/// **参考**: `ECS_COMPONENTS_ARCHITECTURE.md` - Player Component
#[derive(Debug, Clone)]
pub struct PlayerData {
    pub id: u32,
    pub name: String,
    pub class: MirClass,
    pub gender: MirGender,
    pub level: u16,  // 保留等级 (显示和逻辑判断用)
}

/// 本地玩家标记 (只有一个)
#[derive(Debug, Clone, Copy)]
pub struct LocalPlayer;

/// 远程玩家标记 (网络同步)
#[derive(Debug, Clone, Copy)]
pub struct RemotePlayer {
    pub id: u32,
}

/// 其他玩家组件（区别于本地玩家Player）
#[derive(Debug, Clone)]
pub struct OtherPlayer {
    pub name: String,
    pub class: MirClass,
    pub gender: MirGender,
    pub level: u16,
    pub guild_name: Option<String>,
}

impl OtherPlayer {
    pub fn new(name: String, class: MirClass, gender: MirGender, level: u16) -> Self {
        Self {
            name,
            class,
            gender,
            level,
            guild_name: None,
        }
    }
}

/// 角色组件 - 核心状态（单一职责）
/// 
/// **设计原则**：
/// - 只包含角色的核心游戏逻辑状态
/// - 不重复其他组件的数据（渲染用Animation组件，移动用Movement组件）
/// 
/// **数据所有权**：
/// - `direction`: 由 MovementSystem 根据移动方向更新
/// - `action`: 由 PlayerControlSystem 根据用户输入独占写入 (单一来源原则)
/// 
/// **注意**: `is_moving` 状态已移到 Movement 组件,使用 `movement.is_moving()` 方法查询
#[derive(Debug, Clone)]
pub struct Player {
    /// 面向方向 (8方向枚举)
    pub direction: MirDirection,
    /// 当前动作状态（行走/跑步/站立/攻击）
    /// 
    /// **重要**: 此字段只能由 PlayerControlSystem 写入！
    /// 其他系统(MovementSystem等)只能读取，不能修改
    pub action: PlayerAction,
}

/// 攻击状态组件 (ECS 原则: 状态存储在Component中)
/// 
/// 当玩家/怪物/NPC进行攻击时自动添加此组件
/// 攻击完成后自动移除
#[derive(Debug, Clone, Copy)]
pub struct AttackState {
    /// 攻击开始时间 (用于计算动画完成)
    pub start_time: std::time::Instant,
    /// 攻击类型 (Attack1/2/3)
    pub attack_type: PlayerAction,
}

/// 角色动作
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum PlayerAction {
    Stand = 0,
    Walk = 1,
    Run = 2,
    Attack1 = 3,  // 普通攻击1
    Attack2 = 4,  // 普通攻击2
    Attack3 = 5,  // 普通攻击3
}

impl PlayerAction {
    /// 转换为 MirAction (用于查询 Frame 配置)
    /// 
    /// 这个映射连接了 ECS 的 PlayerAction 和原版的 MirAction
    pub fn to_mir_action(&self) -> MirAction {
        match self {
            PlayerAction::Stand => MirAction::Standing,
            PlayerAction::Walk => MirAction::Walking,
            PlayerAction::Run => MirAction::Running,
            PlayerAction::Attack1 => MirAction::Attack1,
            PlayerAction::Attack2 => MirAction::Attack2,
            PlayerAction::Attack3 => MirAction::Attack3,
        }
    }
    
    /// 是否是攻击动作
    pub fn is_attack(&self) -> bool {
        matches!(self, PlayerAction::Attack1 | PlayerAction::Attack2 | PlayerAction::Attack3)
    }
    
    // ⚠️ 以下方法已废弃，改为从 objects/frames.rs 的 PLAYER_FRAMES 读取
    // 保留仅用于兼容性，后续将移除
    
    #[deprecated(note = "使用 objects::frames::get_player_frame() 替代")]
    pub fn frame_count(&self) -> i32 {
        match self {
            PlayerAction::Stand => 4,
            PlayerAction::Walk => 6,
            PlayerAction::Run => 6,
            PlayerAction::Attack1 => 6,
            PlayerAction::Attack2 => 6,
            PlayerAction::Attack3 => 6,
        }
    }
    
    #[deprecated(note = "使用 objects::frames::get_player_frame() 替代")]
    pub fn frame_interval(&self) -> i32 {
        match self {
            PlayerAction::Stand => 30,
            PlayerAction::Walk => 4,
            PlayerAction::Run => 2,
            PlayerAction::Attack1 => 1,
            PlayerAction::Attack2 => 1,
            PlayerAction::Attack3 => 1,
        }
    }
    
    #[deprecated(note = "使用 objects::frames::get_player_frame() 替代")]
    pub fn duration_ms(&self) -> u64 {
        (self.frame_count() * self.frame_interval() * 100) as u64
    }
    
    #[deprecated(note = "使用 objects::frames::get_player_frame() 替代")]
    pub fn frame_start(&self) -> i32 {
        match self {
            PlayerAction::Stand => 0,
            PlayerAction::Walk => 32,
            PlayerAction::Run => 80,
            PlayerAction::Attack1 => 128,
            PlayerAction::Attack2 => 176,
            PlayerAction::Attack3 => 224,
        }
    }
}

/// 移动模式状态机
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum MoveMode {
    Idle,
    DirectFollow,
    AutoPathfinding,
}

/// 玩家外观组件
#[derive(Debug, Clone)]
pub struct PlayerAppearance {
    pub class: mir2_shared::enums::MirClass,
    pub gender: mir2_shared::enums::MirGender,
    pub hair: u8,
    pub weapon: i16,
    pub armour: i16,
    pub weapon_effect: i16,
    pub wing_effect: u8,
}

impl Default for PlayerAppearance {
    fn default() -> Self {
        Self {
            class: mir2_shared::enums::MirClass::Warrior,
            gender: mir2_shared::enums::MirGender::Male,
            hair: 0,
            weapon: -1,  // -1 表示无武器
            armour: 0,   // 默认盔甲索引
            weapon_effect: 0,
            wing_effect: 0,
        }
    }
}

/// 可见性组件 - 控制实体是否可见
#[derive(Debug, Clone, Copy)]
pub struct Visibility {
    /// 是否隐身（隐身术、潜行等）
    pub hidden: bool,
    /// 是否死亡（死亡状态会影响渲染）
    pub dead: bool,
}

impl Visibility {
    pub fn new() -> Self {
        Self {
            hidden: false,
            dead: false,
        }
    }
    
    pub fn is_visible(&self) -> bool {
        !self.hidden
    }
}

impl Default for Visibility {
    fn default() -> Self {
        Self::new()
    }
}

/// 公会成员组件
#[derive(Debug, Clone)]
pub struct GuildMembership {
    pub guild_name: String,
    pub rank: u8,  // 0=会长, 1=副会长, 2=成员
}

impl GuildMembership {
    pub fn new(guild_name: String, rank: u8) -> Self {
        Self { guild_name, rank }
    }
    
    pub fn is_leader(&self) -> bool {
        self.rank == 0
    }
    
    pub fn is_officer(&self) -> bool {
        self.rank <= 1
    }
}

// ============================================================================
// 战斗属性组件 (Combat & Stats Components)
// ============================================================================
// 
// 注意: Health, Mana, CombatStats 等战斗组件已移到 combat.rs
// 使用时通过 use crate::ecs::components::combat::* 导入
//
// ============================================================================

// ============================================================================
// Player State Machine - 玩家状态机
// ============================================================================
//
// 使用状态机模式管理玩家行为,确保状态转换的正确性和可预测性
//
// **状态图**:
// ```
//     Idle ──┬──> Walking ──> Idle
//            ├──> Running ──> Idle
//            ├──> Attacking ──> Idle
//            ├──> Casting ──> Idle
//            ├──> Hit ──> Idle
//            └──> Dead (终态)
// ```
//
// ============================================================================
