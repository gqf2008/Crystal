//! 新建账号对话框
//! Mirrors Client/MirScenes/LoginScene.cs::NewAccountDialog

use regex::Regex;
use ggez::{Context, graphics::Canvas};
use crate::graphics::{LibraryName, draw_sprite_at};
use crate::ecs::scenes::ui::{Button, TextInput};
use super::dialog_manager::DialogWithValidation;

/// 新建账号结果
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum NewAccountResult {
    Disabled = 0,
    InvalidAccountID = 1,
    InvalidPassword = 2,
    InvalidEmail = 3,
    InvalidUsername = 4,
    InvalidQuestion = 5,
    InvalidAnswer = 6,
    AccountExists = 7,
    Success = 8,
}

impl NewAccountResult {
    pub fn from_u8(value: u8) -> Option<Self> {
        match value {
            0 => Some(Self::Disabled),
            1 => Some(Self::InvalidAccountID),
            2 => Some(Self::InvalidPassword),
            3 => Some(Self::InvalidEmail),
            4 => Some(Self::InvalidUsername),
            5 => Some(Self::InvalidQuestion),
            6 => Some(Self::InvalidAnswer),
            7 => Some(Self::AccountExists),
            8 => Some(Self::Success),
            _ => None,
        }
    }

    pub fn message(&self) -> &'static str {
        match self {
            Self::Disabled => "Account creation is currently disabled.",
            Self::InvalidAccountID => "Your AccountID is not acceptable.",
            Self::InvalidPassword => "Your Password is not acceptable.",
            Self::InvalidEmail => "Your E-Mail Address is not acceptable.",
            Self::InvalidUsername => "Your User Name is not acceptable.",
            Self::InvalidQuestion => "Your Secret Question is not acceptable.",
            Self::InvalidAnswer => "Your Secret Answer is not acceptable.",
            Self::AccountExists => "An Account with this ID already exists.",
            Self::Success => "Your account was created successfully.",
        }
    }
}

/// 注册数据
#[derive(Debug, Clone, Default)]
pub struct AccountRegistration {
    pub account_id: String,
    pub password: String,
    pub password_confirm: String,
    pub email: String,
    pub username: String,
    pub birth_date: String,
    pub secret_question: String,
    pub secret_answer: String,
}

/// 输入字段枚举
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum InputField {
    AccountId,
    Password,
    PasswordConfirm,
    Email,
    Username,
    BirthDate,
    Question,
    Answer,
}

/// 新建账号对话框
pub struct NewAccountDialog {
    pub x: f32, pub y: f32, pub visible: bool,
    pub registration: AccountRegistration,
    pub focused_field: InputField,
    
    // 输入框
    account_input: TextInput,
    password_input: TextInput,
    confirm_input: TextInput,
    email_input: TextInput,
    username_input: TextInput,
    birthdate_input: TextInput,
    question_input: TextInput,
    answer_input: TextInput,
    
    // 按钮
    ok_button: Button,
    cancel_button: Button,
    
    // 验证状态
    pub account_valid: bool,
    pub password_valid: bool,
    pub confirm_valid: bool,
    pub email_valid: bool,
    
    min_account_len: usize,
    max_account_len: usize,
    min_password_len: usize,
    max_password_len: usize,
}

impl NewAccountDialog {
    pub fn new(screen_w: f32, screen_h: f32) -> Self {
        // C#原版: Index 63, Prguse库, 需要居中
        // 对话框尺寸从纹理资源获取,暂用估算值 520x470
        let dialog_w = 520.0;
        let dialog_h = 470.0;
        let x = (screen_w - dialog_w) / 2.0;
        let y = (screen_h - dialog_h) / 2.0;
        
        Self {
            x, y, visible: false,
            registration: AccountRegistration::default(),
            focused_field: InputField::AccountId,
            
            // C#原版输入框位置(相对对话框): AccountID(226,103), Password1(226,129), etc.
            account_input: TextInput::new(x + 226.0, y + 103.0, 136.0, 15),    // MaxLength对应Globals.MaxAccountIDLength
            password_input: TextInput::new(x + 226.0, y + 129.0, 136.0, 20).password(),
            confirm_input: TextInput::new(x + 226.0, y + 155.0, 136.0, 20).password(),
            username_input: TextInput::new(x + 226.0, y + 189.0, 136.0, 20),
            birthdate_input: TextInput::new(x + 226.0, y + 215.0, 136.0, 10),
            question_input: TextInput::new(x + 226.0, y + 250.0, 190.0, 30),
            answer_input: TextInput::new(x + 226.0, y + 276.0, 190.0, 30),
            email_input: TextInput::new(x + 226.0, y + 311.0, 136.0, 50),
            
            // C#原版按钮: OK(135,425), Cancel(409,425)
            ok_button: Button::new_with_states(x + 135.0, y + 425.0, LibraryName::Title, 200, 201, 202),
            cancel_button: Button::new_with_states(x + 409.0, y + 425.0, LibraryName::Title, 203, 204, 205),
            
            account_valid: false,
            password_valid: false,
            confirm_valid: false,
            email_valid: true, // 可选字段
            
            // Globals常量值(参考C#)
            min_account_len: 3,   // Globals.MinAccountIDLength
            max_account_len: 15,  // Globals.MaxAccountIDLength
            min_password_len: 5,  // Globals.MinPasswordLength
            max_password_len: 20, // Globals.MaxPasswordLength
        }
    }
    
    pub fn update_positions(&mut self, screen_w: f32, screen_h: f32) {
        // 动态调整位置保持居中
        let dialog_w = 520.0;
        let dialog_h = 470.0;
        self.x = (screen_w - dialog_w) / 2.0;
        self.y = (screen_h - dialog_h) / 2.0;
        
        // 更新所有子组件位置
        self.account_input.x = self.x + 226.0;
        self.account_input.y = self.y + 103.0;
        
        self.password_input.x = self.x + 226.0;
        self.password_input.y = self.y + 129.0;
        
        self.confirm_input.x = self.x + 226.0;
        self.confirm_input.y = self.y + 155.0;
        
        self.username_input.x = self.x + 226.0;
        self.username_input.y = self.y + 189.0;
        
        self.birthdate_input.x = self.x + 226.0;
        self.birthdate_input.y = self.y + 215.0;
        
        self.question_input.x = self.x + 226.0;
        self.question_input.y = self.y + 250.0;
        
        self.answer_input.x = self.x + 226.0;
        self.answer_input.y = self.y + 276.0;
        
        self.email_input.x = self.x + 226.0;
        self.email_input.y = self.y + 311.0;
        
        self.ok_button.x = self.x + 135.0;
        self.ok_button.y = self.y + 425.0;
        
        self.cancel_button.x = self.x + 409.0;
        self.cancel_button.y = self.y + 425.0;
    }
    
    pub fn show(&mut self) {
        self.visible = true;
        self.account_input.focused = true;
    }
    
    pub fn hide(&mut self) {
        self.visible = false;
    }
    
    pub fn update(&mut self, dt: f32) {
        self.account_input.update(dt);
        self.password_input.update(dt);
        self.confirm_input.update(dt);
        self.email_input.update(dt);
        self.username_input.update(dt);
        self.birthdate_input.update(dt);
        self.question_input.update(dt);
        self.answer_input.update(dt);
    }
    
    pub fn on_tab(&mut self) {
        // 切换焦点
        self.focused_field = match self.focused_field {
            InputField::AccountId => InputField::Password,
            InputField::Password => InputField::PasswordConfirm,
            InputField::PasswordConfirm => InputField::Email,
            InputField::Email => InputField::Username,
            InputField::Username => InputField::BirthDate,
            InputField::BirthDate => InputField::Question,
            InputField::Question => InputField::Answer,
            InputField::Answer => InputField::AccountId,
        };
        self.update_focus();
    }
    
    fn update_focus(&mut self) {
        self.account_input.focused = self.focused_field == InputField::AccountId;
        self.password_input.focused = self.focused_field == InputField::Password;
        self.confirm_input.focused = self.focused_field == InputField::PasswordConfirm;
        self.email_input.focused = self.focused_field == InputField::Email;
        self.username_input.focused = self.focused_field == InputField::Username;
        self.birthdate_input.focused = self.focused_field == InputField::BirthDate;
        self.question_input.focused = self.focused_field == InputField::Question;
        self.answer_input.focused = self.focused_field == InputField::Answer;
    }
    
    pub fn on_char(&mut self, ch: char) {
        match self.focused_field {
            InputField::AccountId => {
                if ch.is_ascii_alphanumeric() {
                    self.account_input.add_char(ch);
                    self.registration.account_id = self.account_input.text.clone();
                    self.validate_account();
                }
            }
            InputField::Password => {
                if ch.is_ascii_alphanumeric() {
                    self.password_input.add_char(ch);
                    self.registration.password = self.password_input.text.clone();
                    self.validate_password();
                }
            }
            InputField::PasswordConfirm => {
                if ch.is_ascii_alphanumeric() {
                    self.confirm_input.add_char(ch);
                    self.registration.password_confirm = self.confirm_input.text.clone();
                    self.validate_confirm();
                }
            }
            InputField::Email => {
                if ch.is_ascii_alphanumeric() || matches!(ch, '@' | '.' | '_' | '+' | '-') {
                    self.email_input.add_char(ch);
                    self.registration.email = self.email_input.text.clone();
                    self.validate_email();
                }
            }
            InputField::Username => {
                self.username_input.add_char(ch);
                self.registration.username = self.username_input.text.clone();
            }
            InputField::BirthDate => {
                if ch.is_ascii_digit() || matches!(ch, '/' | '-') {
                    self.birthdate_input.add_char(ch);
                    self.registration.birth_date = self.birthdate_input.text.clone();
                }
            }
            InputField::Question => {
                self.question_input.add_char(ch);
                self.registration.secret_question = self.question_input.text.clone();
            }
            InputField::Answer => {
                self.answer_input.add_char(ch);
                self.registration.secret_answer = self.answer_input.text.clone();
            }
        }
    }
    
    pub fn on_backspace(&mut self) {
        match self.focused_field {
            InputField::AccountId => {
                self.account_input.backspace();
                self.registration.account_id = self.account_input.text.clone();
                self.validate_account();
            }
            InputField::Password => {
                self.password_input.backspace();
                self.registration.password = self.password_input.text.clone();
                self.validate_password();
            }
            InputField::PasswordConfirm => {
                self.confirm_input.backspace();
                self.registration.password_confirm = self.confirm_input.text.clone();
                self.validate_confirm();
            }
            InputField::Email => {
                self.email_input.backspace();
                self.registration.email = self.email_input.text.clone();
                self.validate_email();
            }
            InputField::Username => {
                self.username_input.backspace();
                self.registration.username = self.username_input.text.clone();
            }
            InputField::BirthDate => {
                self.birthdate_input.backspace();
                self.registration.birth_date = self.birthdate_input.text.clone();
            }
            InputField::Question => {
                self.question_input.backspace();
                self.registration.secret_question = self.question_input.text.clone();
            }
            InputField::Answer => {
                self.answer_input.backspace();
                self.registration.secret_answer = self.answer_input.text.clone();
            }
        }
    }
    
    fn validate_account(&mut self) {
        let len = self.registration.account_id.chars().count();
        let pattern = format!(r"^[A-Za-z0-9]{{{},{}}}$", self.min_account_len, self.max_account_len);
        self.account_valid = if let Ok(regex) = Regex::new(&pattern) {
            regex.is_match(&self.registration.account_id) && len >= self.min_account_len && len <= self.max_account_len
        } else {
            false
        };
    }
    
    fn validate_password(&mut self) {
        let len = self.registration.password.chars().count();
        let pattern = format!(r"^[A-Za-z0-9]{{{},{}}}$", self.min_password_len, self.max_password_len);
        self.password_valid = if let Ok(regex) = Regex::new(&pattern) {
            regex.is_match(&self.registration.password) && len >= self.min_password_len && len <= self.max_password_len
        } else {
            false
        };
        self.validate_confirm(); // 重新验证确认密码
    }
    
    fn validate_confirm(&mut self) {
        self.confirm_valid = !self.registration.password_confirm.is_empty() 
            && self.registration.password == self.registration.password_confirm;
    }
    
    fn validate_email(&mut self) {
        if self.registration.email.is_empty() {
            self.email_valid = true; // 可选字段
        } else {
            let pattern = r"^[a-zA-Z0-9._%+-]+@[a-zA-Z0-9.-]+\.[a-zA-Z]{2,}$";
            self.email_valid = if let Ok(regex) = Regex::new(pattern) {
                regex.is_match(&self.registration.email)
            } else {
                false
            };
        }
    }
    
    pub fn can_submit(&self) -> bool {
        self.account_valid && self.password_valid && self.confirm_valid && self.email_valid
    }
    
    /// 获取验证错误消息
    pub fn get_validation_error(&self) -> String {
        if !self.account_valid {
            if self.registration.account_id.is_empty() {
                return "Please enter an Account ID.".to_string();
            } else {
                return "Account ID must be 3-15 alphanumeric characters.".to_string();
            }
        }
        if !self.password_valid {
            if self.registration.password.is_empty() {
                return "Please enter a Password.".to_string();
            } else {
                return "Password must be 5-20 alphanumeric characters.".to_string();
            }
        }
        if !self.confirm_valid {
            if self.registration.password_confirm.is_empty() {
                return "Please confirm your Password.".to_string();
            } else {
                return "Passwords do not match.".to_string();
            }
        }
        if !self.email_valid && !self.registration.email.is_empty() {
            return "Invalid email format.".to_string();
        }
        "Please complete all required fields.".to_string()
    }
    
    pub fn on_mouse_move(&mut self, x: f32, y: f32) {
        self.ok_button.update_hover(x, y);
        self.cancel_button.update_hover(x, y);
    }
    
    pub fn on_mouse_down(&mut self, x: f32, y: f32) -> NewAccountAction {
        if self.ok_button.contains(x, y) {
            if self.can_submit() {
                return NewAccountAction::Submit;
            } else {
                // 验证失败,返回具体的错误消息
                let error_msg = self.get_validation_error();
                tracing::info!("❌ 表单验证失败: {}", error_msg);
                return NewAccountAction::ValidationFailed(error_msg);
            }
        }
        if self.cancel_button.contains(x, y) {
            return NewAccountAction::Cancel;
        }
        
        // 检测点击了哪个输入框 - 使用C#原版布局
        let rel_y = y - self.y;
        if rel_y >= 103.0 && rel_y < 129.0 {
            self.focused_field = InputField::AccountId;
        } else if rel_y >= 129.0 && rel_y < 155.0 {
            self.focused_field = InputField::Password;
        } else if rel_y >= 155.0 && rel_y < 189.0 {
            self.focused_field = InputField::PasswordConfirm;
        } else if rel_y >= 189.0 && rel_y < 215.0 {
            self.focused_field = InputField::Username;
        } else if rel_y >= 215.0 && rel_y < 250.0 {
            self.focused_field = InputField::BirthDate;
        } else if rel_y >= 250.0 && rel_y < 276.0 {
            self.focused_field = InputField::Question;
        } else if rel_y >= 276.0 && rel_y < 311.0 {
            self.focused_field = InputField::Answer;
        } else if rel_y >= 311.0 && rel_y < 340.0 {
            self.focused_field = InputField::Email;
        }
        self.update_focus();
        
        NewAccountAction::None
    }
    
    pub fn draw(&self, ctx: &mut Context, canvas: &mut Canvas) -> anyhow::Result<()> {
        if !self.visible { return Ok(()); }
        
        // 绘制对话框背景 - C#原版: Index=63, Library=Prguse
        draw_sprite_at(ctx, canvas, &LibraryName::Prguse, 63, self.x, self.y)?;
        
        // 绘制所有输入框(按C#原版顺序)
        self.account_input.draw(ctx, canvas)?;
        self.password_input.draw(ctx, canvas)?;
        self.confirm_input.draw(ctx, canvas)?;
        self.username_input.draw(ctx, canvas)?;
        self.birthdate_input.draw(ctx, canvas)?;
        self.question_input.draw(ctx, canvas)?;
        self.answer_input.draw(ctx, canvas)?;
        self.email_input.draw(ctx, canvas)?;
        
        // 绘制按钮
        self.ok_button.draw(ctx, canvas)?;
        self.cancel_button.draw(ctx, canvas)?;
        
        Ok(())
    }
    
    /// 构建新建账号的网络命令
    pub fn build_network_command(&self) -> crate::network::NetworkCommand {
        crate::network::NetworkCommand::NewAccount {
            account_id: self.registration.account_id.clone(),
            password: self.registration.password.clone(),
            birth_date: 0, // TODO: 解析birth_date字符串转timestamp
            username: self.registration.username.clone(),
            secret_question: self.registration.secret_question.clone(),
            secret_answer: self.registration.secret_answer.clone(),
            email: self.registration.email.clone(),
        }
    }
}

impl DialogWithValidation for NewAccountDialog {
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
pub enum NewAccountAction {
    None,
    Submit,
    Cancel,
    ValidationFailed(String), // 验证失败,携带错误消息
}
