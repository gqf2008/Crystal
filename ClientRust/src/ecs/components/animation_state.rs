// ============================================================================
// Animation State Component - 动画状态组件
// ============================================================================
//
// 用于解耦动画状态决策和动画播放
//
// ============================================================================

use std::time::Instant;
use std::collections::VecDeque;

/// 动作类型枚举 (对应 C# MapObject.Action)
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ActionType {
    /// 站立
    Standing,
    /// 行走
    Walking,
    /// 奔跑
    Running,
    /// 攻击1
    Attack1,
    /// 攻击2
    Attack2,
    /// 攻击3
    Attack3,
    /// 受击
    Struck,
    /// 死亡
    Die,
    /// 已死亡
    Dead,
    /// 施法
    Spell,
    /// 采集
    Harvest,
    /// 魔法施放
    MagicCast,
}

impl ActionType {
    /// 获取动作的帧数
    pub fn frame_count(&self) -> u32 {
        match self {
            ActionType::Standing => 4,
            ActionType::Walking => 6,
            ActionType::Running => 6,
            ActionType::Attack1 | ActionType::Attack2 | ActionType::Attack3 => 6,
            ActionType::Struck => 2,
            ActionType::Die => 10,
            ActionType::Dead => 1,
            ActionType::Spell | ActionType::MagicCast => 10,
            ActionType::Harvest => 8,
        }
    }

    /// 是否循环播放
    pub fn is_looping(&self) -> bool {
        matches!(
            self,
            ActionType::Standing | ActionType::Walking | ActionType::Running | ActionType::Harvest
        )
    }
}

/// 排队的动作 (用于动作队列)
#[derive(Debug, Clone)]
pub struct QueuedAction {
    /// 动作类型
    pub action: ActionType,
    /// 动作方向 (可选)
    pub direction: Option<u8>,
    /// 目标位置 (可选)
    pub target_position: Option<(i32, i32)>,
    /// 优先级 (数字越大优先级越高)
    pub priority: u8,
}

impl QueuedAction {
    pub fn new(action: ActionType) -> Self {
        Self {
            action,
            direction: None,
            target_position: None,
            priority: 0,
        }
    }

    pub fn with_direction(mut self, direction: u8) -> Self {
        self.direction = Some(direction);
        self
    }

    pub fn with_target(mut self, position: (i32, i32)) -> Self {
        self.target_position = Some(position);
        self
    }

    pub fn with_priority(mut self, priority: u8) -> Self {
        self.priority = priority;
        self
    }
}

/// 动画状态枚举
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AnimationState {
    /// 站立
    Idle,
    /// 行走
    Walk,
    /// 奔跑
    Run,
    /// 攻击
    Attack,
    /// 受伤
    Hit,
    /// 死亡
    Die,
    /// 施法
    Spell,
    /// 采集
    Harvest,
}

impl AnimationState {
    /// 获取动画帧数
    pub fn frame_count(&self) -> u8 {
        match self {
            AnimationState::Idle => 4,
            AnimationState::Walk => 6,
            AnimationState::Run => 6,
            AnimationState::Attack => 6,
            AnimationState::Hit => 2,
            AnimationState::Die => 10,
            AnimationState::Spell => 10,
            AnimationState::Harvest => 8,
        }
    }
    
    /// 获取动画帧间隔（帧数）
    /// 动画播放速度固定,不受velocity影响
    /// 
    /// 🎯 大幅加快动画播放速度以匹配快速位移(96/144 px/s):
    /// - Walk: 3帧间隔 = 6帧×0.05s = 0.3s/循环
    /// - Run: 1帧间隔 = 6帧×0.017s = 0.1s/循环
    pub fn frame_interval(&self) -> u32 {
        match self {
            AnimationState::Idle => 10,      // 站立：慢速播放
            AnimationState::Walk => 3,       // 🎯 更快走路动画 (4→3)
            AnimationState::Run => 1,        // 🎯 极快跑步动画 (2→1)
            AnimationState::Attack => 4,
            AnimationState::Hit => 8,
            AnimationState::Die => 10,
            AnimationState::Spell => 5,
            AnimationState::Harvest => 6,
        }
    }
    
    /// 是否循环播放
    pub fn is_looping(&self) -> bool {
        matches!(
            self,
            AnimationState::Idle | AnimationState::Walk | AnimationState::Run | AnimationState::Harvest
        )
    }
}

/// 动画状态组件 - 决策层
#[derive(Debug, Clone)]
pub struct AnimationControl {
    /// 当前动画状态
    pub current_state: AnimationState,
    
    /// 上一个动画状态
    pub previous_state: AnimationState,
    
    /// 状态切换时间
    pub state_change_time: Instant,
    
    /// 过渡时间（秒）
    pub transition_duration: f32,
    
    /// 是否强制切换（忽略当前动画是否完成）
    pub force_change: bool,
    
    /// 动画方向（0-7）
    pub direction: u8,
    
    /// 是否循环播放
    pub loop_animation: bool,
    
    /// 当前帧索引（由播放系统更新）
    pub current_frame: u8,
    
    /// 🎯 帧计时器：累积的时间，用于判断何时切换到下一帧
    /// **动画播放速度固定**,不受velocity或speed_scale影响
    pub frame_timer: f32,
}

impl AnimationControl {
    pub fn new() -> Self {
        Self {
            current_state: AnimationState::Idle,
            previous_state: AnimationState::Idle,
            state_change_time: Instant::now(),
            transition_duration: 0.1,
            force_change: false,
            direction: 0,
            loop_animation: true,
            current_frame: 0,
            frame_timer: 0.0, // 🎯 帧计时器初始化为0,动画速度固定
        }
    }
    
    /// 设置动画状态（默认非强制）
    pub fn set_state(&mut self, state: AnimationState) {
        self.set_state_with_force(state, false);
    }
    
    /// 设置动画状态（可指定是否强制）
    pub fn set_state_with_force(&mut self, state: AnimationState, force: bool) {
        if self.current_state != state || force {
            self.previous_state = self.current_state;
            self.current_state = state;
            self.state_change_time = Instant::now();
            self.force_change = force;
            self.current_frame = 0; // 重置帧索引
            self.frame_timer = 0.0; // 🎯 重置帧计时器
            self.loop_animation = state.is_looping(); // 根据状态设置是否循环
        }
    }
    
    /// 获取状态持续时间（秒）
    pub fn state_duration(&self) -> f32 {
        self.state_change_time.elapsed().as_secs_f32()
    }
    
    /// 是否在过渡中
    pub fn is_transitioning(&self) -> bool {
        self.state_duration() < self.transition_duration
    }
    
    /// 检查当前动画是否播放完毕
    pub fn is_finished(&self) -> bool {
        self.current_frame >= self.current_state.frame_count() - 1
    }
}

impl Default for AnimationControl {
    fn default() -> Self {
        Self::new()
    }
}

