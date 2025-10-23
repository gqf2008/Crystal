//! 修改密码对话框
//! Mirrors Client/MirScenes/LoginScene.cs::ChangePasswordDialog

use regex::Regex;
use ggez::{Context, graphics::Canvas};
use crate::graphics::{LibraryName, draw_sprite_at};
use crate::ecs::scenes::ui::{Button, TextInput};
use super::dialog_manager::DialogWithValidation;

/// 修改密码结果
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ChangePasswordResult {
    Disabled = 0,
    InvalidAccountID = 1,
    InvalidCurrentPassword = 2,
    InvalidNewPassword = 3,
    AccountNotFound = 4,
    IncorrectPassword = 5,
    Success = 6,
}

impl ChangePasswordResult {
    pub fn from_u8(value: u8) -> Option<Self> {
        match value {
            0 => Some(Self::Disabled),
            1 => Some(Self::InvalidAccountID),
            2 => Some(Self::InvalidCurrentPassword),
            3 => Some(Self::InvalidNewPassword),
            4 => Some(Self::AccountNotFound),
            5 => Some(Self::IncorrectPassword),
            6 => Some(Self::Success),
            _ => None,
        }
    }

    pub fn message(&self) -> &'static str {
        match self {
            Self::Disabled => "Password changing is currently disabled.",
            Self::InvalidAccountID => "Your AccountID is not acceptable.",
            Self::InvalidCurrentPassword => "The current Password is not acceptable.",
            Self::InvalidNewPassword => "Your new Password is not acceptable.",
            Self::AccountNotFound => "No account with that ID exists.",
            Self::IncorrectPassword => "Incorrect password for that account ID.",
            Self::Success => "Your password was changed successfully.",
        }
    }
}

/// 输入字段枚举
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PasswordInputField {
    AccountId,
    CurrentPassword,
    NewPassword,
    NewPasswordConfirm,
}

/// 修改密码对话框
pub struct ChangePasswordDialog {
    pub x: f32, pub y: f32, pub visible: bool,
    pub account_id: String,
    pub current_password: String,
    pub new_password: String,
    pub new_password_confirm: String,
    pub focused_field: PasswordInputField,
    
    // 输入框
    account_input: TextInput,
    current_password_input: TextInput,
    new_password_input: TextInput,
    confirm_input: TextInput,
    
    // 按钮
    ok_button: Button,
    cancel_button: Button,
    
    // 验证状态
    pub account_valid: bool,
    pub current_valid: bool,
    pub new_password_valid: bool,
    pub confirm_valid: bool,
    
    min_account_len: usize,
    max_account_len: usize,
    min_password_len: usize,
    max_password_len: usize,
}

impl ChangePasswordDialog {
    // 相对于对话框纹理的偏移常量
    const OFFSET_ACCOUNT_X: f32 = 178.0;
    const OFFSET_ACCOUNT_Y: f32 = 75.0;
    const OFFSET_CURRENT_PASSWORD_X: f32 = 178.0;
    const OFFSET_CURRENT_PASSWORD_Y: f32 = 113.0;
    const OFFSET_NEW_PASSWORD_X: f32 = 178.0;
    const OFFSET_NEW_PASSWORD_Y: f32 = 151.0;
    const OFFSET_CONFIRM_X: f32 = 178.0;
    const OFFSET_CONFIRM_Y: f32 = 188.0;
    const OFFSET_OK_BUTTON_X: f32 = 80.0;
    const OFFSET_OK_BUTTON_Y: f32 = 236.0;
    const OFFSET_CANCEL_BUTTON_X: f32 = 222.0;
    const OFFSET_CANCEL_BUTTON_Y: f32 = 236.0;
    
    pub fn new(screen_w: f32, screen_h: f32) -> Self {
        // 对话框纹理尺寸 (Prguse:2 - TODO: 从图库查询)
        let dialog_w = 360.0;
        let dialog_h = 280.0;
        
        // 在1024x768设计空间居中
        let x = (screen_w - dialog_w) / 2.0;
        let y = (screen_h - dialog_h) / 2.0;
        
        Self {
            x, y, visible: false,
            account_id: String::new(),
            current_password: String::new(),
            new_password: String::new(),
            new_password_confirm: String::new(),
            focused_field: PasswordInputField::AccountId,
            
            // 使用常量偏移创建UI元素
            account_input: TextInput::new(x + Self::OFFSET_ACCOUNT_X, y + Self::OFFSET_ACCOUNT_Y, 136.0, 20),
            current_password_input: TextInput::new(x + Self::OFFSET_CURRENT_PASSWORD_X, y + Self::OFFSET_CURRENT_PASSWORD_Y, 136.0, 20).password(),
            new_password_input: TextInput::new(x + Self::OFFSET_NEW_PASSWORD_X, y + Self::OFFSET_NEW_PASSWORD_Y, 136.0, 20).password(),
            confirm_input: TextInput::new(x + Self::OFFSET_CONFIRM_X, y + Self::OFFSET_CONFIRM_Y, 136.0, 20).password(),
            
            ok_button: Button::new_with_states(x + Self::OFFSET_OK_BUTTON_X, y + Self::OFFSET_OK_BUTTON_Y, LibraryName::Title, 107, 108, 109),
            cancel_button: Button::new_with_states(x + Self::OFFSET_CANCEL_BUTTON_X, y + Self::OFFSET_CANCEL_BUTTON_Y, LibraryName::Title, 110, 111, 112),
            
            account_valid: false,
            current_valid: false,
            new_password_valid: false,
            confirm_valid: false,
            
            min_account_len: 3,
            max_account_len: 20,
            min_password_len: 3,
            max_password_len: 20,
        }
    }
    
    /// 更新所有子组件位置（当对话框x/y改变时调用）
    pub fn update_positions(&mut self) {
        // 使用常量偏移更新位置
        self.account_input.x = self.x + Self::OFFSET_ACCOUNT_X;
        self.account_input.y = self.y + Self::OFFSET_ACCOUNT_Y;
        
        self.current_password_input.x = self.x + Self::OFFSET_CURRENT_PASSWORD_X;
        self.current_password_input.y = self.y + Self::OFFSET_CURRENT_PASSWORD_Y;
        
        self.new_password_input.x = self.x + Self::OFFSET_NEW_PASSWORD_X;
        self.new_password_input.y = self.y + Self::OFFSET_NEW_PASSWORD_Y;
        
        self.confirm_input.x = self.x + Self::OFFSET_CONFIRM_X;
        self.confirm_input.y = self.y + Self::OFFSET_CONFIRM_Y;
        
        self.ok_button.x = self.x + Self::OFFSET_OK_BUTTON_X;
        self.ok_button.y = self.y + Self::OFFSET_OK_BUTTON_Y;
        
        self.cancel_button.x = self.x + Self::OFFSET_CANCEL_BUTTON_X;
        self.cancel_button.y = self.y + Self::OFFSET_CANCEL_BUTTON_Y;
    }
    
    pub fn show(&mut self, autofill_id: Option<String>, autofill_password: Option<String>) {
        self.visible = true;
        if let Some(id) = autofill_id {
            self.account_input.text = id.clone();
            self.account_id = id;
            self.validate_account();
        }
        if let Some(password) = autofill_password {
            self.current_password_input.text = password.clone();
            self.current_password = password;
            self.validate_current();
        }
        self.account_input.focused = true;
    }
    
    pub fn hide(&mut self) {
        self.visible = false;
    }
    
    pub fn update(&mut self, dt: f32) {
        self.account_input.update(dt);
        self.current_password_input.update(dt);
        self.new_password_input.update(dt);
        self.confirm_input.update(dt);
    }
    
    pub fn on_tab(&mut self) {
        self.focused_field = match self.focused_field {
            PasswordInputField::AccountId => PasswordInputField::CurrentPassword,
            PasswordInputField::CurrentPassword => PasswordInputField::NewPassword,
            PasswordInputField::NewPassword => PasswordInputField::NewPasswordConfirm,
            PasswordInputField::NewPasswordConfirm => PasswordInputField::AccountId,
        };
        self.update_focus();
    }
    
    fn update_focus(&mut self) {
        self.account_input.focused = self.focused_field == PasswordInputField::AccountId;
        self.current_password_input.focused = self.focused_field == PasswordInputField::CurrentPassword;
        self.new_password_input.focused = self.focused_field == PasswordInputField::NewPassword;
        self.confirm_input.focused = self.focused_field == PasswordInputField::NewPasswordConfirm;
    }
    
    pub fn on_char(&mut self, ch: char) {
        if !ch.is_ascii_alphanumeric() { return; }
        
        match self.focused_field {
            PasswordInputField::AccountId => {
                self.account_input.add_char(ch);
                self.account_id = self.account_input.text.clone();
                self.validate_account();
            }
            PasswordInputField::CurrentPassword => {
                self.current_password_input.add_char(ch);
                self.current_password = self.current_password_input.text.clone();
                self.validate_current();
            }
            PasswordInputField::NewPassword => {
                self.new_password_input.add_char(ch);
                self.new_password = self.new_password_input.text.clone();
                self.validate_new_password();
            }
            PasswordInputField::NewPasswordConfirm => {
                self.confirm_input.add_char(ch);
                self.new_password_confirm = self.confirm_input.text.clone();
                self.validate_confirm();
            }
        }
    }
    
    /// 处理 IME 输入 (中文输入)
    pub fn on_text_input(&mut self, text: &str) {
        match self.focused_field {
            PasswordInputField::AccountId => {
                self.account_input.add_text(text);
                self.account_id = self.account_input.text.clone();
                self.validate_account();
            }
            PasswordInputField::CurrentPassword => {
                self.current_password_input.add_text(text);
                self.current_password = self.current_password_input.text.clone();
                self.validate_current();
            }
            PasswordInputField::NewPassword => {
                self.new_password_input.add_text(text);
                self.new_password = self.new_password_input.text.clone();
                self.validate_new_password();
            }
            PasswordInputField::NewPasswordConfirm => {
                self.confirm_input.add_text(text);
                self.new_password_confirm = self.confirm_input.text.clone();
                self.validate_confirm();
            }
        }
    }
    
    pub fn on_backspace(&mut self) {
        match self.focused_field {
            PasswordInputField::AccountId => {
                self.account_input.backspace();
                self.account_id = self.account_input.text.clone();
                self.validate_account();
            }
            PasswordInputField::CurrentPassword => {
                self.current_password_input.backspace();
                self.current_password = self.current_password_input.text.clone();
                self.validate_current();
            }
            PasswordInputField::NewPassword => {
                self.new_password_input.backspace();
                self.new_password = self.new_password_input.text.clone();
                self.validate_new_password();
            }
            PasswordInputField::NewPasswordConfirm => {
                self.confirm_input.backspace();
                self.new_password_confirm = self.confirm_input.text.clone();
                self.validate_confirm();
            }
        }
    }
    
    fn validate_account(&mut self) {
        let len = self.account_id.chars().count();
        let pattern = format!(r"^[A-Za-z0-9]{{{},{}}}$", self.min_account_len, self.max_account_len);
        self.account_valid = if let Ok(regex) = Regex::new(&pattern) {
            regex.is_match(&self.account_id) && len >= self.min_account_len && len <= self.max_account_len
        } else {
            false
        };
    }
    
    fn validate_current(&mut self) {
        let len = self.current_password.chars().count();
        self.current_valid = len >= self.min_password_len && len <= self.max_password_len;
    }
    
    fn validate_new_password(&mut self) {
        let len = self.new_password.chars().count();
        let pattern = format!(r"^[A-Za-z0-9]{{{},{}}}$", self.min_password_len, self.max_password_len);
        self.new_password_valid = if let Ok(regex) = Regex::new(&pattern) {
            regex.is_match(&self.new_password) && len >= self.min_password_len && len <= self.max_password_len
        } else {
            false
        };
        self.validate_confirm(); // 重新验证确认密码
    }
    
    fn validate_confirm(&mut self) {
        self.confirm_valid = !self.new_password_confirm.is_empty() 
            && self.new_password == self.new_password_confirm;
    }
    
    pub fn can_submit(&self) -> bool {
        self.account_valid && self.current_valid && self.new_password_valid && self.confirm_valid
    }
    
    /// 获取验证错误消息
    pub fn get_validation_error(&self) -> String {
        if !self.account_valid {
            if self.account_id.is_empty() {
                return "Please enter an Account ID.".to_string();
            }
            return format!("Account ID must be {}-{} alphanumeric characters.", 
                self.min_account_len, self.max_account_len);
        }
        if !self.current_valid {
            if self.current_password.is_empty() {
                return "Please enter your current Password.".to_string();
            }
            return format!("Current Password must be {}-{} characters.", 
                self.min_password_len, self.max_password_len);
        }
        if !self.new_password_valid {
            if self.new_password.is_empty() {
                return "Please enter a new Password.".to_string();
            }
            return format!("New Password must be {}-{} alphanumeric characters.", 
                self.min_password_len, self.max_password_len);
        }
        if !self.confirm_valid {
            if self.new_password_confirm.is_empty() {
                return "Please confirm your new Password.".to_string();
            }
            return "New Passwords do not match.".to_string();
        }
        "Unknown validation error.".to_string()
    }
    
    /// 构建网络命令
    pub fn build_network_command(&self) -> crate::network::NetworkCommand {
        crate::network::NetworkCommand::ChangePassword {
            account_id: self.account_id.clone(),
            current_password: self.current_password.clone(),
            new_password: self.new_password.clone(),
        }
    }
    
    pub fn on_mouse_move(&mut self, x: f32, y: f32) {
        self.ok_button.update_hover(x, y);
        self.cancel_button.update_hover(x, y);
    }
    
    pub fn on_mouse_down(&mut self, x: f32, y: f32) -> ChangePasswordAction {
        if self.ok_button.contains(x, y) {
            if self.can_submit() {
                return ChangePasswordAction::Submit;
            } else {
                return ChangePasswordAction::ValidationFailed(self.get_validation_error());
            }
        }
        if self.cancel_button.contains(x, y) {
            return ChangePasswordAction::Cancel;
        }
        
        // 检测点击了哪个输入框 - 根据C#位置调整
        // AccountID: y=75, height=18
        if y >= self.y + 75.0 && y < self.y + 93.0 {
            self.focused_field = PasswordInputField::AccountId;
        // Current Password: y=113
        } else if y >= self.y + 113.0 && y < self.y + 131.0 {
            self.focused_field = PasswordInputField::CurrentPassword;
        // New Password 1: y=151
        } else if y >= self.y + 151.0 && y < self.y + 169.0 {
            self.focused_field = PasswordInputField::NewPassword;
        // New Password 2: y=188
        } else if y >= self.y + 188.0 && y < self.y + 206.0 {
            self.focused_field = PasswordInputField::NewPasswordConfirm;
        }
        self.update_focus();
        
        ChangePasswordAction::None
    }
    
    pub fn draw(&self, ctx: &mut Context, canvas: &mut Canvas) -> anyhow::Result<()> {
        if !self.visible { return Ok(()); }
        
        // 绘制对话框背景 - C#: Index = 50, Library = Prguse
        draw_sprite_at(ctx, canvas, &LibraryName::Prguse, 50, self.x, self.y)?;
        
        // 绘制所有输入框
        self.account_input.draw(ctx, canvas)?;
        self.current_password_input.draw(ctx, canvas)?;
        self.new_password_input.draw(ctx, canvas)?;
        self.confirm_input.draw(ctx, canvas)?;
        
        // 绘制按钮
        self.ok_button.draw(ctx, canvas)?;
        self.cancel_button.draw(ctx, canvas)?;
        
        Ok(())
    }
}

impl DialogWithValidation for ChangePasswordDialog {
    fn on_tab(&mut self) {
        self.on_tab();
    }
    
    fn on_backspace(&mut self) {
        self.on_backspace();
    }
    
    fn on_char(&mut self, ch: char) {
        self.on_char(ch);
    }
    
    fn can_submit(&self) -> bool {
        self.can_submit()
    }
    
    fn get_validation_error(&self) -> String {
        self.get_validation_error()
    }
    
    fn build_network_command(&self) -> crate::network::NetworkCommand {
        self.build_network_command()
    }
}

#[derive(Debug, PartialEq)]
pub enum ChangePasswordAction {
    None,
    Submit,
    Cancel,
    ValidationFailed(String),
}
