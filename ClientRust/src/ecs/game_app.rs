// ============================================================================
// 游戏主应用 - 基于 ECS 架构
// ============================================================================
//
// 管理整个游戏的生命周期：
// - 场景切换（登录 → 选择角色 → 游戏）
// - ECS World 管理
// - 网络连接管理
// - 资源加载
//
// ============================================================================

use ggez::{Context, GameResult};
use ggez::event::EventHandler;
use ggez::graphics::{Canvas, Color};
use hecs::World;
use std::sync::Arc;
use parking_lot::RwLock;
use tokio::runtime::Runtime;
use tokio::sync::mpsc;

use crate::ecs::scenes::{Scene, SceneType, LoginScene, SelectScene, GameScene};
use crate::network::{NetworkManager, GameEvent, NetworkCommand};
use crate::settings::ClientSettings;
use mir2_shared::packets::CharacterSummary;

/// 游戏主应用
pub struct GameState {
    /// ECS World
    world: World,
    
    /// 当前场景
    current_scene: Box<dyn Scene>,
    
    /// 场景类型
    scene_type: SceneType,
    
    /// 网络管理器（可选）
    network_manager: Option<Arc<RwLock<NetworkManager>>>,
    
    /// 事件接收器（从网络线程接收）
    event_rx: mpsc::UnboundedReceiver<GameEvent>,
    
    /// 命令发送器（发送到网络线程）
    command_tx: mpsc::UnboundedSender<NetworkCommand>,
    
    /// Tokio runtime（用于网络线程）
    tokio_runtime: Option<Runtime>,
    
    /// 客户端设置
    settings: Arc<RwLock<ClientSettings>>,
    
    /// 场景间临时数据 - 角色列表（LoginScene → SelectScene）
    pending_characters: Option<Vec<CharacterSummary>>,
    
    /// 场景间临时数据 - 选中的角色索引（SelectScene → GameScene）
    selected_character_index: Option<i32>,
}

impl GameState {
    /// 创建新的游戏应用
    pub fn new(ctx: &mut Context) -> GameResult<Self> {
        println!("🎮 游戏应用初始化中...");
        
        // 加载客户端设置
        let settings = ClientSettings::load(false, None)
            .map_err(|e| {
                eprintln!("❌ 加载设置失败: {}", e);
                ggez::GameError::CustomError(format!("加载设置失败: {}", e))
            })?;
        let settings = Arc::new(RwLock::new(settings));
        
        // 创建事件和命令通道
        let (event_tx, event_rx) = mpsc::unbounded_channel::<GameEvent>();
        let (command_tx, command_rx) = mpsc::unbounded_channel::<NetworkCommand>();
        
        // 创建 Tokio runtime（用于网络线程）
        let tokio_runtime = tokio::runtime::Builder::new_multi_thread()
            .enable_all()
            .thread_name("mir2-network")
            .build()
            .map_err(|e| {
                eprintln!("❌ 创建Tokio runtime失败: {}", e);
                ggez::GameError::CustomError(format!("创建Tokio runtime失败: {}", e))
            })?;
        
        // 创建网络管理器
        let network_manager = NetworkManager::new(
            settings.clone(),
            event_tx.clone(),
            command_rx,
        );
        
        let network_manager_arc = Arc::new(parking_lot::RwLock::new(network_manager));
        
        // 注意：网络管理器在这里创建，但不立即启动连接
        // 实际使用中，可以在LoginScene中发送Connect命令来连接服务器
        
        println!("✅ 网络系统初始化完成");
        
        // 创建 ECS World
        let world = World::new();
        
        // 创建初始场景（登录场景）
        let login_scene = LoginScene::new();
        
        Ok(Self {
            world,
            current_scene: Box::new(login_scene),
            scene_type: SceneType::Login,
            network_manager: Some(network_manager_arc),
            event_rx,
            command_tx,
            tokio_runtime: Some(tokio_runtime),
            settings,
            pending_characters: None,
            selected_character_index: None,
        })
    }
    
    /// 发送网络命令
    pub fn send_command(&self, command: NetworkCommand) {
        if let Err(e) = self.command_tx.send(command) {
            eprintln!("❌ 发送网络命令失败: {}", e);
        }
    }
    
    /// 处理网络事件并分发到当前场景
    fn process_network_events(&mut self, ctx: &mut Context) -> GameResult<Option<SceneType>> {
        let mut next_scene = None;
        let mut events_to_process = Vec::new();
        
        // 收集所有待处理的网络事件
        while let Ok(event) = self.event_rx.try_recv() {
            events_to_process.push(event);
        }
        
        // 分发事件到当前场景
        for event in events_to_process {
            // 先处理全局事件（连接/断开）
            match &event {
                GameEvent::Connected => {
                    println!("✅ 已连接到服务器");
                }
                GameEvent::Disconnected { reason } => {
                    println!("⚠️ 与服务器断开连接: {}", reason);
                    // 断开连接时返回登录场景
                    if self.scene_type != SceneType::Login {
                        next_scene = Some(SceneType::Login);
                    }
                }
                _ => {}
            }
            
            // 将事件分发到当前场景
            match self.scene_type {
                SceneType::Login => {
                    // LoginScene特殊处理：登录成功需要切换场景并传递角色列表
                    if let GameEvent::LoginSuccess { ref characters } = event {
                        println!("✅ 登录成功！收到 {} 个角色", characters.len());
                        // 保存角色列表，准备切换到SelectScene
                        self.pending_characters = Some(characters.clone());
                        next_scene = Some(SceneType::Select);
                    }
                    
                    // 将事件传递给LoginScene处理
                    if let Some(login_scene) = self.current_scene.as_mut().as_any_mut().downcast_mut::<LoginScene>() {
                        login_scene.handle_network_event(&event);
                    }
                }
                SceneType::Select => {
                    // SelectScene特殊处理：开始游戏成功
                    if let GameEvent::StartGameResponse { result } = event {
                        if result == 0 {
                            println!("🎮 开始游戏成功");
                            // 获取SelectScene中选中的角色索引
                            if let Some(select_scene) = self.current_scene.as_mut().as_any_mut().downcast_mut::<SelectScene>() {
                                self.selected_character_index = Some(select_scene.selected_index);
                            }
                            next_scene = Some(SceneType::Game);
                        }
                    }
                    
                    // 将事件传递给SelectScene处理
                    if let Some(select_scene) = self.current_scene.as_mut().as_any_mut().downcast_mut::<SelectScene>() {
                        select_scene.handle_network_event(&event);
                    }
                }
                SceneType::Game => {
                    // 将事件传递给GameScene处理
                    if let Some(game_scene) = self.current_scene.as_mut().as_any_mut().downcast_mut::<GameScene>() {
                        game_scene.handle_network_event(&mut self.world, &event);
                    }
                }
            }
        }
        
        Ok(next_scene)
    }
    
    /// 切换场景
    pub fn switch_scene(&mut self, ctx: &mut Context, scene_type: SceneType) -> GameResult {
        println!("🔄 切换场景: {:?} -> {:?}", self.scene_type, scene_type);
        
        self.scene_type = scene_type;
        
        self.current_scene = match scene_type {
            SceneType::Login => Box::new(LoginScene::new()),
            SceneType::Select => {
                // 从LoginScene传递角色列表
                let characters = self.pending_characters.take().unwrap_or_else(Vec::new);
                println!("🎭 创建SelectScene，角色数: {}", characters.len());
                Box::new(SelectScene::new(characters))
            },
            SceneType::Game => Box::new(GameScene::new(ctx, &mut self.world)?),
        };
        
        Ok(())
    }
    
    /// 获取当前场景类型
    pub fn current_scene_type(&self) -> SceneType {
        self.scene_type
    }
}

impl EventHandler for GameState {
    fn update(&mut self, ctx: &mut Context) -> GameResult {
        // 处理网络事件并分发到场景（可能触发场景切换）
        if let Some(next_scene) = self.process_network_events(ctx)? {
            self.switch_scene(ctx, next_scene)?;
        }
        
        // 更新当前场景
        if let Some(next_scene) = self.current_scene.update(ctx, &mut self.world, &self.command_tx)? {
            // 场景请求切换
            self.switch_scene(ctx, next_scene)?;
        }
        
        Ok(())
    }
    
    fn draw(&mut self, ctx: &mut Context) -> GameResult {
        let mut canvas = Canvas::from_frame(ctx, Color::BLACK);
        
        // 绘制当前场景
        self.current_scene.draw(ctx, &mut canvas, &self.world)?;
        
        canvas.finish(ctx)?;
        Ok(())
    }
    
    fn mouse_button_down_event(
        &mut self,
        ctx: &mut Context,
        button: ggez::winit::event::MouseButton,
        x: f32,
        y: f32,
    ) -> GameResult {
        self.current_scene.on_mouse_down(ctx, &mut self.world, button, x, y)
    }
    
    fn mouse_button_up_event(
        &mut self,
        ctx: &mut Context,
        button: ggez::winit::event::MouseButton,
        x: f32,
        y: f32,
    ) -> GameResult {
        self.current_scene.on_mouse_up(ctx, &mut self.world, button, x, y)
    }
    
    fn mouse_motion_event(
        &mut self,
        ctx: &mut Context,
        x: f32,
        y: f32,
        _dx: f32,
        _dy: f32,
    ) -> GameResult {
        self.current_scene.on_mouse_move(ctx, &mut self.world, x, y)
    }
    
    fn key_down_event(
        &mut self,
        ctx: &mut Context,
        input: ggez::input::keyboard::KeyInput,
        _repeated: bool,
    ) -> GameResult {
        // 处理场景切换
        if let Some(new_scene_type) = self.current_scene.on_key_down(ctx, &mut self.world, input, &self.command_tx)? {
            self.switch_scene(ctx, new_scene_type)?;
        }
        Ok(())
    }
}
