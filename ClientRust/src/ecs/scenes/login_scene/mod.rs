//! LoginScene模块 - 简洁OOP架构
mod change_password;
mod dialog_manager;
mod input_handler;
mod login;
mod message_box;
mod network_handler;
mod new_account;
mod virtual_keyboard;

pub use change_password::{
    ChangePasswordAction, ChangePasswordDialog, ChangePasswordResult, PasswordInputField,
};
pub use login::{DialogAction, LoginDialog};
pub use message_box::MessageBox;
pub use new_account::{
    AccountRegistration, InputField, NewAccountAction, NewAccountDialog, NewAccountResult,
};
pub use virtual_keyboard::{FocusedInput, VirtualKeyboard, VirtualKeyboardAction};

use ggez::graphics::Canvas;
use ggez::{Context, GameResult};
use hecs::World;

use super::{Scene, SceneType};
use crate::ecs::{Coord, WorldExt};
use crate::graphics::{draw_sprite_at, LibraryName};

/// 登录场景
pub struct LoginScene {
    connecting: bool,
    login_enabled: bool,
    version_verified: bool,    // 🆕 ClientVersion是否已被服务器验证
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


impl LoginScene {
    pub fn new() -> Self {
        Self {
            connecting: false,
            login_enabled: true,
            version_verified: false,    // 🆕 初始未验证
            should_switch_scene: false, // 🆕 初始不切换场景
            pending_login: None,
            last_login_attempt: None, // 🆕 初始未尝试登录
            background_frame: 0,
            animation_timer: 0.0,
            animation_paused: true,
            login_dialog: LoginDialog::new(Coord::DESIGN_WIDTH, Coord::DESIGN_HEIGHT),
            new_account_dialog: None,
            change_password_dialog: None,
            message_box: None,
            virtual_keyboard: None,
        }
    }

    pub fn show_message(&mut self, message: &str) {
        self.message_box = Some(MessageBox::new(
            message.to_string(),
            Coord::DESIGN_WIDTH,
            Coord::DESIGN_HEIGHT,
        ));
    }

    /// 将窗口坐标转换为设计坐标系（1280x960）
    fn window_to_design_coords(&self, ctx: &Context, window_x: f32, window_y: f32) -> (f32, f32) {
        let (window_width, window_height) = ctx.gfx.drawable_size();
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

        let design_x = (viewport_x / viewport_width) * Coord::DESIGN_WIDTH;
        let design_y = (viewport_y / viewport_height) * Coord::DESIGN_HEIGHT;

        (design_x, design_y)
    }

    fn submit_login(&mut self, world: &mut World) {
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
            if let Err(e) = world.network().send(cmd) {
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

    fn update(&mut self, ctx: &mut Context, world: &mut World) -> GameResult<Option<SceneType>> {
        self.handle_input_event(ctx, world)?;
        self.handle_network_event(world);

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
        canvas.set_screen_coordinates(ggez::graphics::Rect::new(
            0.0,
            0.0,
            Coord::DESIGN_WIDTH,
            Coord::DESIGN_HEIGHT,
        ));

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
        self.login_dialog.x = (Coord::DESIGN_WIDTH - dialog_w) / 2.0;
        self.login_dialog.y = (Coord::DESIGN_HEIGHT - dialog_h) / 2.0;
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
            msg_box.update_positions(Coord::DESIGN_WIDTH, Coord::DESIGN_HEIGHT);
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
}
