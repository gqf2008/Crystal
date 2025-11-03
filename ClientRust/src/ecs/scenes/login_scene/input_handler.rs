//! LoginScene 网络事件处理模块
//!
//! 负责处理所有与服务器通信相关的事件响应

use super::LoginScene;
use crate::ecs::{Coord, GameContext};
use ggez::winit::event::MouseButton;
use ggez::winit::keyboard::KeyCode;
use ggez::{Context, GameResult};
use hecs::World;

use super::change_password::{ChangePasswordAction, ChangePasswordDialog};
use super::dialog_manager::{handle_dialog_keycode, DialogKeyResult};
use super::login::DialogAction;
use super::new_account::{NewAccountAction, NewAccountDialog};
use super::virtual_keyboard::{FocusedInput, VirtualKeyboard, VirtualKeyboardAction};
use super::SceneType;

impl LoginScene {
    fn on_mouse_move(
        &mut self,
        ctx: &mut Context,
        _world: &mut World,
        x: f32,
        y: f32,
    ) -> GameResult {
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

    fn on_mouse_down(
        &mut self,
        ctx: &mut Context,
        world: &mut World,
        _button: &MouseButton,
        x: f32,
        y: f32,
    ) -> GameResult {
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
                                self.login_dialog
                                    .account_input
                                    .add_char(ch.to_ascii_lowercase());
                            }
                            FocusedInput::Password => {
                                self.login_dialog
                                    .password_input
                                    .add_char(ch.to_ascii_lowercase());
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
                    use crate::ecs::WorldExt;
                    if let Err(e) = world.network().send(cmd) {
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
                    use crate::ecs::WorldExt;
                    if let Err(e) = world.network().send(cmd) {
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
            DialogAction::Login => self.submit_login(world),
            DialogAction::OpenNewAccount => {
                tracing::info!("🆕 打开新建账号对话框");
                let mut dialog = NewAccountDialog::new(Coord::DESIGN_WIDTH, Coord::DESIGN_HEIGHT);
                dialog.show();
                self.new_account_dialog = Some(dialog);
            }
            DialogAction::OpenChangePassword => {
                tracing::info!("🔑 打开修改密码对话框");
                // 从登录框预填充账号和密码
                let (account_id, password) = self
                    .login_dialog
                    .get_credentials()
                    .map(|(id, pwd)| (Some(id), Some(pwd)))
                    .unwrap_or((None, None));
                let mut dialog = ChangePasswordDialog::new(Coord::DESIGN_WIDTH, Coord::DESIGN_HEIGHT);
                dialog.show(account_id, password);
                self.change_password_dialog = Some(dialog);
            }
            DialogAction::OpenViewKey => {
                tracing::info!("⌨️ 打开虚拟键盘");
                let mut keyboard = VirtualKeyboard::new(Coord::DESIGN_WIDTH, Coord::DESIGN_HEIGHT);
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

    fn on_key_down(
        &mut self,
        world: &mut World,
        key: &KeyCode,
        text: Option<&str>,
    ) -> GameResult<Option<SceneType>> {
        // 虚拟键盘优先级最高(处理ESC/Backspace/Enter/Space)
        if let Some(keyboard) = &mut self.virtual_keyboard {
            match key {
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
            return Ok(None);
        }

        // 消息框优先级最高
        if self.message_box.is_some() {
            if matches!(key, KeyCode::Escape | KeyCode::Enter) {
                self.message_box = None;
            }
            return Ok(None);
        }

        // 处理修改密码对话框（优先级高）
        if let Some(dialog) = &mut self.change_password_dialog {
            let result =
                handle_dialog_keycode(dialog, key, text.as_deref(), world, "发送修改密码命令失败");
            match result {
                DialogKeyResult::Close => self.change_password_dialog = None,
                DialogKeyResult::ValidationFailed(msg) | DialogKeyResult::SendError(msg) => {
                    self.show_message(&msg);
                }
                DialogKeyResult::Handled => {}
            }
            return Ok(None);
        }

        // 处理新建账号对话框
        if let Some(dialog) = &mut self.new_account_dialog {
            let result =
                handle_dialog_keycode(dialog, key, text.as_deref(), world, "发送注册命令失败");
            match result {
                DialogKeyResult::Close => self.new_account_dialog = None,
                DialogKeyResult::ValidationFailed(msg) | DialogKeyResult::SendError(msg) => {
                    self.show_message(&msg);
                }
                DialogKeyResult::Handled => {}
            }
            return Ok(None);
        }

        // 处理登录对话框
        match key {
            KeyCode::Tab => self.login_dialog.on_tab(),
            KeyCode::Enter => {
                let action = self.login_dialog.on_enter();
                if action == DialogAction::Login {
                    self.submit_login(world);
                }
            }
            KeyCode::Backspace => self.login_dialog.on_backspace(),
            _ => {
                // 🔥 不要在这里处理 text 字段，因为 on_text_input 会处理
                // 避免重复输入
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

    fn on_text_input(&mut self, _world: &mut World, character: String) -> GameResult {
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

    /// 基于 InputContext 的输入事件处理
    /// 
    /// 使用 GameContext 提供的事件迭代器
    pub(crate) fn handle_input_event(&mut self, game_ctx: &mut GameContext) -> GameResult {
        // ⚠️ 先收集所有事件，避免借用冲突
        // 迭代器持有 game_ctx 的不可变借用，但处理函数需要可变借用 ctx/world
        
        let mouse_moves: Vec<_> = game_ctx.input().mouse_motion().collect();
        let mouse_downs: Vec<_> = if let Some((btn, x, y)) = game_ctx.input().mouse_button_pressed(MouseButton::Left) {
            vec![(btn, x, y)]
        } else {
            vec![]
        };
        let key_downs: Vec<_> = game_ctx.input().pressed_keys()
            .map(|(k, t)| (k, t.map(|s| s.to_string())))
            .collect();
        let text_inputs: Vec<_> = game_ctx.input().text_input().collect();
        
        // 1️⃣ 处理鼠标移动事件
        for (x, y, _dx, _dy) in mouse_moves {
            self.on_mouse_move(game_ctx.ctx, game_ctx.world, x, y)?;
        }
        
        // 2️⃣ 处理鼠标按下事件
        for (button, x, y) in mouse_downs {
            self.on_mouse_down(game_ctx.ctx, game_ctx.world, &button, x, y)?;
        }
        
        // 3️⃣ 处理键盘按下事件
        for (keycode, text) in key_downs {
            self.on_key_down(game_ctx.world, &keycode, text.as_deref())?;
        }
        
        // 4️⃣ 处理文本输入事件
        for character in text_inputs {
            self.on_text_input(game_ctx.world, character.to_string())?;
        }
        
        Ok(())
    }
}
