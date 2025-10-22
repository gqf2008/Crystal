// ChangePasswordDialog - Password change dialog
// Mirrors Client/MirScenes/LoginScene.cs::ChangePasswordDialog (lines 1144-1354)

use regex::Regex;

/// Result codes for password change
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

/// Input field enum for focus management
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PasswordInputField {
    AccountId,
    CurrentPassword,
    NewPassword,
    NewPasswordConfirm,
}

/// Change password dialog
#[derive(Debug)]
pub struct ChangePasswordDialog {
    // UI state
    pub visible: bool,
    pub enabled: bool,
    pub ok_button_enabled: bool,

    // Input fields
    pub account_id: String,
    pub current_password: String,
    pub new_password: String,
    pub new_password_confirm: String,

    // Validation state
    pub account_id_valid: bool,
    pub current_password_valid: bool,
    pub new_password1_valid: bool,
    pub new_password2_valid: bool,

    // Validation rules
    min_account_length: usize,
    max_account_length: usize,
    min_password_length: usize,
    max_password_length: usize,
    
    // Input focus state
    pub focused_field: PasswordInputField,
    
    // Cursor state
    pub cursor_visible: bool,
    pub cursor_blink_timer: f32,
    
    // Button hover states
    pub ok_button_hovered: bool,
    pub cancel_button_hovered: bool,
    
    // IME support
    pub ime_preedit: String,
}

impl ChangePasswordDialog {
    /// Create new change password dialog
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
            account_id: String::new(),
            current_password: String::new(),
            new_password: String::new(),
            new_password_confirm: String::new(),
            account_id_valid: false,
            current_password_valid: false,
            new_password1_valid: false,
            new_password2_valid: false,
            min_account_length,
            max_account_length,
            min_password_length,
            max_password_length,
            focused_field: PasswordInputField::AccountId, // 默认聚焦账号输入框
            cursor_visible: true,
            cursor_blink_timer: 0.0,
            ok_button_hovered: false,
            cancel_button_hovered: false,
            ime_preedit: String::new(),
        }
    }

    /// Show dialog with optional autofill
    pub fn show(&mut self, autofill_id: Option<String>, autofill_password: Option<String>) {
        self.visible = true;

        if let Some(id) = autofill_id {
            self.set_account_id(id);
        }

        if let Some(password) = autofill_password {
            self.set_current_password(password);
        }
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
            PasswordInputField::AccountId => PasswordInputField::CurrentPassword,
            PasswordInputField::CurrentPassword => PasswordInputField::NewPassword,
            PasswordInputField::NewPassword => PasswordInputField::NewPasswordConfirm,
            PasswordInputField::NewPasswordConfirm => PasswordInputField::AccountId, // 循环回到第一个
        };
        self.cursor_visible = true;
        self.cursor_blink_timer = 0.0;
    }
    
    /// Handle Backspace key (delete last character from focused field)
    pub fn handle_backspace(&mut self) {
        let text = match self.focused_field {
            PasswordInputField::AccountId => &mut self.account_id,
            PasswordInputField::CurrentPassword => &mut self.current_password,
            PasswordInputField::NewPassword => &mut self.new_password,
            PasswordInputField::NewPasswordConfirm => &mut self.new_password_confirm,
        };
        
        if !text.is_empty() {
            text.pop();
            self.validate_current_field();
        }
    }
    
    /// Handle text input (add character to focused field)
    pub fn handle_text_input(&mut self, ch: char) {
        // 字符过滤: 账号和密码只允许字母和数字
        if !ch.is_ascii_alphanumeric() {
            return; // 拒绝非法字符
        }
        
        let text = match self.focused_field {
            PasswordInputField::AccountId => &mut self.account_id,
            PasswordInputField::CurrentPassword => &mut self.current_password,
            PasswordInputField::NewPassword => &mut self.new_password,
            PasswordInputField::NewPasswordConfirm => &mut self.new_password_confirm,
        };
        
        // 检查长度限制
        let max_len = match self.focused_field {
            PasswordInputField::AccountId => self.max_account_length,
            _ => self.max_password_length,
        };
        
        if text.len() < max_len {
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
        // 清除拼音编辑状态
        self.ime_preedit.clear();
        
        // 将中文字符添加到当前聚焦的输入框
        for ch in text.chars() {
            if !ch.is_control() {
                self.handle_text_input(ch);
            }
        }
    }
    
    /// Validate the currently focused field
    fn validate_current_field(&mut self) {
        match self.focused_field {
            PasswordInputField::AccountId => self.validate_account_id(),
            PasswordInputField::CurrentPassword => self.validate_current_password(),
            PasswordInputField::NewPassword => {
                self.validate_new_password1();
                self.validate_new_password2(); // 也重新验证确认密码
            }
            PasswordInputField::NewPasswordConfirm => self.validate_new_password2(),
        }
    }

    /// Close dialog and clear data
    pub fn close(&mut self) {
        self.visible = false;
        self.account_id.clear();
        self.current_password.clear();
        self.new_password.clear();
        self.new_password_confirm.clear();
        self.reset_validation();
    }

    /// Reset validation state
    fn reset_validation(&mut self) {
        self.account_id_valid = false;
        self.current_password_valid = false;
        self.new_password1_valid = false;
        self.new_password2_valid = false;
        self.refresh_ok_button();
    }

    /// Update account ID
    pub fn set_account_id(&mut self, text: String) {
        self.account_id = text;
        self.validate_account_id();
    }

    /// Update current password
    pub fn set_current_password(&mut self, text: String) {
        self.current_password = text;
        self.validate_current_password();
    }

    /// Update new password
    pub fn set_new_password(&mut self, text: String) {
        self.new_password = text;
        self.validate_new_password1();
        self.validate_new_password2(); // Re-check confirmation
    }

    /// Update new password confirmation
    pub fn set_new_password_confirm(&mut self, text: String) {
        self.new_password_confirm = text;
        self.validate_new_password2();
    }

    /// Validate account ID
    fn validate_account_id(&mut self) {
        if self.account_id.is_empty() {
            self.account_id_valid = false;
        } else {
            let pattern = format!(
                r"^[A-Za-z0-9]{{{},{}}}$",
                self.min_account_length,
                self.max_account_length
            );
            if let Ok(regex) = Regex::new(&pattern) {
                self.account_id_valid = regex.is_match(&self.account_id);
            } else {
                self.account_id_valid = false;
            }
        }
        self.refresh_ok_button();
    }

    /// Validate current password (字母数字,符合长度即可)
    fn validate_current_password(&mut self) {
        if self.current_password.is_empty() {
            self.current_password_valid = false;
        } else {
            let pattern = format!(
                r"^[A-Za-z0-9]{{{},{}}}$",
                self.min_password_length,
                self.max_password_length
            );
            if let Ok(regex) = Regex::new(&pattern) {
                self.current_password_valid = regex.is_match(&self.current_password);
            } else {
                self.current_password_valid = false;
            }
        }
        self.refresh_ok_button();
    }

    /// Validate new password (字母数字,符合长度即可)
    fn validate_new_password1(&mut self) {
        if self.new_password.is_empty() {
            self.new_password1_valid = false;
        } else {
            let pattern = format!(
                r"^[A-Za-z0-9]{{{},{}}}$",
                self.min_password_length,
                self.max_password_length
            );
            if let Ok(regex) = Regex::new(&pattern) {
                self.new_password1_valid = regex.is_match(&self.new_password);
            } else {
                self.new_password1_valid = false;
            }
        }
        self.refresh_ok_button();
    }

    /// Validate new password confirmation (必须与新密码一致)
    fn validate_new_password2(&mut self) {
        if self.new_password == self.new_password_confirm {
            self.new_password2_valid = self.new_password1_valid;
        } else {
            self.new_password2_valid = false;
        }
        self.refresh_ok_button();
    }

    /// Refresh OK button state
    fn refresh_ok_button(&mut self) {
        self.ok_button_enabled = self.account_id_valid
            && self.current_password_valid
            && self.new_password1_valid
            && self.new_password2_valid;
    }

    /// Check if can submit
    pub fn can_submit(&self) -> bool {
        self.enabled && self.ok_button_enabled
    }

    /// Get validation status for specific field
    pub fn get_field_validation(&self, field: &str) -> bool {
        match field {
            "account_id" => self.account_id_valid,
            "current_password" => self.current_password_valid,
            "new_password1" => self.new_password1_valid,
            "new_password2" => self.new_password2_valid,
            _ => false,
        }
    }

    /// Get change password request data (only if valid)
    pub fn get_request_data(&self) -> Option<(String, String, String)> {
        if self.can_submit() {
            Some((
                self.account_id.clone(),
                self.current_password.clone(),
                self.new_password.clone(),
            ))
        } else {
            None
        }
    }
}

impl Default for ChangePasswordDialog {
    fn default() -> Self {
        Self::new(3, 20, 3, 20)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_change_password_dialog_creation() {
        let dialog = ChangePasswordDialog::default();
        assert!(!dialog.visible);
        assert!(!dialog.ok_button_enabled);
    }

    #[test]
    fn test_password_validation() {
        let mut dialog = ChangePasswordDialog::default();

        dialog.set_account_id("TestUser".to_string());
        dialog.set_current_password("OldPass123".to_string());
        dialog.set_new_password("NewPass123".to_string());
        dialog.set_new_password_confirm("NewPass123".to_string());

        assert!(dialog.can_submit());
    }

    #[test]
    fn test_password_confirmation_mismatch() {
        let mut dialog = ChangePasswordDialog::default();

        dialog.set_account_id("TestUser".to_string());
        dialog.set_current_password("OldPass123".to_string());
        dialog.set_new_password("NewPass123".to_string());
        dialog.set_new_password_confirm("DifferentPass".to_string());

        assert!(!dialog.new_password2_valid);
        assert!(!dialog.can_submit());
    }

    #[test]
    fn test_autofill() {
        let mut dialog = ChangePasswordDialog::default();

        dialog.show(
            Some("AutoUser".to_string()),
            Some("AutoPass".to_string()),
        );

        assert!(dialog.visible);
        assert!(dialog.account_id_valid);
        assert!(dialog.current_password_valid);
    }
}