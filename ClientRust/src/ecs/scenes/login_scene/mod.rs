//! LoginScene模块 - 简洁OOP架构
mod login;
mod message_box;
mod new_account;
mod change_password;
mod network_handler;

pub use login::{LoginDialog, DialogAction};
pub use message_box::MessageBox;
pub use new_account::{NewAccountDialog, NewAccountAction, NewAccountResult, AccountRegistration, InputField};
pub use change_password::{ChangePasswordDialog, ChangePasswordAction, ChangePasswordResult, PasswordInputField};


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
    message_box: Option<MessageBox>,
    status_log: Vec<String>,
}

impl LoginScene {
    pub fn new() -> Self {
        // 初始使用占位尺寸，第一次绘制时会动态调整
        Self {
            connecting: false,
            login_enabled: true,
            background_frame: 0,
            animation_timer: 0.0,
            animation_paused: false,
            login_dialog: LoginDialog::new(1280.0, 720.0),  // 默认1280x720，会动态调整
            new_account_dialog: None,
            message_box: None,
            status_log: Vec::new(),
        }
    }
    
    pub fn show_message(&mut self, message: &str) {
        println!("🔔🔔🔔 show_message被调用: {}", message);
        tracing::info!("📬 显示消息框: {}", message);
        self.message_box = Some(MessageBox::new(message.to_string()));
        println!("🔔�🔔 消息框已创建: message_box={}", self.message_box.is_some());
    }
    
    fn submit_login(&mut self, network_tx: &mpsc::UnboundedSender<NetworkCommand>) {
        if let Some((account, password)) = self.login_dialog.get_credentials() {
            tracing::info!("🔐 提交登录: {}", account);
            let cmd = NetworkCommand::Login {
                username: account.clone(),
                password: password.clone(),
            };
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
        
        Ok(None)
    }
    
    fn draw(&mut self, ctx: &mut Context, canvas: &mut Canvas, _world: &World) -> GameResult {
        // 获取当前窗口尺寸
        let (screen_w, screen_h) = ctx.gfx.drawable_size();
        
        // 动态调整对话框位置(居中)
        let dialog_w = 328.0;
        let dialog_h = 220.0;
        self.login_dialog.x = (screen_w - dialog_w) / 2.0;
        self.login_dialog.y = (screen_h - dialog_h) / 2.0;
        
        // 同步更新所有子组件位置
        self.login_dialog.update_positions();
        
        // 更新新建账号对话框位置
        if let Some(dialog) = &mut self.new_account_dialog {
            dialog.update_positions(screen_w, screen_h);
        }
        
        // 更新消息框位置
        if let Some(msg_box) = &mut self.message_box {
            msg_box.update_positions(screen_w, screen_h);
        }
        
        // 绘制背景动画 (ChrSel库, Index 0-18, 共19帧)
        // 计算缩放比例: ChrSel背景原始尺寸是1024x768
        let bg_original_w = 1024.0;
        let bg_original_h = 768.0;
        let scale_x = screen_w / bg_original_w;
        let scale_y = screen_h / bg_original_h;
        
        let bg_index = self.background_frame as i32;
        let _ = draw_sprite_scaled(ctx, canvas, &LibraryName::ChrSel, bg_index, 0.0, 0.0, scale_x, scale_y);
        
        // 绘制登录对话框
        let _ = self.login_dialog.draw(ctx, canvas);
        
        // 绘制新建账号对话框(在登录对话框上层)
        if let Some(dialog) = &self.new_account_dialog {
            let _ = dialog.draw(ctx, canvas);
        }
        
        // 绘制消息框(最上层)
        if let Some(msg_box) = &self.message_box {
            let _ = msg_box.draw(ctx, canvas);
        }
        Ok(())
    }
    
    fn on_mouse_move(&mut self, _ctx: &mut Context, _world: &mut World, x: f32, y: f32) -> GameResult {
        if let Some(msg_box) = &mut self.message_box {
            msg_box.on_mouse_move(x, y);
            return Ok(());
        }
        
        // 新建账号对话框优先级高于登录对话框
        if let Some(dialog) = &mut self.new_account_dialog {
            dialog.on_mouse_move(x, y);
            return Ok(());
        }
        
        self.login_dialog.on_mouse_move(x, y);
        Ok(())
    }
    
    fn on_mouse_down(&mut self, ctx: &mut Context, _world: &mut World, _button: MouseButton, x: f32, y: f32, network_tx: &mpsc::UnboundedSender<NetworkCommand>) -> GameResult {
        if let Some(msg_box) = &mut self.message_box {
            if msg_box.on_mouse_down(x, y) {
                self.message_box = None;
            }
            return Ok(());
        }
        
        // 处理新建账号对话框
        if let Some(dialog) = &mut self.new_account_dialog {
            let action = dialog.on_mouse_down(x, y);
            match action {
                NewAccountAction::Submit => {
                    tracing::info!("📝 提交账号注册: {}", dialog.registration.account_id);
                    // 发送注册请求到服务器
                    // 注意: birth_date需要转换为Unix timestamp,暂时使用0
                    let cmd = NetworkCommand::NewAccount {
                        account_id: dialog.registration.account_id.clone(),
                        password: dialog.registration.password.clone(),
                        birth_date: 0, // TODO: 解析birth_date字符串转timestamp
                        username: dialog.registration.username.clone(),
                        secret_question: dialog.registration.secret_question.clone(),
                        secret_answer: dialog.registration.secret_answer.clone(),
                        email: dialog.registration.email.clone(),
                    };
                    if let Err(e) = network_tx.send(cmd) {
                        tracing::error!("❌ 发送注册命令失败: {}", e);
                        self.show_message("网络错误，无法发送注册请求");
                    }
                }
                NewAccountAction::Cancel => {
                    tracing::info!("❌ 取消账号注册");
                    self.new_account_dialog = None;
                }
                NewAccountAction::None => {}
            }
            return Ok(());
        }
        
        // 处理登录对话框
        let action = self.login_dialog.on_mouse_down(x, y);
        match action {
            DialogAction::Login => self.submit_login(network_tx),
            DialogAction::OpenNewAccount => {
                tracing::info!("🆕 打开新建账号对话框");
                let (screen_w, screen_h) = ctx.gfx.drawable_size();
                let mut dialog = NewAccountDialog::new(screen_w, screen_h);
                dialog.show();
                self.new_account_dialog = Some(dialog);
            }
            DialogAction::OpenChangePassword => self.show_message("修改密码功能待实现"),
            DialogAction::OpenViewKey => self.show_message("虚拟键盘功能待实现"),
            DialogAction::Exit => tracing::info!("🚪 退出游戏"),
            DialogAction::None => {}
        }
        Ok(())
    }
    
    fn on_key_down(&mut self, _ctx: &mut Context, _world: &mut World, input: KeyInput, network_tx: &mpsc::UnboundedSender<NetworkCommand>) -> GameResult<Option<SceneType>> {
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
        
        // 处理新建账号对话框输入
        if let Some(dialog) = &mut self.new_account_dialog {
            if let ggez::winit::event::KeyEvent {
                physical_key: PhysicalKey::Code(keycode),
                text,
                ..
            } = input.event
            {
                match keycode {
                    KeyCode::Escape => {
                        self.new_account_dialog = None;
                        return Ok(None);
                    }
                    KeyCode::Tab => {
                        dialog.on_tab();
                        return Ok(None);
                    }
                    KeyCode::Backspace => {
                        dialog.on_backspace();
                        return Ok(None);
                    }
                    KeyCode::Enter => {
                        if dialog.can_submit() {
                            tracing::info!("📝 提交账号注册(回车): {}", dialog.registration.account_id);
                            let cmd = NetworkCommand::NewAccount {
                                account_id: dialog.registration.account_id.clone(),
                                password: dialog.registration.password.clone(),
                                birth_date: 0, // TODO: 解析birth_date字符串转timestamp
                                username: dialog.registration.username.clone(),
                                secret_question: dialog.registration.secret_question.clone(),
                                secret_answer: dialog.registration.secret_answer.clone(),
                                email: dialog.registration.email.clone(),
                            };
                            if let Err(e) = network_tx.send(cmd) {
                                tracing::error!("❌ 发送注册命令失败: {}", e);
                                self.show_message("网络错误，无法发送注册请求");
                            }
                        }
                        return Ok(None);
                    }
                    _ => {
                        if let Some(text) = text {
                            for ch in text.chars() {
                                dialog.on_char(ch);
                            }
                        }
                    }
                }
            }
            return Ok(None);
        }
        
        // 处理登录对话框输入
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

