// Ggez 主入口 - 替代原有的 winit + wgpu 架构
//
// 使用 ggez::EventHandler 重构主循环
// 对应 C# 原版: Client/Program.cs + Client/Forms/CMain.cs
//
// 运行: cargo run --bin mir2_client_ggez

use anyhow::Result;
use ggez::{Context, ContextBuilder, GameResult};
use ggez::event::{self, EventHandler};
use ggez::conf::{WindowMode, WindowSetup, NumSamples};
use ggez::graphics::Color;
use ggez::input::keyboard::KeyInput;
use ggez::input::mouse::MouseButton as GgezMouseButton;
use ggez::winit::keyboard::{KeyCode as WinitKeyCode, PhysicalKey, ModifiersState as WinitModifiers};
use std::sync::Arc;
use parking_lot::RwLock;

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
use crate::scenes::{SceneManager, SceneType, KeyCode as SceneKeyCode, MouseButton as SceneMouseButton, ModifiersState};

fn main() -> Result<()> {
    // 1. 初始化日志
    tracing_subscriber::fmt()
        .with_env_filter("info,mir2_client=debug")
        .init();
    
    tracing::info!("=== Crystal Mir2 Client (Ggez版本) ===");
    
    // 2. 加载配置
    let settings = ClientSettings::load(false, None)?;
    tracing::info!("配置加载完成: {:?}", settings.launcher.server_name);
    
    // 3. 设置数据路径 - 使用 Data 目录(包含 .lib 文件)
    let data_path = "Data".to_string();
    graphics::libraries::set_data_path(data_path);
    tracing::info!("数据路径设置为: Data/");
    
    // 4. 创建 ggez Context
    let res = settings.resolution();
    let window_width = res.width as f32;
    let window_height = res.height as f32;
    
    let (mut ctx, event_loop) = ContextBuilder::new("mir2_client", "Crystal")
        .window_setup(
            WindowSetup::default()
                .title(&format!("Crystal - {}", settings.launcher.server_name))
                .samples(NumSamples::Four)  // 4x MSAA
                .vsync(true)
        )
        .window_mode(
            WindowMode::default()
                .dimensions(window_width, window_height)
                .resizable(false)
        )
        .build()?;
    
    tracing::info!("Ggez Context 创建成功: {}x{}", window_width, window_height);
    
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
    
    // 5. 创建游戏状态
    let game = CrystalGame::new(&mut ctx, settings)?;
    
    // 6. 运行事件循环
    event::run(ctx, event_loop, game)
        .map_err(|e| anyhow::anyhow!("游戏循环错误: {}", e))
}

/// 游戏主状态 - 实现 ggez::EventHandler
struct CrystalGame {
    settings: ClientSettings,
    ggez_manager: GgezManager,
    scene_manager: Arc<RwLock<SceneManager>>,
    last_update_time: std::time::Instant,
}

impl CrystalGame {
    fn new(ctx: &mut Context, settings: ClientSettings) -> Result<Self> {
        println!("\n========================================");
        println!("🎮 Crystal Mir2 Client - Ggez版本");
        println!("========================================\n");
        tracing::info!("初始化游戏状态...");
        
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
        
        tracing::info!("游戏初始化完成");
        
        Ok(Self {
            settings,
            ggez_manager,
            scene_manager,
            last_update_time: std::time::Instant::now(),
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
        }
        
        // 检查退出请求 (Ctrl+Q) - 使用 key_down_event 处理
        // ggez 0.10 不提供直接查询按键状态的 API
        
        Ok(())
    }

    fn draw(&mut self, ctx: &mut Context) -> GameResult {
        // 开始帧
        self.ggez_manager.begin_frame();
        
        // 创建 canvas (黑色背景 - 场景会绘制自己的背景)
        let mut canvas = graphics::Canvas::from_frame(ctx, Color::BLACK);
        
        // 绘制当前场景
        {
            let scene_manager = self.scene_manager.read();
            scene_manager.draw(ctx, &mut canvas, &self.ggez_manager);
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
            if let Some(text) = &input.event.text {
                for ch in text.chars() {
                    // 过滤掉特殊控制字符（Tab、回车、换行等）
                    if ch != '\r' && ch != '\n' && ch != '\t' && ch != '\x08' && !ch.is_control() {
                        tracing::trace!("Text input: '{}'", ch);
                        let mut scene_manager = self.scene_manager.write();
                        scene_manager.handle_text_input(ch);
                    }
                }
            }
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
        
        let mut scene_manager = self.scene_manager.write();
        scene_manager.handle_mouse_button(scene_button, true, x as i32, y as i32);
        
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
        
        let mut scene_manager = self.scene_manager.write();
        scene_manager.handle_mouse_button(scene_button, false, x as i32, y as i32);
        
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
        let mut scene_manager = self.scene_manager.write();
        scene_manager.handle_mouse_move(x as i32, y as i32);
        
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
