// ============================================================================
// GameState - 游戏状态管理器
// ============================================================================
//
// 职责：
// - 管理当前场景
// - 游戏主循环（run）
// - 监听场景切换请求

use crate::core::GameError;
use crate::network::NetworkEvent;
use crate::scenes::*;
use macroquad::prelude::*;

// 重导出常用类型
pub use crate::coord::{Coord, MapUtils};
pub use crate::compat::{MapLoader, PathFinder};
pub use macroquad::prelude::{KeyCode, MouseButton};

/// GameResult 类型别名 (替代 ggez::GameResult)
pub type GameResult<T = ()> = Result<T, GameError>;

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

/// 游戏主状态
pub struct GameState {
    /// 当前场景
    current_scene: SceneKind,
}

impl GameState {
    /// 创建游戏状态
    pub async fn new() -> GameResult<Self> {
        // 加载字体
        let font_data = include_bytes!("../assets/fonts/AlibabaPuHuiTi-3-55-Regular.ttf");
        let _font = load_ttf_font_from_bytes(font_data)
            .map_err(|e| GameError::ResourceLoadError(format!("字体加载失败: {}", e)))?;
        
        // 创建初始场景（登录）
        let mut initial_scene = SceneKind::Login(LoginScene::new());
        initial_scene.on_enter()?;
        
        Ok(Self {
            current_scene: initial_scene,
        })
    }
    
    /// 游戏主循环
    pub async fn run(mut self) -> GameResult {
        println!("🎮 游戏启动: {}", self.current_scene.name());
        
        loop {
            let dt = get_frame_time();
            
            // 处理输入
            self.current_scene.handle_input()?;
            
            // 更新场景，获取切换请求
            let transition = self.current_scene.update(dt)?;
            
            // 渲染场景
            self.current_scene.render()?;
            
            // 处理场景切换
            match transition {
                SceneTransition::None => {
                    // 继续当前场景
                }
                SceneTransition::Exit => {
                    println!("👋 游戏退出");
                    break;
                }
                other => {
                    // 切换场景
                    self.switch_to(other)?;
                }
            }
            
            next_frame().await;
        }
        
        Ok(())
    }
    
    /// 切换场景
    fn switch_to(&mut self, transition: SceneTransition) -> GameResult {
        // 离开当前场景
        self.current_scene.on_exit()?;
        
        // 创建新场景
        let mut new_scene = match transition {
            SceneTransition::Login => SceneKind::Login(LoginScene::new()),
            SceneTransition::CharacterSelect => SceneKind::CharacterSelect(SelectScene::new(vec![])?),
            SceneTransition::Game => SceneKind::Game(GameScene::new()),
            SceneTransition::Loading => SceneKind::Loading(LoadingScene::new()),
            SceneTransition::None | SceneTransition::Exit => {
                return Ok(());
            }
        };
        
        println!("🎬 场景切换: {} → {}", self.current_scene.name(), new_scene.name());
        
        // 进入新场景
        new_scene.on_enter()?;
        
        // 替换场景
        self.current_scene = new_scene;
        
        Ok(())
    }
}

