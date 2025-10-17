// Bevy Components - ECS 组件定义
use bevy::prelude::*;
use mir2_shared::MirDirection;

pub mod text_input;
pub use text_input::*;

/// 玩家组件标记
#[derive(Component)]
pub struct Player;

/// 网格坐标组件 (游戏逻辑坐标)
#[derive(Component, Debug, Clone, Copy)]
pub struct GridPosition {
    pub x: i32,
    pub y: i32,
}

impl GridPosition {
    pub fn new(x: i32, y: i32) -> Self {
        Self { x, y }
    }
}

/// 移动组件
#[derive(Component, Debug)]
pub struct Movement {
    pub direction: MirDirection,
    pub is_running: bool,
    pub speed: f32, // 移动速度(毫秒/格)
}

impl Movement {
    pub fn new() -> Self {
        Self {
            direction: MirDirection::Down,
            is_running: false,
            speed: 600.0, // 默认走路速度
        }
    }
    
    pub fn set_running(&mut self, running: bool) {
        self.is_running = running;
        self.speed = if running { 300.0 } else { 600.0 };
    }
}

/// 动画状态组件
#[derive(Component, Debug)]
pub struct AnimationState {
    pub current_frame: usize,
    pub frame_count: usize,
    pub frame_timer: f32,
    pub frame_duration: f32, // 每帧持续时间(秒)
}

impl AnimationState {
    pub fn new(frame_count: usize, fps: f32) -> Self {
        Self {
            current_frame: 0,
            frame_count,
            frame_timer: 0.0,
            frame_duration: 1.0 / fps,
        }
    }
    
    pub fn update(&mut self, delta: f32) {
        self.frame_timer += delta;
        if self.frame_timer >= self.frame_duration {
            self.frame_timer = 0.0;
            self.current_frame = (self.current_frame + 1) % self.frame_count;
        }
    }
}

/// 渲染偏移组件 (用于平滑移动动画)
#[derive(Component, Debug, Default)]
pub struct RenderOffset {
    pub x: f32,
    pub y: f32,
}
