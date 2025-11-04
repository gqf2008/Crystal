// ============================================================================
// 玩家专用组件
// ============================================================================

pub use mir2_shared::{MirClass, MirGender};
use std::time::{Duration, Instant};

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

/// 角色组件 - 查看器中的可控角色
#[derive(Debug, Clone)]
pub struct Player {
    pub direction: u8,  // 0-7 八方向
    pub action: PlayerAction,
    pub frame_index: i32,
    pub frame_time: i32,
    pub speed: f32,
    pub target_x: f32,
    pub target_y: f32,
    pub is_moving: bool,
    pub path: Vec<(i32, i32)>,
    pub path_index: usize,
    pub move_mode: MoveMode,
    pub last_move_time: Instant,  // 上次发送移动命令的时间
    pub move_delay: Duration,     // 移动命令间隔(服务器MoveDelay=600ms)
    pub waiting_server_confirm: bool,        // 🎯 等待服务器确认移动
    // 🎯 碰撞调试信息
    pub collision_detected: bool,  // 是否检测到碰撞
    pub collision_target_grid: Option<(i32, i32)>,  // 碰撞的目标格子
    // 🎯 走/跑机制
    pub can_run: bool,            // 是否允许跑步（需要先走路才能设置为true）
    pub last_run_time: Instant,   // 上次跑步/走路的时间
    pub run_cooldown: Duration,   // 跑步冷却时间（900ms）
}

/// 角色动作
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum PlayerAction {
    Stand = 0,
    Walk = 1,
    Run = 2,
}

impl PlayerAction {
    pub fn frame_count(&self) -> i32 {
        match self {
            PlayerAction::Stand => 4,
            PlayerAction::Walk => 6,
            PlayerAction::Run => 6,
        }
    }
    
    pub fn frame_interval(&self) -> i32 {
        match self {
            PlayerAction::Stand => 30,
            // 🎯 加快动画以匹配移动速度 (60/90 px/s)
            // Walk: 3帧间隔 = 6帧×3tick = 18tick/循环 = 0.3s
            PlayerAction::Walk => 3,
            // Run: 2帧间隔 = 6帧×2tick = 12tick/循环 = 0.2s  
            PlayerAction::Run => 2,
        }
    }
    
    pub fn frame_start(&self) -> i32 {
        match self {
            PlayerAction::Stand => 0,
            PlayerAction::Walk => 32,
            PlayerAction::Run => 80,
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

/// 玩家状态枚举
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PlayerState {
    /// 空闲状态 - 站立不动
    Idle,
    /// 行走状态 - 慢速移动
    Walking,
    /// 奔跑状态 - 快速移动
    Running,
    /// 攻击状态 - 执行攻击动作
    Attacking,
    /// 施法状态 - 释放技能
    Casting,
    /// 受击状态 - 被攻击
    Hit,
    /// 死亡状态 - 角色死亡(终态)
    Dead,
}

impl PlayerState {
    /// 检查是否可以转换到目标状态
    pub fn can_transition_to(&self, target: PlayerState) -> bool {
        match (self, target) {
            // 死亡是终态,不能转换到其他状态
            (PlayerState::Dead, _) => false,
            
            // 任何状态都可以转换到死亡
            (_, PlayerState::Dead) => true,
            
            // 受击状态只能转换到空闲或死亡
            (PlayerState::Hit, PlayerState::Idle) => true,
            (PlayerState::Hit, _) => false,
            
            // 攻击和施法状态只能转换到空闲
            (PlayerState::Attacking, PlayerState::Idle) => true,
            (PlayerState::Attacking, _) => false,
            (PlayerState::Casting, PlayerState::Idle) => true,
            (PlayerState::Casting, _) => false,
            
            // 移动状态可以互相转换和转换到空闲
            (PlayerState::Walking, PlayerState::Idle) => true,
            (PlayerState::Walking, PlayerState::Running) => true,
            (PlayerState::Walking, PlayerState::Walking) => true,
            (PlayerState::Running, PlayerState::Idle) => true,
            (PlayerState::Running, PlayerState::Walking) => true,
            (PlayerState::Running, PlayerState::Running) => true,
            
            // 空闲状态可以转换到任何状态(除了死亡已经在上面处理了)
            (PlayerState::Idle, _) => true,
            
            // 其他转换不允许
            _ => false,
        }
    }

    /// 是否是移动状态
    pub fn is_moving(&self) -> bool {
        matches!(self, PlayerState::Walking | PlayerState::Running)
    }

    /// 是否是动作状态(攻击、施法等需要完整播放的动作)
    pub fn is_action(&self) -> bool {
        matches!(self, PlayerState::Attacking | PlayerState::Casting | PlayerState::Hit)
    }
}

/// 玩家状态机组件
#[derive(Debug, Clone)]
pub struct PlayerStateMachine {
    /// 当前状态
    pub current_state: PlayerState,
    /// 上一个状态(用于恢复)
    pub previous_state: PlayerState,
    /// 状态进入时间
    pub state_enter_time: Instant,
    /// 是否正在转换状态
    pub is_transitioning: bool,
}

impl PlayerStateMachine {
    pub fn new() -> Self {
        Self {
            current_state: PlayerState::Idle,
            previous_state: PlayerState::Idle,
            state_enter_time: Instant::now(),
            is_transitioning: false,
        }
    }

    /// 请求转换到新状态
    pub fn transition_to(&mut self, new_state: PlayerState) -> bool {
        if !self.current_state.can_transition_to(new_state) {
            tracing::warn!(
                "❌ 非法状态转换: {:?} -> {:?}",
                self.current_state,
                new_state
            );
            return false;
        }

        if self.current_state != new_state {
            tracing::debug!(
                "🔄 状态转换: {:?} -> {:?}",
                self.current_state,
                new_state
            );
            self.previous_state = self.current_state;
            self.current_state = new_state;
            self.state_enter_time = Instant::now();
            self.is_transitioning = true;
        }

        true
    }

    /// 获取当前状态的持续时间(毫秒)
    pub fn state_duration(&self) -> u64 {
        self.state_enter_time.elapsed().as_millis() as u64
    }

    /// 恢复到上一个状态
    pub fn revert_to_previous(&mut self) {
        let prev = self.previous_state;
        self.transition_to(prev);
    }

    /// 完成状态转换
    pub fn complete_transition(&mut self) {
        self.is_transitioning = false;
    }

    /// 是否在指定状态
    pub fn is_in_state(&self, state: PlayerState) -> bool {
        self.current_state == state
    }

    /// 处理输入事件,自动转换状态
    pub fn handle_event(&mut self, event: PlayerInputEvent) {
        let new_state = match event {
            PlayerInputEvent::StartWalking => PlayerState::Walking,
            PlayerInputEvent::StartRunning => PlayerState::Running,
            PlayerInputEvent::StopMoving => PlayerState::Idle,
            PlayerInputEvent::Attack => PlayerState::Attacking,
            PlayerInputEvent::CastSpell => PlayerState::Casting,
            PlayerInputEvent::TakeDamage => PlayerState::Hit,
            PlayerInputEvent::Die => PlayerState::Dead,
        };

        self.transition_to(new_state);
    }
}

impl Default for PlayerStateMachine {
    fn default() -> Self {
        Self::new()
    }
}

/// 输入事件枚举 - 驱动状态转换
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum PlayerInputEvent {
    /// 开始行走
    StartWalking,
    /// 开始奔跑
    StartRunning,
    /// 停止移动
    StopMoving,
    /// 执行攻击
    Attack,
    /// 施放技能
    CastSpell,
    /// 受到攻击
    TakeDamage,
    /// 死亡
    Die,
}
