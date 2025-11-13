// ============================================================================
// 游戏核心类型定义和重导出
// ============================================================================
//
// 此模块提供:
// - 游戏上下文类型 (GameContext, GameResult)
// - 常用类型重导出 (避免重复导入)
// - ECS 常量定义 (特殊实体 ID)

// 重新导出核心类型
pub use crate::network::handlers::NetworkEvent;

// 重新导出坐标系统
pub use crate::coord::{Coord, ViewportConfig, MapUtils, CameraController, CELL_WIDTH, CELL_HEIGHT};

/// GameResult 类型别名 (替代 ggez::GameResult)
pub type GameResult<T = ()> = Result<T, GameError>;

/// GameError 类型 (替代 ggez::GameError)
pub use crate::core::GameError;

/// 输入状态包装器（简化版）
pub struct InputState {
    pub mouse: MouseState,
    pub events: EventIterator,
}

impl InputState {
    pub fn new() -> Self {
        Self {
            mouse: MouseState,
            events: EventIterator,
        }
    }
    
    pub fn key_pressed(&self, key: KeyCode) -> bool {
        macroquad::prelude::is_key_pressed(key)
    }
    
    pub fn mouse_left_pressed(&self) -> bool {
        macroquad::prelude::is_mouse_button_pressed(MouseButton::Left)
    }
    
    pub fn mouse_right_pressed(&self) -> bool {
        macroquad::prelude::is_mouse_button_pressed(MouseButton::Right)
    }
    
    pub fn mouse_middle_pressed(&self) -> bool {
        macroquad::prelude::is_mouse_button_pressed(MouseButton::Middle)
    }
    
    pub fn mouse_position(&self) -> (f32, f32) {
        macroquad::prelude::mouse_position()
    }
    
    pub fn ctrl_pressed(&self) -> bool {
        macroquad::prelude::is_key_down(KeyCode::LeftControl) 
            || macroquad::prelude::is_key_down(KeyCode::RightControl)
    }
    
    pub fn mouse_wheel(&self) -> (f32, f32) {
        macroquad::prelude::mouse_wheel()
    }
    
    // 兼容字段访问
    pub fn button_pressed(&self, button: MouseButton) -> bool {
        macroquad::prelude::is_mouse_button_pressed(button)
    }
}

#[derive(Clone, Copy)]
pub struct MouseState;

impl MouseState {
    pub fn button_pressed(&self, button: MouseButton) -> bool {
        macroquad::prelude::is_mouse_button_pressed(button)
    }
    
    pub fn position(&self) -> Vec2 {
        let (x, y) = macroquad::prelude::mouse_position();
        Vec2::new(x, y)
    }
}

#[derive(Clone, Copy)]
pub struct EventIterator;

impl Iterator for EventIterator {
    type Item = GameEvent;
    
    fn next(&mut self) -> Option<Self::Item> {
        None  // 暂时返回空
    }
}

impl EventIterator {
    pub fn iter(&self) -> Self {
        *self
    }
}

pub struct GameEvent;


/// GameContext - 游戏运行时上下文
/// 
/// 职责：
/// - 管理 ECS 世界（实体和组件）
/// - 提供全局服务（网络、资源、事件）
/// - 记录时间状态
pub struct GameContext {
    pub world: hecs::World,                     // ECS 世界
    pub network: crate::components::network::NetworkContext,  // 网络服务
    pub settings: crate::components::settings::Settings,      // 设置
    pub resources: crate::resources::ResourceManager,         // 资源管理
    pub events: crate::event_bus::EventBus,                   // 事件总线
    pub delta_time: f32,                        // 帧时间
    pub start_time: std::time::Instant,         // 启动时间
}

impl GameContext {
    pub fn new() -> Self {
        Self {
            world: hecs::World::new(),
            network: crate::components::network::NetworkContext::new(),
            settings: crate::components::settings::Settings::default(),
            resources: crate::resources::ResourceManager::new(),
            events: crate::event_bus::EventBus::new(),
            delta_time: 0.0,
            start_time: std::time::Instant::now(),
        }
    }
    
    pub fn input(&self) -> InputState {
        InputState::new()
    }
    
    pub fn map_events(&mut self) -> &[NetworkEvent] {
        // macroquad 自动处理事件，暂时返回空切片
        &[]
    }
    
    pub fn drawable_size(&self) -> (f32, f32) {
        (macroquad::prelude::screen_width(), macroquad::prelude::screen_height())
    }
    
    /// 获取事件总线（引用）
    pub fn events(&self) -> &crate::event_bus::EventBus {
        &self.events
    }
    
    /// 获取可变事件总线
    pub fn events_mut(&mut self) -> &mut crate::event_bus::EventBus {
        &mut self.events
    }
    
    /// 获取网络上下文（引用）
    pub fn network(&self) -> &crate::components::network::NetworkContext {
        &self.network
    }
    
    /// 获取可变网络上下文
    pub fn network_mut(&mut self) -> &mut crate::components::network::NetworkContext {
        &mut self.network
    }
    
    /// 获取资源管理器（引用）
    pub fn resources(&self) -> &crate::resources::ResourceManager {
        &self.resources
    }
    
    /// 获取可变资源管理器
    pub fn resources_mut(&mut self) -> &mut crate::resources::ResourceManager {
        &mut self.resources
    }
    
    /// 获取设置（引用）
    pub fn settings(&self) -> &crate::components::settings::Settings {
        &self.settings
    }
    
    /// 获取可变设置
    pub fn settings_mut(&mut self) -> &mut crate::components::settings::Settings {
        &mut self.settings
    }
    
    /// 清理死亡实体
    pub fn cleanup_dead_entities(&mut self) {
        use crate::components::core::Dead;
        let mut to_remove = Vec::new();
        
        for (id, _dead) in self.world.query::<&Dead>().iter() {
            to_remove.push(id);
        }
        
        for id in to_remove {
            let _ = self.world.despawn(id);
        }
    }
}

/// 图形上下文 (macroquad 不需要,但为了兼容性保留)
pub struct GraphicsContext;

impl GraphicsContext {
    pub fn drawable_size(&self) -> (f32, f32) {
        (macroquad::prelude::screen_width(), macroquad::prelude::screen_height())
    }
}

/// Canvas (macroquad 不需要,但为了兼容性保留)
pub struct Canvas;

impl Canvas {
    pub fn draw(&mut self, _drawable: &impl std::fmt::Debug, _param: DrawParam) {
        // macroquad 使用全局绘制函数，这里是空实现
    }
}

// ============================================================================
// ggez 图形类型的 macroquad 映射
// ============================================================================

pub use macroquad::prelude::Color;
pub use macroquad::prelude::Vec2 as Point2;

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

/// Rect 类型别名
pub use macroquad::prelude::Rect;

/// Mesh 占位符 (macroquad 使用不同的渲染方式)
pub struct Mesh;

/// DrawMode 占位符
#[derive(Debug, Clone, Copy)]
pub enum DrawMode {
    Fill,
    Stroke(f32),
}

/// Text 占位符 (macroquad 使用不同的文字渲染)
pub struct Text {
    pub content: String,
}

impl Text {
    pub fn new(content: impl Into<String>) -> Self {
        Self {
            content: content.into(),
        }
    }
}

/// TextFragment 占位符
pub struct TextFragment {
    pub text: String,
    pub color: Option<Color>,
}

impl From<&str> for TextFragment {
    fn from(s: &str) -> Self {
        Self {
            text: s.to_string(),
            color: None,
        }
    }
}

impl From<String> for TextFragment {
    fn from(s: String) -> Self {
        Self {
            text: s,
            color: None,
        }
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
// 特殊实体 ID (ECS常量) - 已废弃，network 和 settings 现在直接在 GameWorld 里
// ============================================================================

// 网络实体 ID（已废弃）
// pub const NETWORK_ENTITY: hecs::Entity = ...;

// 设置实体 ID（已废弃）
// pub const SETTING_ENTITY: hecs::Entity = ...;

// ============================================================================
// MapLoader 占位符
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

