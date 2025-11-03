//! 登录对话框

use ggez::{Context, graphics::Canvas};
use crate::graphics::{LibraryName, draw_sprite_at};
use crate::ecs::scenes::ui::{Button, TextInput};

pub struct LoginDialog {
    // 对话框位置
    pub x: f32,
    pub y: f32,
    pub visible: bool,
    
    // 输入框
    pub account_input: TextInput,
    pub password_input: TextInput,
    
    // 按钮
    pub ok_button: Button,
    pub new_account_button: Button,
    pub change_password_button: Button,
    pub view_key_button: Button,
    pub exit_button: Button,
}

impl LoginDialog {
    // 相对于对话框纹理的偏移常量
    const OFFSET_ACCOUNT_INPUT_X: f32 = 85.0;
    const OFFSET_ACCOUNT_INPUT_Y: f32 = 85.0;
    const OFFSET_PASSWORD_INPUT_X: f32 = 85.0;
    const OFFSET_PASSWORD_INPUT_Y: f32 = 108.0;
    const OFFSET_OK_BUTTON_X: f32 = 227.0;
    const OFFSET_OK_BUTTON_Y: f32 = 81.0;
    const OFFSET_NEW_ACCOUNT_X: f32 = 60.0;
    const OFFSET_NEW_ACCOUNT_Y: f32 = 163.0;
    const OFFSET_CHANGE_PASSWORD_X: f32 = 166.0;
    const OFFSET_CHANGE_PASSWORD_Y: f32 = 163.0;
    const OFFSET_VIEW_KEY_X: f32 = 60.0;
    const OFFSET_VIEW_KEY_Y: f32 = 189.0;
    const OFFSET_EXIT_X: f32 = 166.0;
    const OFFSET_EXIT_Y: f32 = 189.0;
    const OFFSET_TITLE_X: f32 = 54.0;  // (328 - 220) / 2
    const OFFSET_TITLE_Y: f32 = 12.0;
    const OFFSET_ACCOUNT_LABEL_X: f32 = 52.0;
    const OFFSET_ACCOUNT_LABEL_Y: f32 = 83.0;
    const OFFSET_PASSWORD_LABEL_X: f32 = 43.0;
    const OFFSET_PASSWORD_LABEL_Y: f32 = 105.0;
    
    pub fn new(screen_width: f32, screen_height: f32) -> Self {
        // 对话框纹理尺寸 (Prguse:1084 - TODO: 从图库查询)
        let dialog_w = 328.0;
        let dialog_h = 220.0;
        
        // 在1024x768设计空间居中
        let x = (screen_width - dialog_w) / 2.0;
        let y = (screen_height - dialog_h) / 2.0;
        
        let mut dialog = Self {
            x, y, visible: true,
            
            // 输入框 - 使用相对偏移常量
            account_input: TextInput::new(
                x + Self::OFFSET_ACCOUNT_INPUT_X, 
                y + Self::OFFSET_ACCOUNT_INPUT_Y, 
                136.0, 15
            ),
            password_input: TextInput::new(
                x + Self::OFFSET_PASSWORD_INPUT_X, 
                y + Self::OFFSET_PASSWORD_INPUT_Y, 
                136.0, 15
            ).password(),
            
            // 所有按钮使用Title库和相对偏移
            ok_button: Button::new_with_states(
                x + Self::OFFSET_OK_BUTTON_X, 
                y + Self::OFFSET_OK_BUTTON_Y,
                LibraryName::Title,
                320, 321, 322
            ),
            new_account_button: Button::new_with_states(
                x + Self::OFFSET_NEW_ACCOUNT_X, 
                y + Self::OFFSET_NEW_ACCOUNT_Y,
                LibraryName::Title,
                323, 324, 325
            ),
            change_password_button: Button::new_with_states(
                x + Self::OFFSET_CHANGE_PASSWORD_X, 
                y + Self::OFFSET_CHANGE_PASSWORD_Y,
                LibraryName::Title,
                326, 327, 328
            ),
            view_key_button: Button::new_with_states(
                x + Self::OFFSET_VIEW_KEY_X, 
                y + Self::OFFSET_VIEW_KEY_Y,
                LibraryName::Title,
                332, 333, 334
            ),
            exit_button: Button::new_with_states(
                x + Self::OFFSET_EXIT_X, 
                y + Self::OFFSET_EXIT_Y,
                LibraryName::Title,
                329, 330, 331
            ),
        };
        
        // 🔥 设置初始焦点到账号输入框
        dialog.account_input.focused = true;
        
        dialog
    }
    
    /// 更新所有子组件位置（当对话框x/y改变时调用）
    pub fn update_positions(&mut self) {
        // 使用常量偏移更新位置
        self.account_input.x = self.x + Self::OFFSET_ACCOUNT_INPUT_X;
        self.account_input.y = self.y + Self::OFFSET_ACCOUNT_INPUT_Y;
        self.password_input.x = self.x + Self::OFFSET_PASSWORD_INPUT_X;
        self.password_input.y = self.y + Self::OFFSET_PASSWORD_INPUT_Y;
        
        self.ok_button.x = self.x + Self::OFFSET_OK_BUTTON_X;
        self.ok_button.y = self.y + Self::OFFSET_OK_BUTTON_Y;
        self.new_account_button.x = self.x + Self::OFFSET_NEW_ACCOUNT_X;
        self.new_account_button.y = self.y + Self::OFFSET_NEW_ACCOUNT_Y;
        self.change_password_button.x = self.x + Self::OFFSET_CHANGE_PASSWORD_X;
        self.change_password_button.y = self.y + Self::OFFSET_CHANGE_PASSWORD_Y;
        self.view_key_button.x = self.x + Self::OFFSET_VIEW_KEY_X;
        self.view_key_button.y = self.y + Self::OFFSET_VIEW_KEY_Y;
        self.exit_button.x = self.x + Self::OFFSET_EXIT_X;
        self.exit_button.y = self.y + Self::OFFSET_EXIT_Y;
    }
    
    pub fn get_credentials(&self) -> Option<(String, String)> {
        if self.account_input.text.is_empty() || self.password_input.text.is_empty() { return None; }
        Some((self.account_input.text.clone(), self.password_input.text.clone()))
    }
    
    /// 构建登录的网络命令(如果凭证有效)
    pub fn build_network_command(&self) -> Option<crate::network::handlers::GameEvent> {
        self.get_credentials().map(|(username, password)| {
            crate::network::handlers::GameEvent::LoginRequest { username, password }
        })
    }
    
    pub fn clear(&mut self) { 
        self.account_input.clear(); 
        self.password_input.clear(); 
    }
    
    pub fn update(&mut self, dt: f32) { 
        self.account_input.update(dt); 
        self.password_input.update(dt); 
    }
    
    pub fn on_mouse_move(&mut self, x: f32, y: f32) {
        self.ok_button.update_hover(x, y);
        self.new_account_button.update_hover(x, y);
        self.change_password_button.update_hover(x, y);
        self.view_key_button.update_hover(x, y);
        self.exit_button.update_hover(x, y);
    }
    
    pub fn on_mouse_down(&mut self, x: f32, y: f32) -> DialogAction {
        self.account_input.focused = self.account_input.contains(x, y);
        self.password_input.focused = self.password_input.contains(x, y);
        
        if self.ok_button.contains(x, y) { return DialogAction::Login; }
        if self.new_account_button.contains(x, y) { return DialogAction::OpenNewAccount; }
        if self.change_password_button.contains(x, y) { return DialogAction::OpenChangePassword; }
        if self.view_key_button.contains(x, y) { return DialogAction::OpenViewKey; }
        if self.exit_button.contains(x, y) { return DialogAction::Exit; }
        
        DialogAction::None
    }
    pub fn on_char(&mut self, c: char) {
        if self.account_input.focused { self.account_input.add_char(c); }
        else if self.password_input.focused { self.password_input.add_char(c); }
    }
    pub fn on_backspace(&mut self) {
        if self.account_input.focused { self.account_input.backspace(); }
        else if self.password_input.focused { self.password_input.backspace(); }
    }
    pub fn on_tab(&mut self) {
        if self.account_input.focused {
            self.account_input.focused = false; self.password_input.focused = true;
        } else if self.password_input.focused {
            self.password_input.focused = false; self.account_input.focused = true;
        } else {
            self.account_input.focused = true;
        }
    }
    pub fn on_enter(&mut self) -> DialogAction {
        if self.account_input.focused || self.password_input.focused { DialogAction::Login } else { DialogAction::None }
    }
    
    /// 处理 IME 输入 (中文输入)
    pub fn on_text_input(&mut self, text: &str) {
        if self.account_input.focused {
            self.account_input.add_text(text);
        } else if self.password_input.focused {
            self.password_input.add_text(text);
        }
    }
    
    pub fn draw(&self, ctx: &mut ggez::graphics::GraphicsContext, canvas: &mut Canvas) -> anyhow::Result<()> {
        if !self.visible { return Ok(()); }
        
        // 1. 绘制对话框背景 (Prguse:1084 - 328x220纹理)
        draw_sprite_at(ctx, canvas, &LibraryName::Prguse, 1084, self.x, self.y)?;
        
        // 2. 绘制标签 (使用相对偏移常量)
        draw_sprite_at(ctx, canvas, &LibraryName::Title, 30, 
                      self.x + Self::OFFSET_TITLE_X, self.y + Self::OFFSET_TITLE_Y)?;
        
        draw_sprite_at(ctx, canvas, &LibraryName::Title, 31, 
                      self.x + Self::OFFSET_ACCOUNT_LABEL_X, self.y + Self::OFFSET_ACCOUNT_LABEL_Y)?;
        
        draw_sprite_at(ctx, canvas, &LibraryName::Title, 32, 
                      self.x + Self::OFFSET_PASSWORD_LABEL_X, self.y + Self::OFFSET_PASSWORD_LABEL_Y)?;
        
       
        
        // 3. 绘制输入框
        self.account_input.draw(ctx, canvas)?;
        self.password_input.draw(ctx, canvas)?;
    
        // 4. 绘制按钮
        self.ok_button.draw(ctx, canvas)?;
        self.new_account_button.draw(ctx, canvas)?;
        self.change_password_button.draw(ctx, canvas)?;
        self.view_key_button.draw(ctx, canvas)?;
        self.exit_button.draw(ctx, canvas)?;
        
        Ok(())
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DialogAction { None, Login, OpenNewAccount, OpenChangePassword, OpenViewKey, Exit }

