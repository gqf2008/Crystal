// NewAccountDialog - New account registration dialog
// Mirrors Client/MirScenes/LoginScene.cs::NewAccountDialog (lines 746-1142)

use regex::Regex;

/// Result codes for new account creation
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

/// New account registration data
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

/// Input field enum for focus management
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

/// New account dialog for creating new accounts
#[derive(Debug)]
pub struct NewAccountDialog {
    // UI state
    pub visible: bool,
    pub enabled: bool,
    pub ok_button_enabled: bool,

    // Registration data
    pub registration: AccountRegistration,

    // Validation state
    pub account_id_valid: bool,
    pub password1_valid: bool,
    pub password2_valid: bool,
    pub email_valid: bool,
    pub username_valid: bool,
    pub birth_date_valid: bool,
    pub question_valid: bool,
    pub answer_valid: bool,

    // Validation rules
    min_account_length: usize,
    max_account_length: usize,
    min_password_length: usize,
    max_password_length: usize,
    
    // Input focus state
    pub focused_field: InputField,
    
    // Cursor state
    pub cursor_visible: bool,
    pub cursor_blink_timer: f32,
    
    // Text selection state
    pub selection_start: Option<usize>,  // 选择起始位置 (字符索引)
    pub selection_end: Option<usize>,    // 选择结束位置
    pub is_selecting: bool,               // 是否正在选择
    pub last_click_time: f32,             // 用于双击检测
    
    // Button hover states
    pub ok_button_hovered: bool,
    pub cancel_button_hovered: bool,
    
    // IME support
    pub ime_preedit: String,
}

impl NewAccountDialog {
    /// Create new account dialog
    pub fn new(
        min_account_length: usize,
        max_account_length: usize,
        min_password_length: usize,
        max_password_length: usize,
    ) -> Self {
        Self {
            visible: false,
            enabled: true,
            ok_button_enabled: false,
            registration: AccountRegistration::default(),
            account_id_valid: false,
            password1_valid: false,
            password2_valid: false,
            email_valid: true, // Optional field
            username_valid: true, // Optional field
            birth_date_valid: true, // Optional field
            question_valid: true, // Optional field
            answer_valid: true, // Optional field
            min_account_length,
            max_account_length,
            min_password_length,
            max_password_length,
            focused_field: InputField::AccountId, // 默认聚焦账号输入框
            cursor_visible: true,
            cursor_blink_timer: 0.0,
            
            // Text selection
            selection_start: None,
            selection_end: None,
            is_selecting: false,
            last_click_time: 0.0,
            ok_button_hovered: false,
            cancel_button_hovered: false,
            ime_preedit: String::new(),
        }
    }

    /// Show dialog
    pub fn show(&mut self) {
        self.visible = true;
    }

    /// Hide dialog
    pub fn hide(&mut self) {
        self.visible = false;
    }
    
    /// Update dialog (for cursor blinking, etc.)
    pub fn update(&mut self, delta_time: f32) {
        // 光标闪烁逻辑 (每0.5秒切换一次)
        self.cursor_blink_timer += delta_time;
        if self.cursor_blink_timer >= 0.5 {
            self.cursor_blink_timer = 0.0;
            self.cursor_visible = !self.cursor_visible;
        }
    }
    
    /// Handle Tab key (switch to next field)
    pub fn handle_tab(&mut self) {
        self.focused_field = match self.focused_field {
            InputField::AccountId => InputField::Password,
            InputField::Password => InputField::PasswordConfirm,
            InputField::PasswordConfirm => InputField::Email,
            InputField::Email => InputField::Username,
            InputField::Username => InputField::BirthDate,
            InputField::BirthDate => InputField::Question,
            InputField::Question => InputField::Answer,
            InputField::Answer => InputField::AccountId, // 循环回到第一个
        };
        self.cursor_visible = true;
        self.cursor_blink_timer = 0.0;
    }
    
    /// Handle Backspace key (delete last character from focused field)
    pub fn handle_backspace(&mut self) {
        // If there's a selection, delete it instead of deleting one character
        if self.get_selection_range().is_some() {
            self.delete_selection();
            return;
        }
        
        let text = match self.focused_field {
            InputField::AccountId => &mut self.registration.account_id,
            InputField::Password => &mut self.registration.password,
            InputField::PasswordConfirm => &mut self.registration.password_confirm,
            InputField::Email => &mut self.registration.email,
            InputField::Username => &mut self.registration.username,
            InputField::BirthDate => &mut self.registration.birth_date,
            InputField::Question => &mut self.registration.secret_question,
            InputField::Answer => &mut self.registration.secret_answer,
        };
        
        if !text.is_empty() {
            text.pop();
            self.validate_current_field();
        }
    }
    
    /// Handle text input (add character to focused field with validation)
    pub fn handle_text_input(&mut self, ch: char) {
        // 如果有选中的文本,先删除它
        if self.get_selection_range().is_some() {
            self.delete_selection();
        }
        
        // 字符过滤: 根据字段类型只接受特定字符
        let is_valid_char = match self.focused_field {
            InputField::AccountId | InputField::Password | InputField::PasswordConfirm => {
                // 账号和密码: 只允许字母和数字
                ch.is_ascii_alphanumeric()
            }
            InputField::Email => {
                // Email: 允许字母、数字、@、点、下划线、加减号
                ch.is_ascii_alphanumeric() || matches!(ch, '@' | '.' | '_' | '+' | '-' | '%')
            }
            InputField::BirthDate => {
                // 生日: 只允许数字和分隔符 / -
                ch.is_ascii_digit() || matches!(ch, '/' | '-')
            }
            InputField::Username | InputField::Question | InputField::Answer => {
                // 用户名、问题、答案: 允许所有可见字符(包括中文)
                !ch.is_control()
            }
        };
        
        if !is_valid_char {
            return; // 拒绝非法字符
        }
        
        let text = match self.focused_field {
            InputField::AccountId => &mut self.registration.account_id,
            InputField::Password => &mut self.registration.password,
            InputField::PasswordConfirm => &mut self.registration.password_confirm,
            InputField::Email => &mut self.registration.email,
            InputField::Username => &mut self.registration.username,
            InputField::BirthDate => &mut self.registration.birth_date,
            InputField::Question => &mut self.registration.secret_question,
            InputField::Answer => &mut self.registration.secret_answer,
        };
        
        // 检查长度限制 (使用字符数而不是字节数,支持中文)
        let max_len = match self.focused_field {
            InputField::AccountId => self.max_account_length,
            InputField::Password | InputField::PasswordConfirm => self.max_password_length,
            InputField::Email => 50,
            InputField::Username => 20,
            InputField::BirthDate => 10,
            InputField::Question | InputField::Answer => 30,
        };
        
        let current_len = text.chars().count();
        if current_len < max_len {
            text.push(ch);
            self.validate_current_field();
        }
    }
    
    /// Handle IME preedit (拼音编辑中)
    pub fn handle_ime_preedit(&mut self, text: String) {
        self.ime_preedit = text;
    }
    
    /// Handle IME commit (中文确认输入)
    pub fn handle_ime_commit(&mut self, text: String) {
        tracing::info!("NewAccountDialog IME commit - 收到文本: '{}' ({} 字符)", text, text.chars().count());
        
        // 清除拼音编辑状态
        self.ime_preedit.clear();
        
        // 将中文字符添加到当前聚焦的输入框
        for ch in text.chars() {
            tracing::debug!("处理字符: '{}' (U+{:04X})", ch, ch as u32);
            if !ch.is_control() {
                self.handle_text_input(ch);
            } else {
                tracing::debug!("跳过控制字符: U+{:04X}", ch as u32);
            }
        }
        
        // 打印当前输入框内容
        let current_text = match self.focused_field {
            InputField::AccountId => &self.registration.account_id,
            InputField::Password => &self.registration.password,
            InputField::PasswordConfirm => &self.registration.password_confirm,
            InputField::Email => &self.registration.email,
            InputField::Username => &self.registration.username,
            InputField::BirthDate => &self.registration.birth_date,
            InputField::Question => &self.registration.secret_question,
            InputField::Answer => &self.registration.secret_answer,
        };
        tracing::info!("IME commit 后,输入框内容: '{}'", current_text);
    }
    
    // ========== 文本选择功能 ==========
    
    /// 开始文本选择 (鼠标按下时)
    pub fn start_selection(&mut self, char_index: usize) {
        self.selection_start = Some(char_index);
        self.selection_end = Some(char_index);
        self.is_selecting = true;
    }
    
    /// 更新选择范围 (鼠标拖动时)
    pub fn update_selection(&mut self, char_index: usize) {
        if self.is_selecting {
            self.selection_end = Some(char_index);
        }
    }
    
    /// 结束选择 (鼠标释放时)
    pub fn end_selection(&mut self) {
        self.is_selecting = false;
    }
    
    /// 清除选择
    pub fn clear_selection(&mut self) {
        self.selection_start = None;
        self.selection_end = None;
        self.is_selecting = false;
    }
    
    /// 获取选择的文本
    pub fn get_selected_text(&self) -> Option<String> {
        let (start, end) = self.get_selection_range()?;
        let text = self.get_current_field_text();
        let chars: Vec<char> = text.chars().collect();
        
        if end > start && end <= chars.len() {
            Some(chars[start..end].iter().collect())
        } else {
            None
        }
    }
    
    /// 获取标准化的选择范围 (start < end)
    pub fn get_selection_range(&self) -> Option<(usize, usize)> {
        match (self.selection_start, self.selection_end) {
            (Some(start), Some(end)) => {
                if start == end {
                    None // 没有选择
                } else if start < end {
                    Some((start, end))
                } else {
                    Some((end, start)) // 反向选择,交换顺序
                }
            }
            _ => None,
        }
    }
    
    /// 删除选中的文本
    pub fn delete_selection(&mut self) {
        if let Some((start, end)) = self.get_selection_range() {
            let text = self.get_current_field_text();
            let mut chars: Vec<char> = text.chars().collect();
            
            // 删除选中的字符
            chars.drain(start..end);
            let new_text: String = chars.into_iter().collect();
            
            // 更新字段
            self.set_current_field_text(new_text);
            
            // 清除选择并将光标移到删除位置
            self.clear_selection();
        }
    }
    
    /// 全选当前字段
    pub fn select_all(&mut self) {
        let text = self.get_current_field_text();
        let len = text.chars().count();
        if len > 0 {
            self.selection_start = Some(0);
            self.selection_end = Some(len);
        }
    }
    
    /// 复制选中文本到剪贴板 (需要 clipboard crate)
    pub fn copy_selection(&self) -> Option<String> {
        self.get_selected_text()
    }
    
    /// Handle mouse click on input field (for text selection)
    /// Returns true if the click was handled
    pub fn handle_mouse_click(&mut self, x: f32, y: f32, pressed: bool) -> bool {
        // 简化版本:点击输入框时清除选择,将光标移到末尾
        // TODO: 实现精确的字符索引计算来支持点击定位光标
        
        if pressed {
            // 点击时清除现有选择
            self.clear_selection();
            return true;
        }
        false
    }
    
    /// Handle mouse drag for text selection
    pub fn handle_mouse_drag(&mut self, x: f32, y: f32) {
        // TODO: 实现拖拽选择功能
        // 需要计算鼠标位置对应的字符索引,然后调用 update_selection()
    }
    
    /// 获取当前字段的文本
    fn get_current_field_text(&self) -> String {
        match self.focused_field {
            InputField::AccountId => self.registration.account_id.clone(),
            InputField::Password => self.registration.password.clone(),
            InputField::PasswordConfirm => self.registration.password_confirm.clone(),
            InputField::Email => self.registration.email.clone(),
            InputField::Username => self.registration.username.clone(),
            InputField::BirthDate => self.registration.birth_date.clone(),
            InputField::Question => self.registration.secret_question.clone(),
            InputField::Answer => self.registration.secret_answer.clone(),
        }
    }
    
    /// 设置当前字段的文本
    fn set_current_field_text(&mut self, text: String) {
        match self.focused_field {
            InputField::AccountId => self.set_account_id(text),
            InputField::Password => self.set_password(text),
            InputField::PasswordConfirm => self.set_password_confirm(text),
            InputField::Email => self.set_email(text),
            InputField::Username => self.set_username(text),
            InputField::BirthDate => self.set_birth_date(text),
            InputField::Question => self.set_secret_question(text),
            InputField::Answer => self.set_secret_answer(text),
        }
    }
    
    // ========== 文本选择功能结束 ==========
    
    /// Validate the currently focused field
    fn validate_current_field(&mut self) {
        match self.focused_field {
            InputField::AccountId => self.validate_account_id(),
            InputField::Password => {
                self.validate_password1();
                self.validate_password2(); // 也重新验证确认密码
            }
            InputField::PasswordConfirm => self.validate_password2(),
            InputField::Email => self.validate_email(),
            InputField::Username => self.validate_username(),
            InputField::BirthDate => self.validate_birth_date(),
            InputField::Question => self.validate_question(),
            InputField::Answer => self.validate_answer(),
        }
    }

    /// Close dialog and clear data
    pub fn close(&mut self) {
        self.visible = false;
        self.registration = AccountRegistration::default();
        self.reset_validation();
    }

    /// Reset validation state
    fn reset_validation(&mut self) {
        self.account_id_valid = false;
        self.password1_valid = false;
        self.password2_valid = false;
        self.email_valid = true;
        self.username_valid = true;
        self.birth_date_valid = true;
        self.question_valid = true;
        self.answer_valid = true;
        self.refresh_ok_button();
    }

    /// Update account ID
    pub fn set_account_id(&mut self, text: String) {
        self.registration.account_id = text;
        self.validate_account_id();
    }

    /// Update password
    pub fn set_password(&mut self, text: String) {
        self.registration.password = text;
        self.validate_password1();
        self.validate_password2(); // Re-check confirmation match
    }

    /// Update password confirmation
    pub fn set_password_confirm(&mut self, text: String) {
        self.registration.password_confirm = text;
        self.validate_password2();
    }

    /// Update email
    pub fn set_email(&mut self, text: String) {
        self.registration.email = text;
        self.validate_email();
    }

    /// Update username
    pub fn set_username(&mut self, text: String) {
        self.registration.username = text;
        self.validate_username();
    }

    /// Update birth date
    pub fn set_birth_date(&mut self, text: String) {
        self.registration.birth_date = text;
        self.validate_birth_date();
    }

    /// Update secret question
    pub fn set_secret_question(&mut self, text: String) {
        self.registration.secret_question = text;
        self.validate_question();
    }

    /// Update secret answer
    pub fn set_secret_answer(&mut self, text: String) {
        self.registration.secret_answer = text;
        self.validate_answer();
    }

    /// Validate account ID
    fn validate_account_id(&mut self) {
        if self.registration.account_id.is_empty() {
            self.account_id_valid = false;
        } else {
            let pattern = format!(
                r"^[A-Za-z0-9]{{{},{}}}$",
                self.min_account_length,
                self.max_account_length
            );
            if let Ok(regex) = Regex::new(&pattern) {
                self.account_id_valid = regex.is_match(&self.registration.account_id);
            } else {
                self.account_id_valid = false;
            }
        }
        self.refresh_ok_button();
    }

    /// Validate password (必须包含字母和数字)
    fn validate_password1(&mut self) {
        if self.registration.password.is_empty() {
            self.password1_valid = false;
        } else {
            // 检查长度
            let len = self.registration.password.len();
            let valid_length = len >= self.min_password_length && len <= self.max_password_length;
            
            // 检查字符是否都是字母或数字
            let all_alphanumeric = self.registration.password.chars().all(|c| c.is_ascii_alphanumeric());
            
            // 检查是否同时包含字母和数字 (增强安全性)
            let has_letter = self.registration.password.chars().any(|c| c.is_ascii_alphabetic());
            let has_digit = self.registration.password.chars().any(|c| c.is_ascii_digit());
            
            self.password1_valid = valid_length && all_alphanumeric && has_letter && has_digit;
        }
        self.refresh_ok_button();
    }

    /// Validate password confirmation
    fn validate_password2(&mut self) {
        if self.registration.password_confirm.is_empty() {
            self.password2_valid = false;
        } else {
            let pattern = format!(
                r"^[A-Za-z0-9]{{{},{}}}$",
                self.min_password_length,
                self.max_password_length
            );
            if let Ok(regex) = Regex::new(&pattern) {
                let matches_pattern = regex.is_match(&self.registration.password_confirm);
                let matches_password = self.registration.password == self.registration.password_confirm;
                self.password2_valid = matches_pattern && matches_password;
            } else {
                self.password2_valid = false;
            }
        }
        self.refresh_ok_button();
    }

    /// Validate email (必填,格式验证)
    fn validate_email(&mut self) {
        // C# 原版: Email 是必填字段
        if self.registration.email.is_empty() {
            self.email_valid = false;
        } else if self.registration.email.len() > 50 {
            self.email_valid = false;
        } else {
            // Enhanced email pattern - 标准 RFC 5322 简化版
            let pattern = r"^[a-zA-Z0-9._%+-]+@[a-zA-Z0-9.-]+\.[a-zA-Z]{2,}$";
            if let Ok(regex) = Regex::new(pattern) {
                self.email_valid = regex.is_match(&self.registration.email);
            } else {
                self.email_valid = false;
            }
        }
        self.refresh_ok_button();
    }

    /// Validate username (必填, 1-20字符,可包含中文)
    fn validate_username(&mut self) {
        // C# 原版: Username 是必填字段
        if self.registration.username.is_empty() {
            self.username_valid = false;
        } else {
            let len = self.registration.username.chars().count();
            // 允许中文、字母、数字、下划线,长度1-20
            self.username_valid = len >= 1 && len <= 20;
        }
        self.refresh_ok_button();
    }

    /// Validate birth date (必填, 格式: MM/DD/YYYY 或 YYYY-MM-DD)
    fn validate_birth_date(&mut self) {
        // C# 原版: BirthDate 是必填字段
        if self.registration.birth_date.is_empty() {
            self.birth_date_valid = false;
        } else {
            // 接受两种格式: MM/DD/YYYY (美式) 或 YYYY-MM-DD (ISO)
            let pattern1 = r"^(0[1-9]|1[0-2])/(0[1-9]|[12][0-9]|3[01])/\d{4}$"; // MM/DD/YYYY
            let pattern2 = r"^\d{4}-(0[1-9]|1[0-2])-(0[1-9]|[12][0-9]|3[01])$"; // YYYY-MM-DD
            
            let valid_format1 = Regex::new(pattern1).map(|r| r.is_match(&self.registration.birth_date)).unwrap_or(false);
            let valid_format2 = Regex::new(pattern2).map(|r| r.is_match(&self.registration.birth_date)).unwrap_or(false);
            
            self.birth_date_valid = valid_format1 || valid_format2;
        }
        self.refresh_ok_button();
    }

    /// Validate secret question (必填, 1-30字符)
    fn validate_question(&mut self) {
        // C# 原版: Question 是必填字段
        if self.registration.secret_question.is_empty() {
            self.question_valid = false;
        } else {
            let len = self.registration.secret_question.chars().count();
            self.question_valid = len >= 1 && len <= 30;
        }
        self.refresh_ok_button();
    }

    /// Validate secret answer (必填, 1-30字符)
    fn validate_answer(&mut self) {
        // C# 原版: Answer 是必填字段
        if self.registration.secret_answer.is_empty() {
            self.answer_valid = false;
        } else {
            let len = self.registration.secret_answer.chars().count();
            self.answer_valid = len >= 1 && len <= 30;
        }
        self.refresh_ok_button();
    }

    /// Refresh OK button state
    fn refresh_ok_button(&mut self) {
        self.ok_button_enabled = self.account_id_valid
            && self.password1_valid
            && self.password2_valid
            && self.email_valid
            && self.username_valid
            && self.birth_date_valid
            && self.question_valid
            && self.answer_valid;
    }

    /// Check if can submit
    pub fn can_submit(&self) -> bool {
        self.enabled && self.ok_button_enabled
    }

    /// Get validation status for specific field
    pub fn get_field_validation(&self, field: &str) -> bool {
        match field {
            "account_id" => self.account_id_valid,
            "password1" => self.password1_valid,
            "password2" => self.password2_valid,
            "email" => self.email_valid,
            "username" => self.username_valid,
            "birth_date" => self.birth_date_valid,
            "question" => self.question_valid,
            "answer" => self.answer_valid,
            _ => false,
        }
    }
}

impl Default for NewAccountDialog {
    fn default() -> Self {
        Self::new(3, 20, 3, 20)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_new_account_dialog_creation() {
        let dialog = NewAccountDialog::default();
        assert!(!dialog.visible);
        assert!(!dialog.ok_button_enabled);
    }

    #[test]
    fn test_password_confirmation() {
        let mut dialog = NewAccountDialog::default();

        dialog.set_account_id("TestUser".to_string());
        dialog.set_password("TestPass".to_string());
        dialog.set_password_confirm("WrongPass".to_string());

        assert!(!dialog.password2_valid);
        assert!(!dialog.can_submit());

        dialog.set_password_confirm("TestPass".to_string());
        assert!(dialog.password2_valid);
        assert!(dialog.can_submit());
    }

    #[test]
    fn test_email_validation() {
        let mut dialog = NewAccountDialog::default();

        // Empty is valid (optional)
        dialog.set_email(String::new());
        assert!(dialog.email_valid);

        // Valid email
        dialog.set_email("test@example.com".to_string());
        assert!(dialog.email_valid);

        // Invalid email
        dialog.set_email("invalid-email".to_string());
        assert!(!dialog.email_valid);
    }
}