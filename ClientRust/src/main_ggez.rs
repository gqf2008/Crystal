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
use ggez::graphics::{self, Color};
use std::sync::Arc;
use parking_lot::RwLock;

mod settings;
mod graphics;
mod scenes;
mod network;
mod utils;
mod downloader;

use crate::settings::Settings;
use crate::graphics::{GgezManager, libraries};
use crate::scenes::{SceneManager, SceneType, KeyCode as SceneKeyCode, MouseButton as SceneMouseButton, ModifiersState};

fn main() -> Result<()> {
    // 1. 初始化日志
    tracing_subscriber::fmt()
        .with_env_filter("info,mir2_client=debug")
        .init();
    
    tracing::info!("=== Crystal Mir2 Client (Ggez版本) ===");
    
    // 2. 加载配置
    let settings = Settings::load()?;
    tracing::info!("配置加载完成: {:?}", settings.game.server_name);
    
    // 3. 设置数据路径
    graphics::libraries::set_data_path(settings.paths.data_path.clone());
    
    // 4. 创建 ggez Context
    let window_width = settings.game.resolution.0 as f32;
    let window_height = settings.game.resolution.1 as f32;
    
    let (mut ctx, event_loop) = ContextBuilder::new("mir2_client", "Crystal")
        .window_setup(
            WindowSetup::default()
                .title(&format!("Crystal - {}", settings.game.server_name))
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
    
    // 5. 创建游戏状态
    let game = CrystalGame::new(&mut ctx, settings)?;
    
    // 6. 运行事件循环
    event::run(ctx, event_loop, game)
        .map_err(|e| anyhow::anyhow!("游戏循环错误: {}", e))
}

/// 游戏主状态 - 实现 ggez::EventHandler
struct CrystalGame {
    settings: Settings,
    ggez_manager: GgezManager,
    scene_manager: Arc<RwLock<SceneManager>>,
    last_update_time: std::time::Instant,
}

impl CrystalGame {
    fn new(ctx: &mut Context, settings: Settings) -> Result<Self> {
        tracing::info!("初始化游戏状态...");
        
        // 创建渲染管理器
        let screen_width = settings.game.resolution.0 as f32;
        let screen_height = settings.game.resolution.1 as f32;
        let ggez_manager = GgezManager::new(screen_width, screen_height);
        
        // 加载核心图形库 (Data.lib, Prguse.lib 等)
        tracing::info!("加载图形库...");
        if let Err(e) = graphics::libraries::load_core_libraries() {
            tracing::warn!("加载图形库失败: {}, 将在需要时按需加载", e);
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
        
        // 检查退出请求 (Ctrl+Q)
        if ctx.keyboard.is_key_pressed(ggez::input::keyboard::KeyCode::KeyQ) {
            let mods = ctx.keyboard.active_mods();
            if mods.contains(ggez::input::keyboard::KeyMods::CTRL) {
                tracing::info!("用户请求退出 (Ctrl+Q)");
                ctx.request_quit();
            }
        }
        
        Ok(())
    }

    fn draw(&mut self, ctx: &mut Context) -> GameResult {
        // 开始帧
        self.ggez_manager.begin_frame(ctx, Color::from_rgb(20, 30, 60))?;
        
        // 创建 canvas
        let mut canvas = graphics::Canvas::from_frame(ctx, Color::from_rgb(20, 30, 60));
        
        // 绘制当前场景
        {
            let scene_manager = self.scene_manager.read();
            scene_manager.draw(&mut canvas, &self.ggez_manager);
        }
        
        // 结束帧
        canvas.finish(ctx)?;
        self.ggez_manager.end_frame();
        
        Ok(())
    }
    
    fn key_down_event(
        &mut self,
        _ctx: &mut Context,
        input: ggez::input::keyboard::KeyInput,
        _repeated: bool,
    ) -> GameResult {
        if let Some(keycode) = input.keycode {
            let modifiers = ggez::input::keyboard::KeyMods::from_bits_truncate(
                input.mods.bits()
            );
            
            // 转换为 Scene 自定义 ModifiersState
            let modifiers_state = ModifiersState {
                shift: modifiers.contains(ggez::input::keyboard::KeyMods::SHIFT),
                ctrl: modifiers.contains(ggez::input::keyboard::KeyMods::CTRL),
                alt: modifiers.contains(ggez::input::keyboard::KeyMods::ALT),
            };
            
            // 转换 ggez KeyCode 到 Scene KeyCode
            if let Some(scene_keycode) = ggez_keycode_to_scene(keycode) {
                let mut scene_manager = self.scene_manager.write();
                scene_manager.handle_key_press(scene_keycode, modifiers_state);
            }
        }
        
        Ok(())
    }
    
    fn mouse_button_down_event(
        &mut self,
        _ctx: &mut Context,
        button: ggez::event::MouseButton,
        x: f32,
        y: f32,
    ) -> GameResult {
        // 转换 ggez MouseButton 到 winit MouseButton
        let winit_button = match button {
            ggez::event::MouseButton::Left => winit::event::MouseButton::Left,
            ggez::event::MouseButton::Right => winit::event::MouseButton::Right,
            ggez::event::MouseButton::Middle => winit::event::MouseButton::Middle,
            _ => return Ok(()),
        };
        
        let mut scene_manager = self.scene_manager.write();
        scene_manager.handle_mouse_button(winit_button, true, x as i32, y as i32);
        
        Ok(())
    }
    
    fn mouse_button_up_event(
        &mut self,
        _ctx: &mut Context,
        button: ggez::event::MouseButton,
        x: f32,
        y: f32,
    ) -> GameResult {
        let winit_button = match button {
            ggez::event::MouseButton::Left => winit::event::MouseButton::Left,
            ggez::event::MouseButton::Right => winit::event::MouseButton::Right,
            ggez::event::MouseButton::Middle => winit::event::MouseButton::Middle,
            _ => return Ok(()),
        };
        
        let mut scene_manager = self.scene_manager.write();
        scene_manager.handle_mouse_button(winit_button, false, x as i32, y as i32);
        
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

/// 转换 ggez KeyCode 到 winit KeyCode
fn ggez_keycode_to_winit(key: ggez::input::keyboard::KeyCode) -> Option<winit::keyboard::KeyCode> {
    use ggez::input::keyboard::KeyCode as GK;
    use winit::keyboard::KeyCode as WK;
    
    Some(match key {
        GK::KeyA => WK::KeyA,
        GK::KeyB => WK::KeyB,
        GK::KeyC => WK::KeyC,
        GK::KeyD => WK::KeyD,
        GK::KeyE => WK::KeyE,
        GK::KeyF => WK::KeyF,
        GK::KeyG => WK::KeyG,
        GK::KeyH => WK::KeyH,
        GK::KeyI => WK::KeyI,
        GK::KeyJ => WK::KeyJ,
        GK::KeyK => WK::KeyK,
        GK::KeyL => WK::KeyL,
        GK::KeyM => WK::KeyM,
        GK::KeyN => WK::KeyN,
        GK::KeyO => WK::KeyO,
        GK::KeyP => WK::KeyP,
        GK::KeyQ => WK::KeyQ,
        GK::KeyR => WK::KeyR,
        GK::KeyS => WK::KeyS,
        GK::KeyT => WK::KeyT,
        GK::KeyU => WK::KeyU,
        GK::KeyV => WK::KeyV,
        GK::KeyW => WK::KeyW,
        GK::KeyX => WK::KeyX,
        GK::KeyY => WK::KeyY,
        GK::KeyZ => WK::KeyZ,
        GK::Key1 => WK::Digit1,
        GK::Key2 => WK::Digit2,
        GK::Key3 => WK::Digit3,
        GK::Key4 => WK::Digit4,
        GK::Key5 => WK::Digit5,
        GK::Key6 => WK::Digit6,
        GK::Key7 => WK::Digit7,
        GK::Key8 => WK::Digit8,
        GK::Key9 => WK::Digit9,
        GK::Key0 => WK::Digit0,
        GK::Return => WK::Enter,
        GK::Escape => WK::Escape,
        GK::Back => WK::Backspace,
        GK::Tab => WK::Tab,
        GK::Space => WK::Space,
        GK::Left => WK::ArrowLeft,
        GK::Right => WK::ArrowRight,
        GK::Up => WK::ArrowUp,
        GK::Down => WK::ArrowDown,
        GK::F1 => WK::F1,
        GK::F2 => WK::F2,
        GK::F3 => WK::F3,
        GK::F4 => WK::F4,
        GK::F5 => WK::F5,
        GK::F6 => WK::F6,
        GK::F7 => WK::F7,
        GK::F8 => WK::F8,
        GK::F9 => WK::F9,
        GK::F10 => WK::F10,
        GK::F11 => WK::F11,
        GK::F12 => WK::F12,
        _ => return None,
    })
}
