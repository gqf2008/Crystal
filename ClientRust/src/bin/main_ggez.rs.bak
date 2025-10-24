// Ggez 主入口 - 替代原有的 winit + wgpu 架构
//
// 使用 ggez::EventHandler 重构主循环
// 对应 C# 原版: Client/Program.cs + Client/Forms/CMain.cs
//
// 运行: cargo run --bin mir2_client_ggez

use anyhow::Result;
use ggez::{Context, ContextBuilder};
use ggez::event::EventHandler;  // 导入 trait 以使用其方法
use ggez::conf::{WindowMode, WindowSetup, NumSamples};
use tracing::info;

// 使用 mir2_client 库中的模块
use mir2_client::program::{ClientRuntime, CrystalGame};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("\n========================================");
    println!("🎮 Crystal Mir2 Client - Ggez版本");
    println!("========================================\n");

    // 1. 初始化日志系统 (使用 program.rs 的通用方法)
    ClientRuntime::init_logging("error");
    tracing::info!("=== Crystal Mir2 Client (Ggez版本) ===");
    
    // 2. 加载配置 (使用 program.rs 的通用方法)
    let settings = ClientRuntime::load_config(false)?;
    tracing::info!("配置加载完成: {:?}", settings.launcher.server_name);
    
    // 3. 创建 Tokio runtime (使用 program.rs 的通用方法)
    let runtime = ClientRuntime::create_tokio_runtime()?;
    tracing::info!("✅ Tokio runtime 创建成功");
    
    // 4. 初始化图像库系统 (使用 program.rs 的通用方法)
    if let Err(e) = ClientRuntime::init_graphics_libraries("Data") {
        tracing::error!("图像库初始化失败: {}", e);
        tracing::warn!("将继续运行,但部分图像可能无法显示");
    }
    
    // 4. 创建 ggez Context
    let res = settings.resolution();
    // 暂时使用原始分辨率,先把基础功能修好
    let window_width = res.width as f32;   // 1024
    let window_height = res.height as f32; // 768
    
    let (mut ctx, event_loop) = ContextBuilder::new("mir2_client", "Crystal")
        .window_setup(
            WindowSetup::default()
                .title(&format!("Crystal - {}", settings.launcher.server_name))
                .samples(NumSamples::Four)  // 4x MSAA
                .vsync(true)  // 开启垂直同步，锁定 60 FPS
        )
        .window_mode(
            WindowMode::default()
                .dimensions(window_width, window_height)
                .resizable(false)
        )
        .build()?;
    
    info!(
        "Ggez Context 创建成功: {}x{} (vsync开启)",
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
    
    // 5. 创建游戏状态 (使用 program.rs 的 CrystalGame)
    let game = CrystalGame::new(settings, runtime)?;
    
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
