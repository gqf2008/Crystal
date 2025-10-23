//! 登录对话框

use ggez::{Context, graphics::Canvas};
use crate::graphics::{LibraryName, draw_sprite_at};
use crate::ecs::scenes::ui::{Button, TextInput};

pub struct LoginDialog {
    // 对话框位置和尺寸
    pub x: f32,
    pub y: f32,
    pub width: f32,
    pub height: f32,
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
    pub fn new(screen_width: f32, screen_height: f32) -> Self {
        // 对话框尺寸 (C#原版)
        let width = 328.0;
        let height = 220.0;
        
        // 动态居中计算
        let x = (screen_width - width) / 2.0;
        let y = (screen_height - height) / 2.0;
        
        let mut dialog = Self {
            x, y, width, height, visible: true,
            
            // 输入框 - 使用C#原版精确位置
            account_input: TextInput::new(x + 85.0, y + 85.0, 136.0, 15),
            password_input: TextInput::new(x + 85.0, y + 108.0, 136.0, 15).password(),
            
            // 所有按钮使用Title库和正确索引
            ok_button: Button::new_with_states(
                x + 227.0, y + 81.0,
                LibraryName::Title,
                320, 321, 322  // normal, hover, pressed
            ),
            new_account_button: Button::new_with_states(
                x + 60.0, y + 163.0,
                LibraryName::Title,
                323, 324, 325
            ),
            change_password_button: Button::new_with_states(
                x + 166.0, y + 163.0,
                LibraryName::Title,
                326, 327, 328
            ),
            view_key_button: Button::new_with_states(
                x + 60.0, y + 189.0,
                LibraryName::Title,
                332, 333, 334
            ),
            exit_button: Button::new_with_states(
                x + 166.0, y + 189.0,
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
        // 更新输入框位置
        self.account_input.x = self.x + 85.0;
        self.account_input.y = self.y + 85.0;
        self.password_input.x = self.x + 85.0;
        self.password_input.y = self.y + 108.0;
        
        // 更新按钮位置
        self.ok_button.x = self.x + 227.0;
        self.ok_button.y = self.y + 81.0;
        self.new_account_button.x = self.x + 60.0;
        self.new_account_button.y = self.y + 163.0;
        self.change_password_button.x = self.x + 166.0;
        self.change_password_button.y = self.y + 163.0;
        self.view_key_button.x = self.x + 60.0;
        self.view_key_button.y = self.y + 189.0;
        self.exit_button.x = self.x + 166.0;
        self.exit_button.y = self.y + 189.0;
    }
    
    pub fn get_credentials(&self) -> Option<(String, String)> {
        if self.account_input.text.is_empty() || self.password_input.text.is_empty() { return None; }
        Some((self.account_input.text.clone(), self.password_input.text.clone()))
    }
    
    /// 构建登录的网络命令(如果凭证有效)
    pub fn build_network_command(&self) -> Option<crate::network::NetworkCommand> {
        self.get_credentials().map(|(username, password)| {
            crate::network::NetworkCommand::Login { username, password }
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
    
    pub fn draw(&self, ctx: &mut Context, canvas: &mut Canvas) -> anyhow::Result<()> {
        if !self.visible { return Ok(()); }
        
        // 1. 绘制对话框背景 (正确索引: 1084)
        draw_sprite_at(ctx, canvas, &LibraryName::Prguse, 1084, self.x, self.y)?;
        
        // 2. 绘制标签 (Title库)
        // Title标签 (Index=30) - 需要居中，暂时简化
        draw_sprite_at(ctx, canvas, &LibraryName::Title, 30, 
                      self.x + (self.width - 220.0) / 2.0, self.y + 12.0)?;
        
        // AccountID标签 (Index=31)
        draw_sprite_at(ctx, canvas, &LibraryName::Title, 31, 
                      self.x + 52.0, self.y + 83.0)?;
        
        // Password标签 (Index=32)
        draw_sprite_at(ctx, canvas, &LibraryName::Title, 32, 
                      self.x + 43.0, self.y + 105.0)?;
        
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
