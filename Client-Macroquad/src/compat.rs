// ============================================================================
// 兼容层 - macroquad 与 ggez 类型映射
// ============================================================================
//
// 此模块职责:
// 1. 提供 ggez 到 macroquad 的类型映射和适配
// 2. 游戏核心上下文 (GameContext) 管理
// 3. 常用类型重导出,简化导入
//
// 注意: 此文件仅包含必要的兼容代码,已移除未使用的占位符

// ============================================================================
// 核心类型重导出
// ============================================================================

pub use crate::network::handlers::NetworkEvent;
pub use crate::coord::{Coord, MapUtils};

/// GameResult 类型别名 (替代 ggez::GameResult)
pub type GameResult<T = ()> = Result<T, GameError>;

/// GameError 类型 (替代 ggez::GameError)
pub use crate::core::GameError;


// ============================================================================
// ggez 图形类型的 macroquad 映射
// ============================================================================

pub use macroquad::prelude::Color;

/// GraphicsContext 占位符 (macroquad不需要,仅用于trait兼容)
#[allow(dead_code)]
pub struct GraphicsContext;

impl GraphicsContext {
    pub fn drawable_size(&self) -> (f32, f32) {
        (macroquad::prelude::screen_width(), macroquad::prelude::screen_height())
    }
}

/// Canvas 占位符 (macroquad不需要,仅用于trait兼容)
#[allow(dead_code)]
pub struct Canvas;

impl Canvas {
    #[allow(dead_code)]
    pub fn draw(&mut self, _drawable: &impl std::fmt::Debug, _param: DrawParam) {
        // macroquad 使用全局绘制函数,此处为空实现
    }
}

/// DrawParam 兼容结构 (简化版)
#[derive(Debug, Clone, Copy)]
pub struct DrawParam {
    pub dest: Vec2,
    pub rotation: f32,
    pub scale: Vec2,
    pub color: Color,
}

impl Default for DrawParam {
    fn default() -> Self {
        Self {
            dest: Vec2::ZERO,
            rotation: 0.0,
            scale: Vec2::ONE,
            color: macroquad::prelude::WHITE,
        }
    }
}

use macroquad::prelude::Vec2;

impl DrawParam {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn dest(mut self, dest: Vec2) -> Self {
        self.dest = dest;
        self
    }

    pub fn color(mut self, color: Color) -> Self {
        self.color = color;
        self
    }

    pub fn scale(mut self, scale: Vec2) -> Self {
        self.scale = scale;
        self
    }
}

// ============================================================================
// 输入相关兼容
// ============================================================================

/// KeyCode 映射
pub use macroquad::prelude::KeyCode;

/// MouseButton 映射
pub use macroquad::prelude::MouseButton;

// ============================================================================
// MapLoader 和 PathFinder 占位符
// ============================================================================

/// MapLoader 占位符结构
pub struct MapLoader;

impl MapLoader {
    pub fn load_map(_world: &mut hecs::World, _reader: impl std::any::Any) -> GameResult<()> {
        // TODO: 实现地图加载
        Ok(())
    }
}

/// PathFinder 占位符结构
pub struct PathFinder;

impl PathFinder {
    pub fn new(_width: usize, _height: usize, _is_blocking: impl Fn(usize, usize) -> bool) -> Self {
        Self
    }
    
    pub fn find_path(&self, _start: (usize, usize), _end: (usize, usize)) -> Option<Vec<(usize, usize)>> {
        // TODO: 实现 A* 寻路
        None
    }
}

// ============================================================================
// 系统 Trait 已移到 systems/mod.rs
// ============================================================================

