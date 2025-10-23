//! LoginScene模块 - 简洁OOP架构
mod login;
mod message_box;
mod new_account;
mod change_password;
mod network_handler;
mod dialog_manager;

pub use login::{LoginDialog, DialogAction};
pub use message_box::MessageBox;
pub use new_account::{NewAccountDialog, NewAccountAction, NewAccountResult, AccountRegistration, InputField};
pub use change_password::{ChangePasswordDialog, ChangePasswordAction, ChangePasswordResult, PasswordInputField};
use dialog_manager::{handle_dialog_keycode, DialogKeyResult};


use ggez::{Context, GameResult};
use ggez::graphics::Canvas;
use ggez::input::keyboard::KeyInput;
use ggez::winit::event::MouseButton;
use ggez::winit::keyboard::{KeyCode, PhysicalKey};
use tokio::sync::mpsc;
use hecs::World;

use super::{Scene, SceneType};
use crate::network::NetworkCommand;
use crate::graphics::{LibraryName, draw_sprite_at, draw_sprite_scaled};

/// 登录场景
pub struct LoginScene {
    connecting: bool,
    login_enabled: bool,
    background_frame: usize,
    animation_timer: f32,
    animation_paused: bool,
    login_dialog: LoginDialog,
    new_account_dialog: Option<NewAccountDialog>,
    change_password_dialog: Option<ChangePasswordDialog>,
    message_box: Option<MessageBox>,
    status_log: Vec<String>,
    // 坐标转换参数(屏幕坐标 -> 虚拟坐标)
    scale: f32,
    offset_x: f32,
    offset_y: f32,
}

impl LoginScene {
    pub fn new() -> Self {
        // 初始使用占位尺寸，第一次绘制时会动态调整
        Self {
            connecting: false,
            login_enabled: true,
            background_frame: 0,
            animation_timer: 0.0,
            animation_paused: true,  // C#原版: 默认暂停,只有登录成功后才播放动画
            login_dialog: LoginDialog::new(1280.0, 720.0),  // 默认1280x720，会动态调整
            new_account_dialog: None,
            change_password_dialog: None,
            message_box: None,
            status_log: Vec::new(),
            scale: 1.0,
            offset_x: 0.0,
            offset_y: 0.0,
        }
    }
    
    /// 屏幕坐标转换为虚拟1280×720坐标
    fn screen_to_virtual(&self, screen_x: f32, screen_y: f32) -> (f32, f32) {
        let virtual_x = (screen_x - self.offset_x) / self.scale;
        let virtual_y = (screen_y - self.offset_y) / self.scale;
        (virtual_x, virtual_y)
    }
    
    pub fn show_message(&mut self, message: &str) {
        tracing::info!("📬 显示消息框: {}", message);
        self.message_box = Some(MessageBox::new(message.to_string()));
    }
    
    fn submit_login(&mut self, network_tx: &mpsc::UnboundedSender<NetworkCommand>) {
        if let Some(cmd) = self.login_dialog.build_network_command() {
            if let Err(e) = network_tx.send(cmd) {
                tracing::error!("❌ 发送登录命令失败: {}", e);
                self.show_message("网络错误，无法发送登录请求");
                return;
            }
            self.connecting = true;
            self.login_enabled = false;
        } else {
            self.show_message("请输入账号和密码");
        }
    }
}

impl Scene for LoginScene {
    fn as_any_mut(&mut self) -> &mut dyn std::any::Any {
        self
    }
    
    fn update(&mut self, ctx: &mut Context, _world: &mut World, _network_tx: &mpsc::UnboundedSender<NetworkCommand>) -> GameResult<Option<SceneType>> {
        let dt = ctx.time.delta().as_secs_f32();
        if !self.animation_paused {
            self.animation_timer += dt;
            if self.animation_timer >= 0.1 {
                // C#原版: AnimationCount = 19, 即0-18帧
                self.background_frame = (self.background_frame + 1) % 19;
                self.animation_timer = 0.0;
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
        // 获取当前窗口尺寸
        let (screen_w, screen_h) = ctx.gfx.drawable_size();
        
        // 计算全局UI缩放因子 (基准分辨率: 1280x720)
        let base_w = 1280.0;
        let base_h = 720.0;
        self.scale = (screen_w / base_w).min(screen_h / base_h);
        
        // 计算居中偏移 (缩放后的内容居中显示)
        let scaled_w = base_w * self.scale;
        let scaled_h = base_h * self.scale;
        self.offset_x = (screen_w - scaled_w) / 2.0;
        self.offset_y = (screen_h - scaled_h) / 2.0;
        
        // 保存Canvas状态并应用全局变换
        canvas.set_screen_coordinates(ggez::graphics::Rect::new(0.0, 0.0, screen_w, screen_h));
        
        // 绘制背景动画 (ChrSel库, Index 0-18, 共19帧) - 背景铺满整个屏幕
        let bg_original_w = 1024.0;
        let bg_original_h = 768.0;
        let bg_scale_x = screen_w / bg_original_w;
        let bg_scale_y = screen_h / bg_original_h;
        let bg_index = self.background_frame as i32;
        let _ = draw_sprite_scaled(ctx, canvas, &LibraryName::ChrSel, bg_index, 0.0, 0.0, bg_scale_x, bg_scale_y);
        
        // 应用UI缩放和偏移变换
        canvas.set_screen_coordinates(ggez::graphics::Rect::new(
            -self.offset_x / self.scale,
            -self.offset_y / self.scale,
            screen_w / self.scale,
            screen_h / self.scale
        ));
        
        // 更新对话框位置(基于1280x720坐标系)
        let dialog_w = 328.0;
        let dialog_h = 220.0;
        self.login_dialog.x = (base_w - dialog_w) / 2.0;
        self.login_dialog.y = (base_h - dialog_h) / 2.0;
        self.login_dialog.update_positions();
        
        // 更新新建账号对话框位置
        if let Some(dialog) = &mut self.new_account_dialog {
            dialog.update_positions(base_w, base_h);
        }
        
        // 更新修改密码对话框位置
        if let Some(dialog) = &mut self.change_password_dialog {
            dialog.update_positions(base_w, base_h);
        }
        
        // 更新消息框位置
        if let Some(msg_box) = &mut self.message_box {
            msg_box.update_positions(base_w, base_h);
        }
        
        // 绘制所有UI元素(在缩放后的坐标系中)
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
        
        Ok(())
    }
    
    fn on_mouse_move(&mut self, _ctx: &mut Context, _world: &mut World, x: f32, y: f32) -> GameResult {
        // 将屏幕坐标转换为虚拟1280×720坐标
        let (vx, vy) = self.screen_to_virtual(x, y);
        
        if let Some(msg_box) = &mut self.message_box {
            msg_box.on_mouse_move(vx, vy);
            return Ok(());
        }
        
        // 修改密码对话框优先级最高
        if let Some(dialog) = &mut self.change_password_dialog {
            dialog.on_mouse_move(vx, vy);
            return Ok(());
        }
        
        // 新建账号对话框优先级高于登录对话框
        if let Some(dialog) = &mut self.new_account_dialog {
            dialog.on_mouse_move(vx, vy);
            return Ok(());
        }
        
        self.login_dialog.on_mouse_move(vx, vy);
        Ok(())
    }
    
    fn on_mouse_down(&mut self, _ctx: &mut Context, _world: &mut World, _button: MouseButton, x: f32, y: f32, network_tx: &mpsc::UnboundedSender<NetworkCommand>) -> GameResult {
        // 将屏幕坐标转换为虚拟1280×720坐标
        let (vx, vy) = self.screen_to_virtual(x, y);
        
        if let Some(msg_box) = &mut self.message_box {
            if msg_box.on_mouse_down(vx, vy) {
                self.message_box = None;
            }
            return Ok(());
        }
        
        // 处理修改密码对话框
        if let Some(dialog) = &mut self.change_password_dialog {
            let action = dialog.on_mouse_down(vx, vy);
            match action {
                ChangePasswordAction::Submit => {
                    // 构建并发送网络命令
                    let cmd = dialog.build_network_command();
                    if let Err(e) = network_tx.send(cmd) {
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
            let action = dialog.on_mouse_down(vx, vy);
            match action {
                NewAccountAction::Submit => {
                    // 构建并发送网络命令
                    let cmd = dialog.build_network_command();
                    if let Err(e) = network_tx.send(cmd) {
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
        let action = self.login_dialog.on_mouse_down(vx, vy);
        match action {
            DialogAction::Login => self.submit_login(network_tx),
            DialogAction::OpenNewAccount => {
                tracing::info!("🆕 打开新建账号对话框");
                let base_w = 1280.0;
                let base_h = 720.0;
                let mut dialog = NewAccountDialog::new(base_w, base_h);
                dialog.show();
                self.new_account_dialog = Some(dialog);
            }
            DialogAction::OpenChangePassword => {
                tracing::info!("🔑 打开修改密码对话框");
                // 从登录框预填充账号和密码
                let (account_id, password) = self.login_dialog.get_credentials()
                    .map(|(id, pwd)| (Some(id), Some(pwd)))
                    .unwrap_or((None, None));
                let mut dialog = ChangePasswordDialog::new();
                dialog.show(account_id, password);
                self.change_password_dialog = Some(dialog);
            }
            DialogAction::OpenViewKey => self.show_message("虚拟键盘功能待实现"),
            DialogAction::Exit => tracing::info!("🚪 退出游戏"),
            DialogAction::None => {}
        }
        Ok(())
    }
    
    fn on_key_down(&mut self, _ctx: &mut Context, _world: &mut World, input: KeyInput, network_tx: &mpsc::UnboundedSender<NetworkCommand>) -> GameResult<Option<SceneType>> {
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
                    network_tx,
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
                    network_tx,
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
                        self.submit_login(network_tx);
                    }
                }
                KeyCode::Backspace => self.login_dialog.on_backspace(),
                _ => {
                    if let Some(text) = text {
                        for ch in text.chars() {
                            self.login_dialog.on_char(ch);
                        }
                    }
                }
            }
        }
        Ok(None)
    }
}

