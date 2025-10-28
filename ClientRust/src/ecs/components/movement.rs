// ============================================================================
// Movement Components - 移动相关组件
// ============================================================================
//
// 符合ECS架构的移动组件设计
// - VelocityComponent: 速度（每帧移动量）
// - PathComponent: 寻路路径
// - MovementStateComponent: 移动状态
//
// ============================================================================

use std::time::Instant;

/// 速度组件 - 实体的移动速度（每帧）
#[derive(Debug, Clone)]
pub struct VelocityComponent {
    /// X轴速度（像素/帧）
    pub x: f32,
    /// Y轴速度（像素/帧）
    pub y: f32,
    /// 最大速度
    pub max_speed: f32,
}

impl VelocityComponent {
    pub fn new(max_speed: f32) -> Self {
        Self {
            x: 0.0,
            y: 0.0,
            max_speed,
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
pub struct PathComponent {
    /// 路径点列表（格子坐标）
    pub waypoints: Vec<(i32, i32)>,
    /// 当前路径点索引
    pub current_index: usize,
    /// 是否有效
    pub is_valid: bool,
}

impl PathComponent {
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

/// 移动状态组件
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

#[derive(Debug, Clone)]
pub struct MovementStateComponent {
    pub state: MovementState,
    pub last_change_time: Instant,
}

impl MovementStateComponent {
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

impl Default for MovementStateComponent {
    fn default() -> Self {
        Self::new()
    }
}
