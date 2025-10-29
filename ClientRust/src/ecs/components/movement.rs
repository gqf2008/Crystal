// ============================================================================
// Movement Components - 移动相关组件
// ============================================================================
//
// 符合ECS架构的移动组件设计
// - MovementVelocity: 速度（每帧移动量）
// - Path: 寻路路径
// - MovementState: 移动状态
//
// ============================================================================

use std::time::Instant;

/// 默认走路速度（像素/秒）
pub const DEFAULT_WALK_SPEED: f32 = 100.0;

/// 默认跑步速度（像素/秒）
pub const DEFAULT_RUN_SPEED: f32 = 180.0;

/// 默认最大速度（像素/秒）
pub const DEFAULT_MAX_SPEED: f32 = 300.0;

/// 速度组件 - 实体的移动速度（每帧）
#[derive(Debug, Clone)]
pub struct MovementVelocity {
    /// X轴速度（像素/帧）
    pub x: f32,
    /// Y轴速度（像素/帧）
    pub y: f32,
    /// 最大速度
    pub max_speed: f32,
    /// 走路速度（像素/秒）
    pub walk_speed: f32,
    /// 跑步速度（像素/秒）
    pub run_speed: f32,
}

impl MovementVelocity {
    pub fn new(max_speed: f32) -> Self {
        Self {
            x: 0.0,
            y: 0.0,
            max_speed,
            walk_speed: DEFAULT_WALK_SPEED,
            run_speed: DEFAULT_RUN_SPEED,
        }
    }
    
    /// 创建带自定义速度的移动组件
    pub fn with_speeds(max_speed: f32, walk_speed: f32, run_speed: f32) -> Self {
        Self {
            x: 0.0,
            y: 0.0,
            max_speed,
            walk_speed,
            run_speed,
        }
    }
    
    pub fn set(&mut self, x: f32, y: f32) {
        self.x = x;
        self.y = y;
        self.clamp();
    }
    
    pub fn magnitude(&self) -> f32 {
        (self.x * self.x + self.y * self.y).sqrt()
    }
    
    pub fn clamp(&mut self) {
        let mag = self.magnitude();
        if mag > self.max_speed {
            let scale = self.max_speed / mag;
            self.x *= scale;
            self.y *= scale;
        }
    }
    
    pub fn stop(&mut self) {
        self.x = 0.0;
        self.y = 0.0;
    }
}

/// 路径组件 - 存储寻路路径
#[derive(Debug, Clone)]
pub struct Path {
    /// 路径点列表（格子坐标）
    pub waypoints: Vec<(i32, i32)>,
    /// 当前路径点索引
    pub current_index: usize,
    /// 是否有效
    pub is_valid: bool,
}

impl Path {
    pub fn new() -> Self {
        Self {
            waypoints: Vec::new(),
            current_index: 0,
            is_valid: false,
        }
    }
    
    pub fn set_path(&mut self, waypoints: Vec<(i32, i32)>) {
        self.waypoints = waypoints;
        self.current_index = 0;
        self.is_valid = !self.waypoints.is_empty();
    }
    
    pub fn current_waypoint(&self) -> Option<(i32, i32)> {
        if self.current_index < self.waypoints.len() {
            Some(self.waypoints[self.current_index])
        } else {
            None
        }
    }
    
    pub fn advance(&mut self) -> bool {
        if self.current_index < self.waypoints.len() - 1 {
            self.current_index += 1;
            true
        } else {
            self.is_valid = false;
            false
        }
    }
    
    pub fn clear(&mut self) {
        self.waypoints.clear();
        self.current_index = 0;
        self.is_valid = false;
    }
    
    pub fn is_complete(&self) -> bool {
        !self.is_valid || self.current_index >= self.waypoints.len()
    }
}

/// 移动状态枚举
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MovementState {
    /// 静止
    Idle,
    /// 行走
    Walking,
    /// 奔跑
    Running,
    /// 被击退
    Knocked,
}

/// MovementType 别名 (用于系统兼容)
pub type MovementType = MovementState;

/// 移动状态组件 - 存储实体的移动状态
#[derive(Debug, Clone)]
pub struct Movement {
    pub state: MovementState,
    pub last_change_time: Instant,
}

impl Movement {
    pub fn new() -> Self {
        Self {
            state: MovementState::Idle,
            last_change_time: Instant::now(),
        }
    }
    
    pub fn set_state(&mut self, state: MovementState) {
        if self.state != state {
            self.state = state;
            self.last_change_time = Instant::now();
        }
    }
    
    pub fn is_moving(&self) -> bool {
        matches!(self.state, MovementState::Walking | MovementState::Running)
    }
}

impl Default for Movement {
    fn default() -> Self {
        Self::new()
    }
}
