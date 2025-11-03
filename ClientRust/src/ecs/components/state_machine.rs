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

use std::time::Instant;

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
    /// 获取状态对应的动画帧数
    pub fn frame_count(&self) -> i32 {
        match self {
            PlayerState::Idle => 4,
            PlayerState::Walking => 6,
            PlayerState::Running => 6,
            PlayerState::Attacking => 6,
            PlayerState::Casting => 6,
            PlayerState::Hit => 2,
            PlayerState::Dead => 10,
        }
    }

    /// 获取状态对应的帧间隔(毫秒)
    pub fn frame_interval(&self) -> i32 {
        match self {
            PlayerState::Idle => 300,      // 站立动画慢一点
            PlayerState::Walking => 100,   // 行走速度
            PlayerState::Running => 83,    // 跑步更快
            PlayerState::Attacking => 100,
            PlayerState::Casting => 120,
            PlayerState::Hit => 150,
            PlayerState::Dead => 200,
        }
    }

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

impl PlayerStateMachine {
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
