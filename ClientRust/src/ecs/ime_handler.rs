// ============================================================================
// IME Handler - 自定义 Ggez 事件循环，完整支持中文输入法
// ============================================================================
//
// 为什么需要自定义事件循环:
// - ggez 0.10 的默认 event::run() 不支持 WindowEvent::Ime
// - 中文输入必须捕获 Ime::Commit 事件
// - 需要手动实现 winit ApplicationHandler
//
// 参考: Client/bin/main_ggez.rs 的 CustomAppHandler
//
// ============================================================================

use crate::ecs::GameContext;
use anyhow::Result;
use ggez::context::{ContextFields, HasMut};
use ggez::event::EventHandler;
use ggez::graphics::GraphicsContext;
use ggez::input::keyboard::{KeyInput, KeyboardContext};
use ggez::input::mouse::MouseContext;
use ggez::winit::application::ApplicationHandler;
use ggez::winit::event::{DeviceEvent, DeviceId, StartCause, WindowEvent};
use ggez::winit::event::{ElementState, Ime};
use ggez::winit::event_loop::{ActiveEventLoop, ControlFlow, EventLoop};
use ggez::winit::window::WindowId;

/// 游戏 ApplicationHandler - 完整支持 IME 和 GameContext
///
/// 集成了 GameContext，提供：
/// - 完整的 IME 支持（中文输入）
/// - 输入事件收集（frame_input_events）
/// - 网络事件收集
pub struct GameAppHandler<T: EventHandler<GameContext>> {
    ctx: GameContext,
    game: T,
    ime_enabled: bool, // 🆕 跟踪 IME 状态
}

impl<T: EventHandler<GameContext>> GameAppHandler<T> {
    pub fn new(ctx: GameContext, game: T) -> Self {
        Self {
            ctx,
            game,
            ime_enabled: false, // 默认 IME 未启用
        }
    }
}

impl<T: EventHandler<GameContext>> ApplicationHandler<()> for GameAppHandler<T> {
    fn new_events(&mut self, event_loop: &ActiveEventLoop, _: StartCause) {
        // 检查是否请求退出
        if HasMut::<ContextFields>::retrieve_mut(&mut self.ctx).quit_requested {
            HasMut::<ContextFields>::retrieve_mut(&mut self.ctx).continuing = false;
        }
        if !HasMut::<ContextFields>::retrieve_mut(&mut self.ctx).continuing {
            event_loop.exit();
            return;
        }

        event_loop.set_control_flow(ControlFlow::Poll);
    }

    fn resumed(&mut self, _event_loop: &ActiveEventLoop) {}

    fn window_event(
        &mut self,
        event_loop: &ActiveEventLoop,
        mut window_id: WindowId,
        event: WindowEvent,
    ) {
        if let WindowEvent::KeyboardInput { ref event, .. } = event {
            if let Some(ref text) = event.text {
                tracing::debug!(
                    "🔍 [window_event] KeyboardInput 收到: text='{}', physical_key={:?}",
                    text,
                    event.physical_key
                );
            }
        }

        // ===== 首先检查 IME 事件 (这是获取中文的唯一方法!) =====
        if let WindowEvent::Ime(ref ime_event) = event {
            match ime_event {
                Ime::Enabled => {
                    tracing::debug!("IME 已启用");
                    self.ime_enabled = true; // 🆕 记录 IME 状态
                }
                Ime::Disabled => {
                    tracing::debug!("IME 已禁用");
                    self.ime_enabled = false; // 🆕 记录 IME 状态
                }
                Ime::Preedit(text, _) => {
                    tracing::debug!("IME 拼音: {}", text);
                    // Preedit 只是预览，不需要转发
                }
                Ime::Commit(text) => {
                    tracing::info!("✓ IME 确认中文: {}", text);
                    // 将字符串拆分为字符，逐个添加到输入事件并调用 text_input_event
                    for ch in text.chars() {
                        if let Err(e) = self.game.text_input_event(&mut self.ctx, ch) {
                            tracing::warn!("IME commit 字符处理失败: {}", e);
                        }
                    }
                }
            }
            // IME 事件不转发给 ggez
            return;
        }

        // ===== 转发事件给 ggez 更新内部状态 =====
        // 🔥 清除 KeyboardInput.text 字段，防止 ggez 重复处理文本输入
        let mut event_for_ggez = event.clone();
        if let WindowEvent::KeyboardInput { ref mut event, .. } = event_for_ggez {
            // 清除 text 字段，我们已经在下面手动处理了
            event.text = None;
        }
        ggez::event::process_window_event(&mut self.ctx, &mut window_id, &mut event_for_ggez);

        // ===== 手动调用 EventHandler 方法 =====
        match event {
            WindowEvent::CloseRequested => {
                if let Ok(true) = self.game.quit_event(&mut self.ctx) {
                    // 用户取消退出
                } else {
                    event_loop.exit();
                }
            }
            WindowEvent::Focused(gained) => {
                let _ = self.game.focus_event(&mut self.ctx, gained);
            }
            WindowEvent::Resized(size) => {
                let _ =
                    self.game
                        .resize_event(&mut self.ctx, size.width as f32, size.height as f32);
            }
            WindowEvent::KeyboardInput {
                event: key_event, ..
            } => {
                let mods = HasMut::<KeyboardContext>::retrieve_mut(&mut self.ctx).active_modifiers;
                let repeat =
                    HasMut::<KeyboardContext>::retrieve_mut(&mut self.ctx).is_key_repeated();
                let input = KeyInput {
                    event: key_event.clone(),
                    mods,
                };

                match input.event.state {
                    ElementState::Pressed => {
                        let _ = self.game.key_down_event(&mut self.ctx, input, repeat);

                        // 🔥 只在非重复按键时处理文本输入
                        // repeat = true 表示按键重复，不应该再次输入字符
                        if let Some(text) = &key_event.text {
                            tracing::debug!(
                                "⌨️ KeyboardInput.text = '{}', repeat = {}, ime_enabled = {}",
                                text,
                                repeat,
                                self.ime_enabled
                            );

                            // 跳过重复按键的文本输入
                            if repeat {
                                tracing::debug!("⌨️ 跳过重复按键的文本输入");
                                return;
                            }

                            if !self.ime_enabled {
                                // 过滤控制字符（如退格键 \x08、回车 \r 等）
                                // 只接受可打印字符
                                for ch in text.chars() {
                                    // 跳过控制字符（ASCII 0-31 和 127）
                                    if ch.is_control() {
                                        tracing::debug!("⌨️ 跳过控制字符: U+{:04X}", ch as u32);
                                        continue;
                                    }

                                    tracing::debug!("⌨️ 键盘文本输入: '{}'", ch);
                                    if let Err(e) = self.game.text_input_event(&mut self.ctx, ch) {
                                        tracing::warn!("键盘文本输入处理失败: {}", e);
                                    }
                                }
                            } else {
                                tracing::debug!("⌨️ IME 已启用，跳过 KeyboardInput.text 处理");
                            }
                        }
                    }
                    ElementState::Released => {
                        let _ = self.game.key_up_event(&mut self.ctx, input);
                    }
                }
            }
            WindowEvent::MouseInput { state, button, .. } => {
                let position = HasMut::<MouseContext>::retrieve_mut(&mut self.ctx).position();
                match state {
                    ElementState::Pressed => {
                        let _ = self.game.mouse_button_down_event(
                            &mut self.ctx,
                            button,
                            position.x,
                            position.y,
                        );
                    }
                    ElementState::Released => {
                        let _ = self.game.mouse_button_up_event(
                            &mut self.ctx,
                            button,
                            position.x,
                            position.y,
                        );
                    }
                }
            }
            WindowEvent::CursorMoved { .. } => {
                let position = HasMut::<MouseContext>::retrieve_mut(&mut self.ctx).position();
                let delta = HasMut::<MouseContext>::retrieve_mut(&mut self.ctx).last_delta();
                let _ = self.game.mouse_motion_event(
                    &mut self.ctx,
                    position.x,
                    position.y,
                    delta.x,
                    delta.y,
                );
            }
            WindowEvent::MouseWheel { delta, .. } => {
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
        _event_loop: &ActiveEventLoop,
        mut device_id: DeviceId,
        mut event: DeviceEvent,
    ) {
        // 转发设备事件给 ggez
        ggez::event::process_device_event(&mut self.ctx, &mut device_id, &mut event);
    }

    fn about_to_wait(&mut self, event_loop: &ActiveEventLoop) {
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
        HasMut::<MouseContext>::retrieve_mut(&mut self.ctx).reset_delta();
        HasMut::<KeyboardContext>::retrieve_mut(&mut self.ctx).save_keyboard_state();
        HasMut::<MouseContext>::retrieve_mut(&mut self.ctx).save_mouse_state();
    }
}

/// 自定义事件循环 - 完整支持 IME
///
/// 替代 ggez::event::run()，手动实现事件分发
pub fn run_with_ime<T: EventHandler<GameContext> + 'static>(
    ctx: GameContext,
    event_loop: EventLoop<()>,
    game: T,
) -> Result<()> {
    let mut app = GameAppHandler::new(ctx, game);

    event_loop
        .run_app(&mut app)
        .map_err(|e| anyhow::anyhow!("事件循环错误: {}", e))
}
