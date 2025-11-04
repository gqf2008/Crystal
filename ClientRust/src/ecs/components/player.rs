// ============================================================================
// 玩家专用组件
// ============================================================================

pub use mir2_shared::{MirClass, MirGender};

/// 玩家数据组件 (标记这是玩家实体)
#[derive(Debug, Clone)]
pub struct PlayerData {
    pub id: u32,
    pub name: String,
    pub class: MirClass,
    pub gender: MirGender,
    pub level: u16,           // ➕ 玩家等级
    pub exp: i64,
    pub max_experience: i64,  // ➕ 升级所需经验
    pub gold: u32,
    pub credit: u32,          // ➕ 元宝/点数
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
/// - 不重复其他组件的数据（渲染用Animation组件，移动用Path/Velocity组件）
/// 
/// **数据所有权**：
/// - `direction`: 由 MovementSystem 根据移动方向更新
/// - `action`: 由 PlayerControlSystem 根据用户输入独占写入 (单一来源原则)
#[derive(Debug, Clone)]
pub struct Player {
    /// 面向方向 (0-7 八方向)
    pub direction: u8,
    /// 当前动作状态（行走/跑步/站立/攻击）
    /// 
    /// **重要**: 此字段只能由 PlayerControlSystem 写入！
    /// 其他系统(MovementSystem等)只能读取，不能修改
    pub action: PlayerAction,
    /// 是否正在移动
    pub is_moving: bool,
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
    
    pub fn frame_interval(&self) -> i32 {
        match self {
            PlayerAction::Stand => 30,
            // 🎯 调整动画速度以匹配移动速度和视觉效果
            // Walk: 4帧间隔 = 6帧×4tick = 24tick/循环 = 2.4s
            PlayerAction::Walk => 4,
            // Run: 2帧间隔 = 6帧×2tick = 12tick/循环 = 1.2s  
            PlayerAction::Run => 2,
            // Attack: 1帧间隔 = 6帧×1tick = 6tick = 0.6s (保持攻击动画流畅)
            PlayerAction::Attack1 => 1,
            PlayerAction::Attack2 => 1,
            PlayerAction::Attack3 => 1,
        }
    }
    
    /// 计算动画总时长 (毫秒)
    /// animation_count 每 100ms 增加 1
    pub fn duration_ms(&self) -> u64 {
        (self.frame_count() * self.frame_interval() * 100) as u64
    }
    
    /// 是否是攻击动作
    pub fn is_attack(&self) -> bool {
        matches!(self, PlayerAction::Attack1 | PlayerAction::Attack2 | PlayerAction::Attack3)
    }
    
    pub fn frame_start(&self) -> i32 {
        match self {
            PlayerAction::Stand => 0,
            PlayerAction::Walk => 32,
            PlayerAction::Run => 80,
            // C# Attack1: Start=128, Count=6, Interval=100
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
