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
            // Walk: 7帧间隔 → 7*6=42tick/循环 → 700ms/循环
            // 让动画更慢更自然
            PlayerAction::Walk => 7,
            // Run: 6帧间隔 → 6*6=36tick/循环 → 600ms/循环
            // 让跑步动画也慢一些
            PlayerAction::Run => 6,
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
