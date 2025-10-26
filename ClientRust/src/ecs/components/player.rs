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
            PlayerAction::Walk => 6,
            PlayerAction::Run => 5,
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
