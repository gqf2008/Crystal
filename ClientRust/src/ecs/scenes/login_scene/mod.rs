//! LoginScene模块 - 简洁OOP架构
mod login;
mod message_box;
mod new_account;
mod change_password;
mod network_handler;
mod dialog_manager;
mod virtual_keyboard;

pub use login::{LoginDialog, DialogAction};
pub use message_box::MessageBox;
pub use new_account::{NewAccountDialog, NewAccountAction, NewAccountResult, AccountRegistration, InputField};
pub use change_password::{ChangePasswordDialog, ChangePasswordAction, ChangePasswordResult, PasswordInputField};
pub use virtual_keyboard::{VirtualKeyboard, VirtualKeyboardAction, FocusedInput};
use dialog_manager::{handle_dialog_keycode, DialogKeyResult};


use ggez::{Context, GameResult};
use ggez::graphics::Canvas;
use ggez::input::keyboard::KeyInput;
use ggez::winit::event::MouseButton;
use ggez::winit::keyboard::{KeyCode, PhysicalKey};
use std::sync::Arc;
use hecs::World;

use super::{Scene, SceneType};
use crate::network::{NetContext, handlers::GameEvent};
use crate::graphics::{LibraryName, draw_sprite_at};

/// 登录场景
pub struct LoginScene {
    connecting: bool,
    login_enabled: bool,
    version_verified: bool, // 🆕 ClientVersion是否已被服务器验证
    should_switch_scene: bool, // 🆕 登录成功后请求切换场景
    /// 如果用户在 ClientVersion 未验证前尝试登录,把命令缓存起来,验证通过后自动发送
    pending_login: Option<crate::network::handlers::GameEvent>,
    /// 🆕 登录节流: 上次提交登录的时间
    last_login_attempt: Option<std::time::Instant>,
    background_frame: usize,
    animation_timer: f32,
    animation_paused: bool,
    login_dialog: LoginDialog,
    new_account_dialog: Option<NewAccountDialog>,
    change_password_dialog: Option<ChangePasswordDialog>,
    message_box: Option<MessageBox>,
    virtual_keyboard: Option<VirtualKeyboard>,
}

// 设计分辨率：UI纹理的原始设计分辨率（4:3比例）
const DESIGN_WIDTH: f32 = 1024.0;
const DESIGN_HEIGHT: f32 = 768.0;

impl LoginScene {
    pub fn new() -> Self {
        Self {
            connecting: false,
            login_enabled: true,
            version_verified: false, // 🆕 初始未验证
            should_switch_scene: false, // 🆕 初始不切换场景
            pending_login: None,
            last_login_attempt: None, // 🆕 初始未尝试登录
            background_frame: 0,
            animation_timer: 0.0,
            animation_paused: true,
            login_dialog: LoginDialog::new(DESIGN_WIDTH, DESIGN_HEIGHT),
            new_account_dialog: None,
            change_password_dialog: None,
            message_box: None,
            virtual_keyboard: None,
        }
    }
    
    pub fn show_message(&mut self, message: &str) {
        self.message_box = Some(MessageBox::new(message.to_string(), DESIGN_WIDTH, DESIGN_HEIGHT));
    }
    
    /// 将窗口坐标转换为设计坐标系（1280x960）
    fn window_to_design_coords(&self, ctx: &Context, window_x: f32, window_y: f32) -> (f32, f32) {
        let  (window_width,window_height) = ctx.gfx.drawable_size();
        // 计算4:3视口
        let aspect_ratio = 4.0 / 3.0;
        let current_ratio = window_width / window_height;
        
        let (viewport_width, viewport_height) = if current_ratio > aspect_ratio {
            (window_height * aspect_ratio, window_height)
        } else {
            (window_width, window_width / aspect_ratio)
        };
        
        let offset_x = (window_width - viewport_width) / 2.0;
        let offset_y = (window_height - viewport_height) / 2.0;
        
        // 转换：窗口坐标 -> 视口坐标 -> 设计坐标
        let viewport_x = window_x - offset_x;
        let viewport_y = window_y - offset_y;
        
        let design_x = (viewport_x / viewport_width) * DESIGN_WIDTH;
        let design_y = (viewport_y / viewport_height) * DESIGN_HEIGHT;
        
        (design_x, design_y)
    }
    
    fn submit_login(&mut self, net_ctx: &Arc<NetContext>) {
        // 🔒 节流: 防止短时间内重复提交 (1秒冷却)
        if let Some(last_attempt) = self.last_login_attempt {
            if last_attempt.elapsed() < std::time::Duration::from_secs(1) {
                tracing::warn!("⚠️ 登录请求过于频繁,请稍后再试 (需等待1秒)");
                self.show_message("请不要频繁点击，稍后再试");
                return;
            }
        }
        
        // 🔒 检查版本是否已验证
        if !self.version_verified {
            tracing::warn!("⚠️ ClientVersion尚未验证,将缓存登录请求并在验证通过后自动发送");
            // 如果有可用的登录命令,保存以便在版本验证通过后自动发送
            if let Some(cmd) = self.login_dialog.build_network_command() {
                self.pending_login = Some(cmd);
                self.show_message("正在等待版本校验，登录将在验证通过后自动发送");
                self.last_login_attempt = Some(std::time::Instant::now()); // 🆕 记录尝试时间
            } else {
                // 没有有效凭证,提示用户输入
                self.show_message("请输入账号和密码");
            }
            return;
        }
        
        // 🔒 防止重复登录
        if !self.login_enabled {
            tracing::warn!("⚠️ 登录已在进行中,忽略重复请求");
            return;
        }
        
        if let Some(cmd) = self.login_dialog.build_network_command() {
            if let Err(e) = net_ctx.send(cmd) {
                tracing::error!("❌ 发送登录命令失败: {}", e);
                self.show_message("网络错误，无法发送登录请求");
                return;
            }
            tracing::info!("✅ 登录命令已发送,禁用重复提交");
            self.connecting = true;
            self.login_enabled = false;
            self.last_login_attempt = Some(std::time::Instant::now()); // 🆕 记录登录时间
        } else {
            self.show_message("请输入账号和密码");
        }
    }
}

impl Scene for LoginScene {
    fn as_any_mut(&mut self) -> &mut dyn std::any::Any {
        self
    }
    
    fn update(&mut self, ctx: &mut Context, world: &mut World, net_ctx: &Arc<NetContext>) -> GameResult<Option<SceneType>> {
        // 🔄 处理网络事件
        while let Some(event) = net_ctx.try_recv() {
            self.handle_network_event(&event, net_ctx, world);
        }
        
        let dt = ctx.time.delta().as_secs_f32();
        if !self.animation_paused {
            self.animation_timer += dt;
            if self.animation_timer >= 0.1 {
                self.background_frame += 1;
                self.animation_timer = 0.0;
                
                // 🆕 动画播放完19帧(0-18)后切换场景
                if self.background_frame >= 19 {
                    self.background_frame = 0; // 重置到第0帧
                    if self.should_switch_scene {
                        tracing::info!("🎬 登录成功动画播放完成,切换到角色选择场景");
                        return Ok(Some(SceneType::Select));
                    }
                }
            }
        }
        self.login_dialog.update(dt);
        
        // 更新新建账号对话框
        if let Some(dialog) = &mut self.new_account_dialog {
            dialog.update(dt);
        }
        
        // 更新修改密码对话框
        if let Some(dialog) = &mut self.change_password_dialog {
            dialog.update(dt);
        }
        
        Ok(None)
    }
    
    fn draw(&mut self, ctx: &mut Context, canvas: &mut Canvas, _world: &World) -> GameResult {
        canvas.set_screen_coordinates(ggez::graphics::Rect::new(0.0, 0.0, DESIGN_WIDTH, DESIGN_HEIGHT));
        
        // 绘制背景动画(ChrSel库, 1024x768原始尺寸，直接铺满设计坐标系)
        let bg_index = self.background_frame as i32;
        let _ = draw_sprite_at(ctx, canvas, &LibraryName::ChrSel, bg_index, 0.0, 0.0);
        
        // 🆕 登录成功后播放动画时,不再绘制UI界面(只保留背景动画)
        if !self.animation_paused {
            // 动画播放中,跳过所有UI绘制
            return Ok(());
        }
        
        // 更新对话框位置(在设计坐标系中居中)
        let dialog_w = 328.0;
        let dialog_h = 220.0;
        self.login_dialog.x = (DESIGN_WIDTH - dialog_w) / 2.0;
        self.login_dialog.y = (DESIGN_HEIGHT - dialog_h) / 2.0;
        self.login_dialog.update_positions();
        
        // 更新新建账号对话框位置
        if let Some(dialog) = &mut self.new_account_dialog {
            dialog.update_positions();
        }
        
        // 更新修改密码对话框位置
        if let Some(dialog) = &mut self.change_password_dialog {
            dialog.update_positions();
        }
        
        // 更新消息框位置
        if let Some(msg_box) = &mut self.message_box {
            msg_box.update_positions(DESIGN_WIDTH, DESIGN_HEIGHT);
        }
        
        // 绘制所有UI元素(在设计坐标系中)
        let _ = self.login_dialog.draw(ctx, canvas);
        
        if let Some(dialog) = &self.new_account_dialog {
            let _ = dialog.draw(ctx, canvas);
        }
        
        if let Some(dialog) = &self.change_password_dialog {
            let _ = dialog.draw(ctx, canvas);
        }
        
        if let Some(msg_box) = &self.message_box {
            let _ = msg_box.draw(ctx, canvas);
        }
        
        // 虚拟键盘在最上层
        if let Some(keyboard) = &self.virtual_keyboard {
            let _ = keyboard.draw(ctx, canvas);
        }
        
        Ok(())
    }
    
    fn on_mouse_move(&mut self, ctx: &mut Context, _world: &mut World, x: f32, y: f32) -> GameResult {
        // 将窗口坐标转换为设计坐标系（1280x960）
        let (design_x, design_y) = self.window_to_design_coords(ctx, x, y);
        
        // 虚拟键盘优先级最高,但也更新背后的界面(允许悬停效果)
        if let Some(keyboard) = &mut self.virtual_keyboard {
            keyboard.on_mouse_move(design_x, design_y);
            // 继续更新背后的登录界面,让按钮保持悬停效果
            self.login_dialog.on_mouse_move(design_x, design_y);
            return Ok(());
        }
        
        if let Some(msg_box) = &mut self.message_box {
            msg_box.on_mouse_move(design_x, design_y);
            return Ok(());
        }
        
        // 修改密码对话框优先级最高
        if let Some(dialog) = &mut self.change_password_dialog {
            dialog.on_mouse_move(design_x, design_y);
            return Ok(());
        }
        
        // 新建账号对话框优先级高于登录对话框
        if let Some(dialog) = &mut self.new_account_dialog {
            dialog.on_mouse_move(design_x, design_y);
            return Ok(());
        }
        
        self.login_dialog.on_mouse_move(design_x, design_y);
        Ok(())
    }
    
    fn on_mouse_down(&mut self, ctx: &mut Context, _world: &mut World, _button: MouseButton, x: f32, y: f32, net_ctx: &Arc<NetContext>) -> GameResult {
        // 将窗口坐标转换为设计坐标系
        let (design_x, design_y) = self.window_to_design_coords(ctx, x, y);
        
        // 虚拟键盘优先级最高
        if let Some(keyboard) = &mut self.virtual_keyboard {
            let action = keyboard.on_mouse_down(design_x, design_y);
            match action {
                VirtualKeyboardAction::Close => {
                    self.virtual_keyboard = None;
                }
                VirtualKeyboardAction::Delete => {
                    // 删除当前焦点输入框的最后一个字符
                    match keyboard.focused_input {
                        FocusedInput::Account => {
                            self.login_dialog.account_input.backspace();
                        }
                        FocusedInput::Password => {
                            self.login_dialog.password_input.backspace();
                        }
                    }
                }
                VirtualKeyboardAction::Input(ch) => {
                    // 输入字符到当前焦点输入框
                    if ch.is_ascii_alphanumeric() {
                        match keyboard.focused_input {
                            FocusedInput::Account => {
                                self.login_dialog.account_input.add_char(ch.to_ascii_lowercase());
                            }
                            FocusedInput::Password => {
                                self.login_dialog.password_input.add_char(ch.to_ascii_lowercase());
                            }
                        }
                    }
                }
                VirtualKeyboardAction::None => {}
            }
            return Ok(());
        }
        
        if let Some(msg_box) = &mut self.message_box {
            if msg_box.on_mouse_down(design_x, design_y) {
                self.message_box = None;
            }
            return Ok(());
        }
        
        // 处理修改密码对话框
        if let Some(dialog) = &mut self.change_password_dialog {
            let action = dialog.on_mouse_down(design_x, design_y);
            match action {
                ChangePasswordAction::Submit => {
                    // 构建并发送网络命令
                    let cmd = dialog.build_network_command();
                    if let Err(e) = net_ctx.send(cmd) {
                        tracing::error!("❌ 发送修改密码命令失败: {}", e);
                        self.show_message("网络错误，无法发送修改密码请求");
                    }
                }
                ChangePasswordAction::ValidationFailed(error_msg) => {
                    self.show_message(&error_msg);
                }
                ChangePasswordAction::Cancel => {
                    self.change_password_dialog = None;
                }
                ChangePasswordAction::None => {}
            }
            return Ok(());
        }
        
        // 处理新建账号对话框
        if let Some(dialog) = &mut self.new_account_dialog {
            let action = dialog.on_mouse_down(design_x, design_y);
            match action {
                NewAccountAction::Submit => {
                    // 构建并发送网络命令
                    let cmd = dialog.build_network_command();
                    if let Err(e) = net_ctx.send(cmd) {
                        tracing::error!("❌ 发送注册命令失败: {}", e);
                        self.show_message("网络错误，无法发送注册请求");
                    }
                }
                NewAccountAction::ValidationFailed(error_msg) => {
                    self.show_message(&error_msg);
                }
                NewAccountAction::Cancel => {
                    self.new_account_dialog = None;
                }
                NewAccountAction::None => {}
            }
            return Ok(());
        }
        
        // 处理登录对话框
        let action = self.login_dialog.on_mouse_down(design_x, design_y);
        match action {
            DialogAction::Login => self.submit_login(net_ctx),
            DialogAction::OpenNewAccount => {
                tracing::info!("🆕 打开新建账号对话框");
                let mut dialog = NewAccountDialog::new(DESIGN_WIDTH, DESIGN_HEIGHT);
                dialog.show();
                self.new_account_dialog = Some(dialog);
            }
            DialogAction::OpenChangePassword => {
                tracing::info!("🔑 打开修改密码对话框");
                // 从登录框预填充账号和密码
                let (account_id, password) = self.login_dialog.get_credentials()
                    .map(|(id, pwd)| (Some(id), Some(pwd)))
                    .unwrap_or((None, None));
                let mut dialog = ChangePasswordDialog::new(DESIGN_WIDTH, DESIGN_HEIGHT);
                dialog.show(account_id, password);
                self.change_password_dialog = Some(dialog);
            }
            DialogAction::OpenViewKey => {
                tracing::info!("⌨️ 打开虚拟键盘");
                let mut keyboard = VirtualKeyboard::new(DESIGN_WIDTH, DESIGN_HEIGHT);
                // 根据当前焦点决定虚拟键盘输入目标
                let focused = if self.login_dialog.account_input.focused {
                    FocusedInput::Account
                } else {
                    FocusedInput::Password
                };
                keyboard.show(focused);
                self.virtual_keyboard = Some(keyboard);
            }
            DialogAction::Exit => tracing::info!("🚪 退出游戏"),
            DialogAction::None => {}
        }
        Ok(())
    }
    
    fn on_key_down(&mut self, _ctx: &mut Context, _world: &mut World, input: KeyInput, net_ctx: &Arc<NetContext>) -> GameResult<Option<SceneType>> {
        // 虚拟键盘优先级最高(处理ESC/Backspace/Enter/Space)
        if let Some(keyboard) = &mut self.virtual_keyboard {
            if let ggez::winit::event::KeyEvent {
                physical_key: PhysicalKey::Code(keycode),
                ..
            } = input.event
            {
                match keycode {
                    KeyCode::Escape | KeyCode::Enter => {
                        // ESC或Enter关闭虚拟键盘
                        self.virtual_keyboard = None;
                    }
                    KeyCode::Backspace => {
                        // 删除字符
                        match keyboard.focused_input {
                            FocusedInput::Account => {
                                self.login_dialog.account_input.backspace();
                            }
                            FocusedInput::Password => {
                                self.login_dialog.password_input.backspace();
                            }
                        }
                    }
                    KeyCode::Space => {
                        // 空格键输入空格
                        match keyboard.focused_input {
                            FocusedInput::Account => {
                                self.login_dialog.account_input.add_char(' ');
                            }
                            FocusedInput::Password => {
                                self.login_dialog.password_input.add_char(' ');
                            }
                        }
                    }
                    _ => {}
                }
            }
            return Ok(None);
        }
        
        // 消息框优先级最高
        if self.message_box.is_some() {
            if let ggez::winit::event::KeyEvent {
                physical_key: PhysicalKey::Code(keycode),
                ..
            } = input.event
            {
                if matches!(keycode, KeyCode::Escape | KeyCode::Enter) {
                    self.message_box = None;
                }
            }
            return Ok(None);
        }
        
        // 处理修改密码对话框（优先级高）
        if let Some(dialog) = &mut self.change_password_dialog {
            if let ggez::winit::event::KeyEvent {
                physical_key: PhysicalKey::Code(keycode),
                text,
                ..
            } = input.event
            {
                let result = handle_dialog_keycode(
                    dialog,
                    keycode,
                    text.as_deref(),
                    net_ctx,
                    "发送修改密码命令失败",
                );
                match result {
                    DialogKeyResult::Close => self.change_password_dialog = None,
                    DialogKeyResult::ValidationFailed(msg) | DialogKeyResult::SendError(msg) => {
                        self.show_message(&msg);
                    }
                    DialogKeyResult::Handled => {}
                }
            }
            return Ok(None);
        }
        
        // 处理新建账号对话框
        if let Some(dialog) = &mut self.new_account_dialog {
            if let ggez::winit::event::KeyEvent {
                physical_key: PhysicalKey::Code(keycode),
                text,
                ..
            } = input.event
            {
                let result = handle_dialog_keycode(
                    dialog,
                    keycode,
                    text.as_deref(),
                    net_ctx,
                    "发送注册命令失败",
                );
                match result {
                    DialogKeyResult::Close => self.new_account_dialog = None,
                    DialogKeyResult::ValidationFailed(msg) | DialogKeyResult::SendError(msg) => {
                        self.show_message(&msg);
                    }
                    DialogKeyResult::Handled => {}
                }
            }
            return Ok(None);
        }
        
        // 处理登录对话框
        if let ggez::winit::event::KeyEvent {
            physical_key: PhysicalKey::Code(keycode),
            text,
            ..
        } = input.event
        {
            match keycode {
                KeyCode::Tab => self.login_dialog.on_tab(),
                KeyCode::Enter => {
                    let action = self.login_dialog.on_enter();
                    if action == DialogAction::Login {
                        self.submit_login(net_ctx);
                    }
                }
                KeyCode::Backspace => self.login_dialog.on_backspace(),
                _ => {
                    // 🔥 不要在这里处理 text 字段，因为 on_text_input 会处理
                    // 避免重复输入
                }
            }
        }
        Ok(None)
    }
    
    //fn on_resize(&mut self, ctx: &mut Context, _world: &mut World, width: f32, height: f32) -> GameResult {
       // panic!("登录场景不应处理窗口调整事件 {}x{}", width, height);
        // 强制保持4:3比例
        // let aspect_ratio = 4.0 / 3.0;
        // let current_ratio = width / height;
        
        // let (new_width, new_height) = if (current_ratio - aspect_ratio).abs() > 0.01 {
        //     // 比例不对，调整窗口大小
        //     if current_ratio > aspect_ratio {
        //         // 太宽，缩小宽度
        //         (height * aspect_ratio, height)
        //     } else {
        //         // 太高，缩小高度
        //         (width, width / aspect_ratio)
        //     }
        // } else {
        //     (width, height)
        // };
        
        // // 更新窗口大小以保持4:3比例
        // if (new_width - width).abs() > 1.0 || (new_height - height).abs() > 1.0 {
        //     ctx.gfx.set_drawable_size(new_width, new_height)?;
        //     tracing::debug!("🔧 窗口调整为4:3比例: {}x{}", new_width, new_height);
        // }
        
        //Ok(())
   // }
    
    fn on_text_input(&mut self, _ctx: &mut Context, _world: &mut World, character: String) -> GameResult {
        tracing::debug!("📝 LoginScene::on_text_input: '{}'", character);
        
        // 转发到登录对话框
        self.login_dialog.on_text_input(&character);
        
        // 转发到新建账号对话框
        if let Some(dialog) = &mut self.new_account_dialog {
            dialog.on_text_input(&character);
        }
        
        // 转发到修改密码对话框
        if let Some(dialog) = &mut self.change_password_dialog {
            dialog.on_text_input(&character);
        }
        
        Ok(())
    }
}
