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
use mir2_shared::SelectInfo;

// 重导出常用类型
pub use crate::compat::{MapLoader, PathFinder};
pub use crate::coord::{Coord, MapUtils};
pub use macroquad::prelude::{KeyCode, MouseButton};

/// GameResult 类型别名 (替代 ggez::GameResult)
pub type GameResult<T = ()> = Result<T, GameError>;

/// 每帧输入快照（macroquad 轮询输入的轻量包装）
///
/// 说明：这不是 ECS 组件 `components::InputState`。
/// ECS 的 `InputState` 用于“上一帧状态/边缘检测”。
pub struct FrameInput {
    enabled: bool,
    pub mouse: MouseState,
}

impl FrameInput {
    pub fn new(enabled: bool) -> Self {
        Self {
            enabled,
            mouse: MouseState { enabled },
        }
    }

    pub fn key_pressed(&self, key: KeyCode) -> bool {
        if !self.enabled {
            return false;
        }
        macroquad::prelude::is_key_pressed(key)
    }

    pub fn key_down(&self, key: KeyCode) -> bool {
        if !self.enabled {
            return false;
        }
        macroquad::prelude::is_key_down(key)
    }

    pub fn mouse_left_pressed(&self) -> bool {
        if !self.enabled {
            return false;
        }
        macroquad::prelude::is_mouse_button_pressed(MouseButton::Left)
    }

    pub fn mouse_left_down(&self) -> bool {
        if !self.enabled {
            return false;
        }
        macroquad::prelude::is_mouse_button_down(MouseButton::Left)
    }

    pub fn mouse_right_pressed(&self) -> bool {
        if !self.enabled {
            return false;
        }
        macroquad::prelude::is_mouse_button_pressed(MouseButton::Right)
    }

    pub fn mouse_right_down(&self) -> bool {
        if !self.enabled {
            return false;
        }
        macroquad::prelude::is_mouse_button_down(MouseButton::Right)
    }

    pub fn mouse_middle_pressed(&self) -> bool {
        if !self.enabled {
            return false;
        }
        macroquad::prelude::is_mouse_button_pressed(MouseButton::Middle)
    }

    pub fn mouse_middle_down(&self) -> bool {
        if !self.enabled {
            return false;
        }
        macroquad::prelude::is_mouse_button_down(MouseButton::Middle)
    }

    pub fn mouse_position(&self) -> (f32, f32) {
        if !self.enabled {
            return (0.0, 0.0);
        }
        macroquad::prelude::mouse_position()
    }

    pub fn ctrl_pressed(&self) -> bool {
        if !self.enabled {
            return false;
        }
        macroquad::prelude::is_key_down(KeyCode::LeftControl)
            || macroquad::prelude::is_key_down(KeyCode::RightControl)
    }

    pub fn mouse_wheel(&self) -> (f32, f32) {
        if !self.enabled {
            return (0.0, 0.0);
        }
        macroquad::prelude::mouse_wheel()
    }

    // 兼容字段访问
    pub fn button_pressed(&self, button: MouseButton) -> bool {
        if !self.enabled {
            return false;
        }
        macroquad::prelude::is_mouse_button_pressed(button)
    }

    pub fn button_down(&self, button: MouseButton) -> bool {
        if !self.enabled {
            return false;
        }
        macroquad::prelude::is_mouse_button_down(button)
    }
}

#[derive(Clone, Copy)]
pub struct MouseState {
    enabled: bool,
}

impl MouseState {
    pub fn button_pressed(&self, button: MouseButton) -> bool {
        if !self.enabled {
            return false;
        }
        macroquad::prelude::is_mouse_button_pressed(button)
    }

    pub fn button_down(&self, button: MouseButton) -> bool {
        if !self.enabled {
            return false;
        }
        macroquad::prelude::is_mouse_button_down(button)
    }

    pub fn position(&self) -> Vec2 {
        if !self.enabled {
            return Vec2::new(0.0, 0.0);
        }
        let (x, y) = macroquad::prelude::mouse_position();
        Vec2::new(x, y)
    }
}

/// GameContext - 游戏运行时上下文
///
/// 职责：
/// - 管理 ECS 世界（实体和组件）
/// - 提供全局服务（网络、资源、事件）
/// - 记录时间状态
pub struct GameContext {
    pub world: hecs::World,                                  // ECS 世界
    pub network: crate::components::network::NetworkContext, // 网络服务
    /// 真实网络连接（双线程 NetContext）。
    ///
    /// - `None`: 当前未连接服务器（例如 test_game_scene）
    /// - `Some`: NetworkSystem 会从这里拉取入站事件并写入 EventBus
    pub net: Option<crate::network::NetContext>,
    pub settings: crate::components::settings::Settings, // 设置

    pub events: crate::event_bus::EventBus, // 事件总线
    pub delta_time: f32,                    // 帧时间
    pub start_time: std::time::Instant,     // 启动时间

    /// 会话/进场状态（跨帧保留 StartGame* 等关键结果）
    pub session: crate::components::SessionState,

    /// 是否屏蔽本帧 ECS 输入读取（用于 UI 交互期间防止误触）
    pub input_blocked: bool,
}

impl Default for GameContext {
    fn default() -> Self {
        Self::new()
    }
}

impl GameContext {
    pub fn new() -> Self {
        Self {
            world: hecs::World::new(),
            network: crate::components::network::NetworkContext::new(),
            net: None,
            settings: crate::components::settings::Settings::default(),

            events: crate::event_bus::EventBus::new(),
            delta_time: 0.0,
            start_time: std::time::Instant::now(),

            session: crate::components::SessionState::default(),

            input_blocked: false,
        }
    }

    pub fn input(&self) -> FrameInput {
        FrameInput::new(!self.input_blocked)
    }

    pub fn map_events(&mut self) -> &[NetworkEvent] {
        // macroquad 自动处理事件，暂时返回空切片
        &[]
    }

    pub fn drawable_size(&self) -> (f32, f32) {
        (
            macroquad::prelude::screen_width(),
            macroquad::prelude::screen_height(),
        )
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

    /// 获取网络连接上下文（如果已连接）
    pub fn net(&self) -> Option<&crate::network::NetContext> {
        self.net.as_ref()
    }

    /// 设置/替换网络连接上下文
    pub fn set_net(&mut self, net: crate::network::NetContext) {
        self.net = Some(net);
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

        for eref in self.world.iter() {
            if eref.get::<&Dead>().is_some() {
                to_remove.push(eref.entity());
            }
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
                    // 切换场景（异步）
                    self.switch_to(other).await?;
                }
            }

            next_frame().await;
        }

        Ok(())
    }

    /// 切换场景（异步版本）
    async fn switch_to(&mut self, transition: SceneTransition) -> GameResult {
        // 离开当前场景
        self.current_scene.on_exit()?;

        // 创建新场景
        let mut new_scene = match transition {
            SceneTransition::Login => SceneKind::Login(LoginScene::new()),
            SceneTransition::CharacterSelect => {
                // 来自 LoginScene 的真实角色列表（无则回退到最小示例，保证离线可跑）
                let characters: Vec<CharacterInfo> = match crate::network::take_global_characters()
                {
                    Some(list) => {
                        // 写回全局：保证返回选角时还能继续显示真实角色
                        crate::network::set_global_characters(list.clone());
                        list.into_iter()
                            .map(select_info_to_character_info)
                            .collect()
                    }
                    None => vec![CharacterInfo {
                        index: 0,
                        name: "测试角色".to_string(),
                        level: 1,
                        class: 0,
                        gender: 0,
                        last_access: "刚刚".to_string(),
                    }],
                };
                SceneKind::CharacterSelect(SelectScene::new(characters)?)
            }
            SceneTransition::Game => {
                // 创建游戏场景并异步加载纹理
                let mut scene = GameScene::new();
                scene.load_textures();
                SceneKind::Game(scene)
            }
            SceneTransition::Loading => SceneKind::Loading(LoadingScene::new()),
            SceneTransition::None | SceneTransition::Exit => {
                return Ok(());
            }
        };

        println!(
            "🎬 场景切换: {} → {}",
            self.current_scene.name(),
            new_scene.name()
        );

        // 进入新场景
        new_scene.on_enter()?;

        // 替换场景
        self.current_scene = new_scene;

        Ok(())
    }
}

fn select_info_to_character_info(info: SelectInfo) -> CharacterInfo {
    let last_access = info.last_access.format("%Y-%m-%d %H:%M").to_string();

    CharacterInfo {
        index: info.index,
        name: info.name,
        level: info.level,
        class: info.class as u8,
        gender: info.gender as u8,
        last_access,
    }
}
