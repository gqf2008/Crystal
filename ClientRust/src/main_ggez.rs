// Ggez 主入口 - 替代原有的 winit + wgpu 架构
//
// 使用 ggez::EventHandler 重构主循环
// 对应 C# 原版: Client/Program.cs + Client/Forms/CMain.cs
//
// 运行: cargo run --bin mir2_client_ggez

use anyhow::Result;
use ggez::{Context, ContextBuilder, GameResult};
use ggez::event::EventHandler;
use ggez::conf::{WindowMode, WindowSetup, NumSamples};
use ggez::graphics::Color;
use ggez::input::mouse::MouseButton as GgezMouseButton;
use ggez::winit::keyboard::{KeyCode as WinitKeyCode, PhysicalKey};
use std::sync::Arc;
use parking_lot::RwLock;
use tracing::info;

mod settings;
mod graphics;
mod scenes;
mod network;
mod objects;
mod utils;
mod downloader;
mod error;
mod resolution;
mod resources;
mod version;

use settings::ClientSettings;

// Settings 已废弃,使用默认配置
// use crate::settings::Settings;
use crate::graphics::GgezManager;
use crate::scenes::{Scene, SceneManager, SceneType, LoginScene, SelectScene, KeyCode as SceneKeyCode, MouseButton as SceneMouseButton, ModifiersState};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    // 1. 创建 Tokio runtime (用于网络系统)
    let runtime = tokio::runtime::Runtime::new()?;
    
    // 2. 初始化日志
    tracing_subscriber::fmt()
        .with_env_filter("info,mir2_client=debug")
        .init();
    
    tracing::info!("=== Crystal Mir2 Client (Ggez版本) ===");
    
    // 3. 加载配置
    let settings = ClientSettings::load(false, None)?;
    tracing::info!("配置加载完成: {:?}", settings.launcher.server_name);
    
    // 3. 设置数据路径 - 使用 Data 目录(包含 .lib 文件)
    let data_path = "Data".to_string();
    graphics::libraries::set_data_path(data_path);
    tracing::info!("数据路径设置为: Data/");
    
    // 4. 创建 ggez Context
    let res = settings.resolution();
    // 暂时使用原始分辨率,先把基础功能修好
    let scale_factor = 1.0;
    let window_width = res.width as f32;   // 1024
    let window_height = res.height as f32; // 768
    
    let (mut ctx, event_loop) = ContextBuilder::new("mir2_client", "Crystal")
        .window_setup(
            WindowSetup::default()
                .title(&format!("Crystal - {}", settings.launcher.server_name))
                .samples(NumSamples::Four)  // 4x MSAA
                .vsync(false)  // 关闭垂直同步以提高帧率
        )
        .window_mode(
            WindowMode::default()
                .dimensions(window_width, window_height)
                .resizable(false)
        )
        .build()?;
    
        info!(
        "Ggez Context 创建成功: {}x{} (vsync关闭)",
        window_width, window_height
    );
    
    // 添加中文字体支持
    let font_path = std::path::Path::new("resources/font/AlibabaPuHuiTi-3-55-Regular.ttf");
    if font_path.exists() {
        match std::fs::read(font_path) {
            Ok(font_bytes) => {
                ctx.gfx.add_font(
                    "AlibabaPuHuiTi",
                    ggez::graphics::FontData::from_vec(font_bytes)?,
                );
                tracing::info!("✓ 中文字体加载成功: AlibabaPuHuiTi");
            }
            Err(e) => {
                tracing::warn!("⚠ 中文字体加载失败: {}", e);
            }
        }
    } else {
        tracing::warn!("⚠ 字体文件不存在: {:?}", font_path);
    }
   
    // 启用文本输入 (IME) - ggez 0.10 / winit 0.30
    ctx.gfx.window().set_ime_allowed(true);
    tracing::info!("IME 文本输入已启用");
    
    // 5. 创建游戏状态 (传入runtime)
    let game = CrystalGame::new(&mut ctx, settings, runtime)?;
    
    // 6. 运行自定义事件循环 (支持 IME)
    // 注意: 必须使用自定义循环,因为 ggez 0.10 不支持 WindowEvent::Ime
    run_custom_event_loop(ctx, event_loop, game)
}

/// Custom ApplicationHandler that captures IME events
struct CustomAppHandler {
    ctx: Context,
    game: CrystalGame,
}

impl ggez::winit::application::ApplicationHandler<()> for CustomAppHandler {
    fn new_events(&mut self, event_loop: &ggez::winit::event_loop::ActiveEventLoop, _: ggez::winit::event::StartCause) {
        use ggez::context::HasMut;
        use ggez::context::ContextFields;
        
        if HasMut::<ContextFields>::retrieve_mut(&mut self.ctx).quit_requested {
            HasMut::<ContextFields>::retrieve_mut(&mut self.ctx).continuing = false;
        }
        if !HasMut::<ContextFields>::retrieve_mut(&mut self.ctx).continuing {
            event_loop.exit();
            return;
        }
        
        event_loop.set_control_flow(ggez::winit::event_loop::ControlFlow::Poll);
    }
    
    fn resumed(&mut self, _event_loop: &ggez::winit::event_loop::ActiveEventLoop) {}
    
    fn window_event(
        &mut self,
        event_loop: &ggez::winit::event_loop::ActiveEventLoop,
        mut window_id: ggez::winit::window::WindowId,
        mut event: ggez::winit::event::WindowEvent,
    ) {
        use ggez::context::HasMut;
        use ggez::input::keyboard::KeyboardContext;
        use ggez::input::mouse::MouseContext;
        use ggez::winit::event::ElementState;
        use ggez::event::EventHandler;
        
        // ===== 首先检查 IME 事件 (这是获取中文的唯一方法!) =====
        if let ggez::winit::event::WindowEvent::Ime(ref ime_event) = event {
            use ggez::winit::event::Ime;
            match ime_event {
                Ime::Enabled => {
                    tracing::debug!("IME 已启用");
                }
                Ime::Disabled => {
                    tracing::debug!("IME 已禁用");
                }
                Ime::Preedit(text, _) => {
                    tracing::debug!("IME 拼音: {}", text);
                    let mut scene_mgr = self.game.scene_manager.write();
                    scene_mgr.handle_ime_preedit(text.clone());
                }
                Ime::Commit(text) => {
                    tracing::info!("✓ IME 确认中文: {}", text);
                    let mut scene_mgr = self.game.scene_manager.write();
                    scene_mgr.handle_ime_commit(text.clone());
                }
            }
            // IME 事件不转发给 ggez
            return;
        }
        
        // ===== 转发事件给 ggez 更新内部状态 =====
        ggez::event::process_window_event(&mut self.ctx, &mut window_id, &mut event);
        
        // ===== 手动调用 EventHandler 方法 =====
        match event {
            ggez::winit::event::WindowEvent::CloseRequested => {
                if let Ok(true) = self.game.quit_event(&mut self.ctx) {
                    // 用户取消退出
                } else {
                    event_loop.exit();
                }
            }
            ggez::winit::event::WindowEvent::Focused(gained) => {
                let _ = self.game.focus_event(&mut self.ctx, gained);
            }
            ggez::winit::event::WindowEvent::Resized(size) => {
                let _ = self.game.resize_event(&mut self.ctx, size.width as f32, size.height as f32);
            }
            ggez::winit::event::WindowEvent::KeyboardInput { event: key_event, .. } => {
                use ggez::input::keyboard::KeyInput;
                let mods = HasMut::<KeyboardContext>::retrieve_mut(&mut self.ctx).active_modifiers;
                let repeat = HasMut::<KeyboardContext>::retrieve_mut(&mut self.ctx).is_key_repeated();
                let input = KeyInput { event: key_event, mods };
                
                match input.event.state {
                    ElementState::Pressed => {
                        let _ = self.game.key_down_event(&mut self.ctx, input, repeat);
                    }
                    ElementState::Released => {
                        let _ = self.game.key_up_event(&mut self.ctx, input);
                    }
                }
            }
            ggez::winit::event::WindowEvent::MouseInput { state, button, .. } => {
                let position = HasMut::<MouseContext>::retrieve_mut(&mut self.ctx).position();
                match state {
                    ElementState::Pressed => {
                        let _ = self.game.mouse_button_down_event(&mut self.ctx, button, position.x, position.y);
                    }
                    ElementState::Released => {
                        let _ = self.game.mouse_button_up_event(&mut self.ctx, button, position.x, position.y);
                    }
                }
            }
            ggez::winit::event::WindowEvent::CursorMoved { .. } => {
                let position = HasMut::<MouseContext>::retrieve_mut(&mut self.ctx).position();
                let delta = HasMut::<MouseContext>::retrieve_mut(&mut self.ctx).last_delta();
                let _ = self.game.mouse_motion_event(&mut self.ctx, position.x, position.y, delta.x, delta.y);
            }
            ggez::winit::event::WindowEvent::MouseWheel { delta, .. } => {
                let (x, y) = match delta {
                    ggez::winit::event::MouseScrollDelta::LineDelta(x, y) => (x, y),
                    ggez::winit::event::MouseScrollDelta::PixelDelta(pos) => {
                        // 简单处理:直接使用像素值
                        (pos.x as f32, pos.y as f32)
                    }
                };
                let _ = self.game.mouse_wheel_event(&mut self.ctx, x, y);
            }
            _ => {}
        }
    }
    
    fn device_event(
        &mut self,
        _event_loop: &ggez::winit::event_loop::ActiveEventLoop,
        mut device_id: ggez::winit::event::DeviceId,
        mut event: ggez::winit::event::DeviceEvent,
    ) {
        // 转发设备事件给 ggez
        ggez::event::process_device_event(&mut self.ctx, &mut device_id, &mut event);
    }
    
    fn about_to_wait(&mut self, event_loop: &ggez::winit::event_loop::ActiveEventLoop) {
        use ggez::context::HasMut;
        use ggez::graphics::GraphicsContext;
        
        // 更新定时器
        HasMut::<ggez::timer::TimeContext>::retrieve_mut(&mut self.ctx).tick();
        
        // 更新游戏逻辑
        if let Err(e) = self.game.update(&mut self.ctx) {
            tracing::error!("Update error: {}", e);
            event_loop.exit();
            return;
        }
        
        // 开始绘制帧
        if let Err(e) = HasMut::<GraphicsContext>::retrieve_mut(&mut self.ctx).begin_frame() {
            tracing::error!("Begin frame error: {}", e);
            event_loop.exit();
            return;
        }
        
        // 绘制游戏画面
        if let Err(e) = self.game.draw(&mut self.ctx) {
            tracing::error!("Draw error: {}", e);
            event_loop.exit();
            return;
        }
        
        // 结束绘制帧
        if let Err(e) = HasMut::<GraphicsContext>::retrieve_mut(&mut self.ctx).end_frame() {
            tracing::error!("End frame error: {}", e);
            event_loop.exit();
            return;
        }
        
        // 保存输入状态
        use ggez::input::keyboard::KeyboardContext;
        use ggez::input::mouse::MouseContext;
        
        HasMut::<MouseContext>::retrieve_mut(&mut self.ctx).reset_delta();
        HasMut::<KeyboardContext>::retrieve_mut(&mut self.ctx).save_keyboard_state();
        HasMut::<MouseContext>::retrieve_mut(&mut self.ctx).save_mouse_state();
    }
}

/// 自定义事件循环 - 完整支持 IME
fn run_custom_event_loop(
    ctx: Context,
    event_loop: ggez::winit::event_loop::EventLoop<()>,
    game: CrystalGame,
) -> Result<(), Box<dyn std::error::Error>> {
    let mut app = CustomAppHandler { ctx, game };
    
    event_loop
        .run_app(&mut app)
        .map_err(|e| Box::new(e) as Box<dyn std::error::Error>)
}

/// 游戏主状态 - 实现 ggez::EventHandler
struct CrystalGame {
    settings: ClientSettings,
    ggez_manager: GgezManager,
    scene_manager: Arc<RwLock<SceneManager>>,
    last_update_time: std::time::Instant,
    scale_factor: f32,  // 窗口缩放因子
    
    // 网络系统
    game_client: crate::network::game_client::SharedGameClient,
    event_rx: tokio::sync::mpsc::UnboundedReceiver<crate::network::game_client::GameEvent>,
    command_tx: tokio::sync::mpsc::UnboundedSender<crate::network::NetworkCommand>,
    network_task: Option<tokio::task::JoinHandle<()>>,
    
    // 缓存的 MapInformation 事件(用于场景切换时不丢失)
    cached_map_info: Option<(i32, String, String)>, // (map_index, file_name, title)
    
    // Tokio runtime (保持网络任务运行)
    #[allow(dead_code)]
    runtime: tokio::runtime::Runtime,
}

impl CrystalGame {
    fn new(_ctx: &mut Context, settings: ClientSettings, runtime: tokio::runtime::Runtime) -> Result<Self> {
        // 确保runtime可用
        let _guard = runtime.enter();
        println!("\n========================================");
        println!("🎮 Crystal Mir2 Client - Ggez版本");
        println!("========================================\n");
        tracing::info!("初始化游戏状态...");
        
        // 创建网络系统
        println!("🌐 初始化网络系统...");
        let game_client = crate::network::game_client::new_shared_client();
        let (event_tx, event_rx) = tokio::sync::mpsc::unbounded_channel();
        let (command_tx, command_rx) = tokio::sync::mpsc::unbounded_channel();
        
        // 启动网络任务 (Tokio runtime)
        println!("🚀 启动网络管理器...");
        let settings_arc = Arc::new(RwLock::new(settings.clone()));
        let gc = game_client.clone();
        
        let network_task = Some(tokio::spawn(async move {
            // 设置事件通道 (tokio::sync::RwLock 是异步的,需要await)
            gc.write().await.set_event_channel(event_tx.clone());
            
            use crate::network::NetworkManager;
            let mut network_manager = NetworkManager::new(settings_arc, event_tx, command_rx);
            
            // 自动连接到服务器
            tracing::info!("🔌 正在连接服务器...");
            if let Err(e) = network_manager.connect().await {
                tracing::error!("❌ 连接服务器失败: {}", e);
            } else {
                tracing::info!("✅ 已连接到服务器");
            }
            
            // 进入消息循环 (不返回Result)
            crate::network::network_task(network_manager).await;
        }));
        
        println!("✓ 网络系统初始化完成");
        
        // 创建渲染管理器
        let res = settings.resolution();
        let screen_width = res.width as f32;
        let screen_height = res.height as f32;
        let ggez_manager = GgezManager::new(screen_width, screen_height);
        println!("✓ Ggez 渲染管理器已创建: {}x{}", screen_width, screen_height);
        
        // 加载核心图形库 (Data.lib, Prguse.lib 等)
        println!("📦 正在加载图形库...");
        tracing::info!("加载图形库...");
        if let Err(e) = graphics::libraries::load_core_libraries() {
            println!("⚠ 加载图形库失败: {}", e);
            tracing::warn!("加载图形库失败: {}, 将在需要时按需加载", e);
        } else {
            println!("✓ 所有图形库加载成功!");
        }
        
        // 创建场景管理器
        let mut scene_manager = SceneManager::new();
        
        // 根据启动模式选择初始场景
        let initial_scene = if settings.launcher.enabled {
            tracing::info!("启动器模式: 显示 LauncherWindow");
            // TODO: 创建 LauncherScene
            SceneType::Login  // 暂时使用 LoginScene
        } else {
            tracing::info!("直接登录模式: 显示 LoginScene");
            SceneType::Login
        };
        
        scene_manager.switch_scene(initial_scene)?;
        
        let scene_manager = Arc::new(RwLock::new(scene_manager));
        
        // 将 GameClient 和 CommandTx 传递给场景
        {
            let mut mgr = scene_manager.write();
            mgr.set_game_client(game_client.clone());
            mgr.set_command_sender(command_tx.clone());
        }
        
        tracing::info!("游戏初始化完成");
        println!("\n✅ 所有系统启动完毕!\n");
        
        Ok(Self {
            settings,
            ggez_manager,
            scene_manager,
            last_update_time: std::time::Instant::now(),
            scale_factor: 1.0,  // 暂时不缩放
            game_client,
            event_rx,
            command_tx,
            network_task,
            cached_map_info: None,  // 初始化为 None
            runtime,
        })
    }
}

impl EventHandler for CrystalGame {
    fn update(&mut self, ctx: &mut Context) -> GameResult {
        // 计算 delta_time
        let now = std::time::Instant::now();
        let delta_time = now.duration_since(self.last_update_time).as_secs_f32();
        self.last_update_time = now;
        
        // 限制 delta_time (防止卡顿时跳跃过大)
        let delta_time = delta_time.min(0.1);
        
        // 处理网络事件
        while let Ok(event) = self.event_rx.try_recv() {
            tracing::debug!("收到网络事件: {:?}", event);
            
            // 特殊处理: 缓存 MapInformation 事件,防止场景切换时丢失
            if let crate::network::game_client::GameEvent::MapInformation { map_index, ref file_name, ref title } = event {
                tracing::info!("💾 Caching MapInformation: {} ({})", title, file_name);
                self.cached_map_info = Some((map_index, file_name.clone(), title.clone()));
            }
            
            let mut scene_manager = self.scene_manager.write();
            scene_manager.process_event(&event);
        }
        
        // 检查 LoginScene 是否需要切换到 SelectScene
        let should_switch_to_select = {
            let scene_manager = self.scene_manager.read();
            if scene_manager.current_scene_type() == Some(SceneType::Login) {
                if let Some(scene) = scene_manager.current_scene() {
                    if let Some(login_scene) = scene.as_any().downcast_ref::<LoginScene>() {
                        login_scene.ready_for_character_select
                    } else {
                        false
                    }
                } else {
                    false
                }
            } else {
                false
            }
        };
        
        if should_switch_to_select {
            // 获取角色列表
            let characters = {
                let scene_manager = self.scene_manager.read();
                if let Some(scene) = scene_manager.current_scene() {
                    if let Some(login_scene) = scene.as_any().downcast_ref::<LoginScene>() {
                        login_scene.characters.clone()
                    } else {
                        Vec::new()
                    }
                } else {
                    Vec::new()
                }
            };
            
            tracing::info!("✅ 登录成功,切换到角色选择场景 ({} 个角色)", characters.len());
            
            // 🎨 预加载 SelectScene 所需的纹理 (不持有任何锁)
            tracing::info!("📦 预加载 SelectScene 纹理...");
            self.load_select_scene_textures(ctx);
            
            // 创建 SelectScene
            let mut select_scene = Box::new(SelectScene::new(characters));
            
            // 设置网络命令发送器
            select_scene.set_command_sender(self.command_tx.clone());
            
            // 初始化并替换当前场景
            select_scene.initialize();
            
            let mut scene_manager = self.scene_manager.write();
            scene_manager.set_current_scene(select_scene);
            tracing::info!("场景切换完成: Select");
        }
        
        // 检查 SelectScene 是否需要切换到 GameScene
        let should_switch_to_game = {
            let scene_manager = self.scene_manager.read();
            if scene_manager.current_scene_type() == Some(SceneType::Select) {
                if let Some(scene) = scene_manager.current_scene() {
                    if let Some(select_scene) = scene.as_any().downcast_ref::<SelectScene>() {
                        select_scene.pending_scene_change == Some(SceneType::Game)
                    } else {
                        false
                    }
                } else {
                    false
                }
            } else {
                false
            }
        };
        
        if should_switch_to_game {
            tracing::info!("✅ 开始游戏,切换到 GameScene");
            
            // 切换到 GameScene
            let mut scene_manager = self.scene_manager.write();
            if let Err(e) = scene_manager.switch_scene(SceneType::Game) {
                tracing::error!("❌ 切换到 GameScene 失败: {}", e);
            } else {
                tracing::info!("🎮 场景切换完成: Game");
                
                // 如果有缓存的地图信息,立即发送到 GameScene
                if let Some((map_index, file_name, title)) = &self.cached_map_info {
                    tracing::info!("🔄 Resending cached MapInformation to GameScene: {} ({})", title, file_name);
                    
                    // 重新创建 MapInformation 事件并发送到 GameScene
                    let event = crate::network::game_client::GameEvent::MapInformation {
                        map_index: *map_index,
                        file_name: file_name.clone(),
                        title: title.clone(),
                    };
                    
                    drop(scene_manager); // 释放写锁
                    let mut scene_manager = self.scene_manager.write();
                    scene_manager.process_event(&event);
                } else {
                    tracing::warn!("⚠️  No cached map information available");
                }
            }
        }
        
        // 处理场景切换
        {
            let mut scene_manager = self.scene_manager.write();
            if let Err(e) = scene_manager.process_transitions() {
                tracing::error!("场景切换错误: {}", e);
                ctx.request_quit();
                return Ok(());
            }
            
            // 更新当前场景
            scene_manager.update(delta_time);
            
            // TODO: 检查场景是否请求退出
            // if let Some(scene) = scene_manager.current_scene() {
            //     if let Some(select_scene) = scene.as_any().downcast_ref::<SelectScene>() {
            //         if select_scene.should_exit {
            //             tracing::info!("SelectScene 请求退出游戏");
            //             ctx.request_quit();
            //             return Ok(());
            //         }
            //     }
            // }
        }
        
        // 检查退出请求 (Ctrl+Q) - 使用 key_down_event 处理
        // ggez 0.10 不提供直接查询按键状态的 API
        
        Ok(())
    }

    fn draw(&mut self, ctx: &mut Context) -> GameResult {
        // 开始帧
        self.ggez_manager.begin_frame();
        
        // 根据当前场景选择背景色
        let bg_color = {
            let scene_manager = self.scene_manager.read();
            match scene_manager.current_scene_type() {
                Some(SceneType::Game) => Color::from_rgb(20, 30, 40), // GameScene: 深蓝灰色
                _ => Color::BLACK, // 其他场景: 黑色
            }
        };
        
        // 创建 canvas
        let mut canvas = graphics::Canvas::from_frame(ctx, bg_color);        // 绘制当前场景
        {
            let scene_manager = self.scene_manager.read();
            scene_manager.draw(ctx, &mut canvas, &mut self.ggez_manager);
        }
        
        // 结束帧
        canvas.finish(ctx)?;
        self.ggez_manager.end_frame();
        
        Ok(())
    }
    
    fn key_down_event(
        &mut self,
        ctx: &mut Context,
        input: ggez::input::keyboard::KeyInput,
        _repeated: bool,
    ) -> GameResult {
        tracing::info!("🔑 key_down_event 被调用!");
        
        // ggez 0.10: KeyInput 结构变为 winit 事件
        let mut key_consumed = false;
        
        if let PhysicalKey::Code(keycode) = input.event.physical_key {
            // 检查 Ctrl+Q 退出
            if keycode == WinitKeyCode::KeyQ && input.mods.control_key() {
                tracing::info!("用户请求退出 (Ctrl+Q)");
                ctx.request_quit();
                return Ok(());
            }
            
            let modifiers = input.mods;
            
            // 转换为 Scene 自定义 ModifiersState
            let modifiers_state = ModifiersState {
                shift: modifiers.shift_key(),
                ctrl: modifiers.control_key(),
                alt: modifiers.alt_key(),
            };
            
            // 转换 ggez KeyCode 到 Scene KeyCode
            if let Some(scene_keycode) = ggez_keycode_to_scene(keycode) {
                let mut scene_manager = self.scene_manager.write();
                key_consumed = scene_manager.handle_key_press(scene_keycode, modifiers_state);
            }
        }
        
        // 只有在按键未被消费时才处理文本输入
        // 这样可以防止空格、M等功能键的文本被输入到文本框
        if !key_consumed {
            // 调试: 检查 text 字段
            tracing::debug!("KeyEvent - text field: {:?}", input.event.text);
            
            if let Some(text) = &input.event.text {
                tracing::info!("✓ 收到文本: '{}'", text);
                for ch in text.chars() {
                    // 过滤掉特殊控制字符（Tab、回车、换行等）
                    if ch != '\r' && ch != '\n' && ch != '\t' && ch != '\x08' && !ch.is_control() {
                        tracing::info!("→ 处理字符: '{}'", ch);
                        let mut scene_manager = self.scene_manager.write();
                        scene_manager.handle_text_input(ch);
                    } else {
                        tracing::debug!("✗ 过滤控制字符: {:?}", ch);
                    }
                }
            } else {
                tracing::debug!("✗ text 字段为 None");
            }
        } else {
            tracing::debug!("✗ 按键被消费,跳过文本输入");
        }
        
        Ok(())
    }
    
    fn mouse_button_down_event(
        &mut self,
        _ctx: &mut Context,
        button: GgezMouseButton,
        x: f32,
        y: f32,
    ) -> GameResult {
        // 转换 ggez MouseButton 到 Scene MouseButton
        let scene_button = match button {
            GgezMouseButton::Left => SceneMouseButton::Left,
            GgezMouseButton::Right => SceneMouseButton::Right,
            GgezMouseButton::Middle => SceneMouseButton::Middle,
            GgezMouseButton::Back | GgezMouseButton::Forward => SceneMouseButton::Other(0),
            GgezMouseButton::Other(v) => SceneMouseButton::Other(v),
        };
        
        // 将实际窗口坐标转换为逻辑坐标
        let logical_x = x / self.scale_factor;
        let logical_y = y / self.scale_factor;
        
        let mut scene_manager = self.scene_manager.write();
        scene_manager.handle_mouse_button(scene_button, true, logical_x as i32, logical_y as i32);
        
        Ok(())
    }
    
    fn mouse_button_up_event(
        &mut self,
        _ctx: &mut Context,
        button: GgezMouseButton,
        x: f32,
        y: f32,
    ) -> GameResult {
        let scene_button = match button {
            GgezMouseButton::Left => SceneMouseButton::Left,
            GgezMouseButton::Right => SceneMouseButton::Right,
            GgezMouseButton::Middle => SceneMouseButton::Middle,
            GgezMouseButton::Back | GgezMouseButton::Forward => SceneMouseButton::Other(0),
            GgezMouseButton::Other(v) => SceneMouseButton::Other(v),
        };
        
        // 将实际窗口坐标转换为逻辑坐标
        let logical_x = x / self.scale_factor;
        let logical_y = y / self.scale_factor;
        
        let mut scene_manager = self.scene_manager.write();
        scene_manager.handle_mouse_button(scene_button, false, logical_x as i32, logical_y as i32);
        
        Ok(())
    }
    
    fn mouse_motion_event(
        &mut self,
        _ctx: &mut Context,
        x: f32,
        y: f32,
        _dx: f32,
        _dy: f32,
    ) -> GameResult {
        // 将实际窗口坐标转换为逻辑坐标
        let logical_x = x / self.scale_factor;
        let logical_y = y / self.scale_factor;
        
        let mut scene_manager = self.scene_manager.write();
        scene_manager.handle_mouse_move(logical_x as i32, logical_y as i32);
        
        Ok(())
    }
    
    /// 处理文本输入事件 (包括 IME 中文输入)
    /// ggez 会自动处理 IME，并将最终确认的字符通过此方法传递
    fn text_input_event(&mut self, _ctx: &mut Context, character: char) -> GameResult {
        tracing::debug!("收到文本输入: '{}'", character);
        
        // 将字符传递给场景管理器
        let mut scene_manager = self.scene_manager.write();
        scene_manager.handle_text_input(character);
        
        Ok(())
    }
}

/// 转换 winit KeyCode 到 Scene KeyCode
fn ggez_keycode_to_scene(key: ggez::winit::keyboard::KeyCode) -> Option<SceneKeyCode> {
    use ggez::winit::keyboard::KeyCode as GK;
    
    Some(match key {
        GK::KeyA => SceneKeyCode::KeyA,
        GK::KeyB => SceneKeyCode::KeyB,
        GK::KeyC => SceneKeyCode::KeyC,
        GK::KeyD => SceneKeyCode::KeyD,
        GK::KeyE => SceneKeyCode::KeyE,
        GK::KeyF => SceneKeyCode::KeyF,
        GK::KeyG => SceneKeyCode::KeyG,
        GK::KeyH => SceneKeyCode::KeyH,
        GK::KeyI => SceneKeyCode::KeyI,
        GK::KeyJ => SceneKeyCode::KeyJ,
        GK::KeyK => SceneKeyCode::KeyK,
        GK::KeyL => SceneKeyCode::KeyL,
        GK::KeyM => SceneKeyCode::KeyM,
        GK::KeyN => SceneKeyCode::KeyN,
        GK::KeyO => SceneKeyCode::KeyO,
        GK::KeyP => SceneKeyCode::KeyP,
        GK::KeyQ => SceneKeyCode::KeyQ,
        GK::KeyR => SceneKeyCode::KeyR,
        GK::KeyS => SceneKeyCode::KeyS,
        GK::KeyT => SceneKeyCode::KeyT,
        GK::KeyU => SceneKeyCode::KeyU,
        GK::KeyV => SceneKeyCode::KeyV,
        GK::KeyW => SceneKeyCode::KeyW,
        GK::KeyX => SceneKeyCode::KeyX,
        GK::KeyY => SceneKeyCode::KeyY,
        GK::KeyZ => SceneKeyCode::KeyZ,
        GK::Digit1 => SceneKeyCode::Digit1,
        GK::Digit2 => SceneKeyCode::Digit2,
        GK::Digit3 => SceneKeyCode::Digit3,
        GK::Digit4 => SceneKeyCode::Digit4,
        GK::Digit5 => SceneKeyCode::Digit5,
        GK::Digit6 => SceneKeyCode::Digit6,
        GK::Digit7 => SceneKeyCode::Digit7,
        GK::Digit8 => SceneKeyCode::Digit8,
        GK::Digit9 => SceneKeyCode::Digit9,
        GK::Digit0 => SceneKeyCode::Digit0,
        GK::Enter => SceneKeyCode::Enter,
        GK::Escape => SceneKeyCode::Escape,
        GK::Backspace => SceneKeyCode::Backspace,
        GK::Tab => SceneKeyCode::Tab,
        GK::Space => SceneKeyCode::Space,
        GK::ArrowLeft => SceneKeyCode::ArrowLeft,
        GK::ArrowRight => SceneKeyCode::ArrowRight,
        GK::ArrowUp => SceneKeyCode::ArrowUp,
        GK::ArrowDown => SceneKeyCode::ArrowDown,
        GK::F1 => SceneKeyCode::F1,
        GK::F2 => SceneKeyCode::F2,
        GK::F3 => SceneKeyCode::F3,
        GK::F4 => SceneKeyCode::F4,
        GK::F5 => SceneKeyCode::F5,
        GK::F6 => SceneKeyCode::F6,
        GK::F7 => SceneKeyCode::F7,
        GK::F8 => SceneKeyCode::F8,
        GK::F9 => SceneKeyCode::F9,
        GK::F10 => SceneKeyCode::F10,
        GK::F11 => SceneKeyCode::F11,
        GK::F12 => SceneKeyCode::F12,
        _ => return None,
    })
}

/// CrystalGame 辅助方法
impl CrystalGame {
    /// 预加载 SelectScene 所需的纹理
    fn load_select_scene_textures(&mut self, ctx: &mut Context) {
        use crate::graphics::libraries::{get_library, LibraryName};
        
        // SelectScene 需要的纹理列表
        let mut textures_to_load = vec![
            (LibraryName::Prguse, 65),   // 背景
            (LibraryName::Title, 40),     // 标题
            (LibraryName::Prguse, 44),   // 空角色槽位
        ];
        
        // 添加按钮纹理 Title_340-354 (StartGame, NewCharacter, DeleteCharacter, Credits, ExitGame)
        for i in 340..=354 {
            textures_to_load.push((LibraryName::Title, i));
        }
        
        // 添加角色槽位纹理 Title_660-669 (职业槽位: 660-664未选中, 665-669选中)
        for i in 660..=669 {
            textures_to_load.push((LibraryName::Title, i));
        }
        
        // 添加角色预览纹理 ChrSel_220 和混合纹理 ChrSel_780 (220+560)
        textures_to_load.push((LibraryName::ChrSel, 220));
        textures_to_load.push((LibraryName::ChrSel, 780));  // 220 + 560 混合效果
        
        // NewCharacterDialog 纹理
        textures_to_load.push((LibraryName::Prguse, 73));   // 对话框背景
        textures_to_load.push((LibraryName::Title, 20));    // 标题标签
        textures_to_load.push((LibraryName::Title, 280));   // 取消按钮 (Normal)
        textures_to_load.push((LibraryName::Title, 281));   // 取消按钮 (Hover)
        textures_to_load.push((LibraryName::Title, 282));   // 取消按钮 (Pressed)
        textures_to_load.push((LibraryName::Title, 360));   // 确认按钮 (Normal)
        textures_to_load.push((LibraryName::Title, 361));   // 确认按钮 (Hover)
        textures_to_load.push((LibraryName::Title, 362));   // 确认按钮 (Pressed)
        
        // DeleteCharacterDialog / MessageBox / InputBox 纹理
        textures_to_load.push((LibraryName::Prguse, 360));  // MessageBox 背景
        textures_to_load.push((LibraryName::Prguse, 660));  // InputBox 背景
        // OK 按钮 (200-202)
        textures_to_load.push((LibraryName::Title, 200));   // OK (Normal)
        textures_to_load.push((LibraryName::Title, 201));   // OK (Hover)
        textures_to_load.push((LibraryName::Title, 202));   // OK (Pressed)
        // Cancel 按钮 (203-205)
        textures_to_load.push((LibraryName::Title, 203));   // Cancel (Normal)
        textures_to_load.push((LibraryName::Title, 204));   // Cancel (Hover)
        textures_to_load.push((LibraryName::Title, 205));   // Cancel (Pressed)
        // Yes 按钮 (206-208)
        textures_to_load.push((LibraryName::Title, 206));   // Yes (Normal)
        textures_to_load.push((LibraryName::Title, 207));   // Yes (Hover)
        textures_to_load.push((LibraryName::Title, 208));   // Yes (Pressed)
        // No 按钮 (210-212)
        textures_to_load.push((LibraryName::Title, 210));   // No (Normal)
        textures_to_load.push((LibraryName::Title, 211));   // No (Hover)
        textures_to_load.push((LibraryName::Title, 212));   // No (Pressed)
        
        // 职业按钮 (战士、法师、道士、刺客、弓箭手)
        for i in 2426..=2440 {  // 2426-2428 (战士), 2429-2431 (法师), 2432-2434 (道士), 2435-2437 (刺客), 2438-2440 (弓箭手)
            textures_to_load.push((LibraryName::Prguse, i));
        }
        
        // 性别按钮 (男、女)
        for i in 2420..=2425 {  // 2420-2422 (男), 2423-2425 (女)
            textures_to_load.push((LibraryName::Prguse, i));
        }
        
        // 角色预览动画 (所有职业和性别组合)
        // 战士男 20-35, 法师男 40-55, 道士男 60-75, 刺客男 80-95, 弓箭手男 100-115
        // 战士女 300-315, 法师女 320-335, 道士女 340-355, 刺客女 360-375, 弓箭手女 140-155 (注意:不是380!)
        for base in [20, 40, 60, 80, 100, 300, 320, 340, 360, 140] {
            for i in base..base+16 {  // 16帧动画
                textures_to_load.push((LibraryName::ChrSel, i));
            }
        }
        
        // 法师混合效果纹理 (基础索引 + 560)
        // 法师男: 40-55 + 560 = 600-615
        // 法师女: 320-335 + 560 = 880-895
        for base in [40, 320] {
            for i in (base+560)..(base+560+16) {
                textures_to_load.push((LibraryName::ChrSel, i));
            }
        }
        
        for (lib_name, index) in textures_to_load {
            let key = format!("{}_{}", lib_name.default_path(), index);
            
            // 检查是否已缓存
            if self.ggez_manager.get_texture(&key).is_some() {
                continue;
            }
            
            // 从 MLibrary 加载
            if let Some(lib_arc) = get_library(lib_name) {
                let mut lib = lib_arc.lock().unwrap();
                if let Ok((info, pixels)) = lib.load_rgba_data(index) {
                    let width = info.width as u16;  // i16 -> u16
                    let height = info.height as u16;  // i16 -> u16
                    drop(lib);  // 释放锁
                    if let Err(e) = self.ggez_manager.create_texture_from_rgba(
                        ctx,
                        width,
                        height,
                        pixels.as_slice(),
                        key.clone(),
                    ) {
                        tracing::warn!("⚠️ 创建纹理失败 {}: {}", key, e);
                    } else {
                        tracing::debug!("✓ 纹理已加载: {}", key);
                    }
                } else {
                    tracing::warn!("⚠️ 无法从库 {} 获取图像 {}", lib_name.default_path(), index);
                }
            } else {
                tracing::warn!("⚠️ 库未加载: {:?}", lib_name);
            }
        }
        
        tracing::info!("✓ SelectScene 纹理预加载完成");
    }
}
