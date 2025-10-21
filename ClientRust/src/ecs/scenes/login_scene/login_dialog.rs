// LoginDialog - Login credentials dialog
// Mirrors Client/MirScenes/LoginScene.cs::LoginDialog (lines 323-550)

use regex::Regex;

/// Login dialog for entering account credentials
#[derive(Debug)]
pub struct LoginDialog {
    // UI state
    pub visible: bool,
    pub enabled: bool,

    // Input fields
    pub account_id: String,
    pub password: String,

    // Validation state
    account_id_valid: bool,
    password_valid: bool,

    // Validation rules
    min_account_length: usize,
    max_account_length: usize,
    min_password_length: usize,
    max_password_length: usize,
    
    // Input focus state
    pub account_focused: bool,
    pub password_focused: bool,
    
    // Cursor state
    pub cursor_visible: bool,
    pub cursor_blink_timer: f32,
    
    // Button hover states
    pub ok_button_hovered: bool,
    pub new_account_button_hovered: bool,
    pub change_password_button_hovered: bool,
    pub close_button_hovered: bool,
    
    // IME support
    pub ime_preedit: String,  // 拼音编辑中的文本
}

impl LoginDialog {
    /// Create new login dialog
    pub fn new(
        min_account_length: usize,
        max_account_length: usize,
        min_password_length: usize,
        max_password_length: usize,
    ) -> Self {
        Self {
            visible: false,
            enabled: true,
            account_id: String::new(),
            password: String::new(),
            account_id_valid: false,
            password_valid: false,
            min_account_length,
            max_account_length,
            min_password_length,
            max_password_length,
            account_focused: true,  // 默认聚焦账号输入框
            password_focused: false,
            cursor_visible: true,
            cursor_blink_timer: 0.0,
            ok_button_hovered: false,
            new_account_button_hovered: false,
            change_password_button_hovered: false,
            close_button_hovered: false,
            ime_preedit: String::new(),
        }
    }

    /// Load from settings
    pub fn load_from_settings(&mut self, account_id: String, password: String) {
        self.account_id = account_id;
        self.password = password;
        self.validate_account_id();
        self.validate_password();
    }

    /// Show dialog
    pub fn show(&mut self) {
        if self.visible {
            return;
        }
        self.visible = true;
        // Auto-login if both fields are filled
        if !self.account_id.is_empty() && !self.password.is_empty() {
            // Will trigger login in update
        }
    }

    /// Hide dialog
    pub fn hide(&mut self) {
        self.visible = false;
    }

    /// Clear input fields
    pub fn clear(&mut self) {
        self.account_id.clear();
        self.password.clear();
        self.account_id_valid = false;
        self.password_valid = false;
    }

    /// Update account ID and validate
    pub fn set_account_id(&mut self, text: String) {
        self.account_id = text;
        self.validate_account_id();
    }

    /// Update password and validate
    pub fn set_password(&mut self, text: String) {
        self.password = text;
        self.validate_password();
    }

    /// Validate account ID
    fn validate_account_id(&mut self) {
        if self.account_id.is_empty() {
            self.account_id_valid = false;
            return;
        }

        // Regex: alphanumeric only, min-max length
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

    /// Validate password
    fn validate_password(&mut self) {
        if self.password.is_empty() {
            self.password_valid = false;
            return;
        }

        // Regex: alphanumeric only, min-max length
        let pattern = format!(
            r"^[A-Za-z0-9]{{{},{}}}$",
            self.min_password_length,
            self.max_password_length
        );

        if let Ok(regex) = Regex::new(&pattern) {
            self.password_valid = regex.is_match(&self.password);
        } else {
            self.password_valid = false;
        }
    }

    /// Check if OK button should be enabled
    pub fn is_ok_button_enabled(&self) -> bool {
        self.enabled && self.account_id_valid && self.password_valid
    }

    /// Get account ID validation status
    pub fn is_account_id_valid(&self) -> bool {
        self.account_id_valid
    }

    /// Get password validation status
    pub fn is_password_valid(&self) -> bool {
        self.password_valid
    }

    /// Handle Enter key press
    pub fn handle_enter_key(&self) -> bool {
        if !self.account_id_valid {
            // Focus should move to account ID
            return false;
        }
        if !self.password_valid {
            // Focus should move to password
            return false;
        }
        // Can submit if OK button enabled
        self.is_ok_button_enabled()
    }

    /// Get login credentials (only if valid)
    pub fn get_credentials(&self) -> Option<(String, String)> {
        if self.is_ok_button_enabled() {
            Some((self.account_id.clone(), self.password.clone()))
        } else {
            None
        }
    }
    
    /// Update cursor blink animation
    pub fn update(&mut self, delta_time: f32) {
        // 光标闪烁 (每 0.5 秒切换)
        self.cursor_blink_timer += delta_time;
        if self.cursor_blink_timer >= 0.5 {
            self.cursor_visible = !self.cursor_visible;
            self.cursor_blink_timer = 0.0;
        }
    }
    
    /// Handle text input (for account or password based on focus)
    pub fn handle_text_input(&mut self, character: char) {
        if self.account_focused {
            if self.account_id.len() < self.max_account_length {
                self.account_id.push(character);
                self.validate_account_id();
            }
        } else if self.password_focused {
            if self.password.len() < self.max_password_length {
                self.password.push(character);
                self.validate_password();
            }
        }
    }
    
    /// Handle backspace key
    pub fn handle_backspace(&mut self) {
        if self.account_focused && !self.account_id.is_empty() {
            self.account_id.pop();
            self.validate_account_id();
        } else if self.password_focused && !self.password.is_empty() {
            self.password.pop();
            self.validate_password();
        }
    }
    
    /// Handle Tab key (switch focus between fields)
    pub fn handle_tab(&mut self) {
        if self.account_focused {
            self.account_focused = false;
            self.password_focused = true;
        } else if self.password_focused {
            self.account_focused = true;
            self.password_focused = false;
        } else {
            // If no focus, focus on account
            self.account_focused = true;
            self.password_focused = false;
        }
        // Reset cursor blink when switching focus
        self.cursor_visible = true;
        self.cursor_blink_timer = 0.0;
    }
    
    /// Set focus to account field
    pub fn focus_account(&mut self) {
        self.account_focused = true;
        self.password_focused = false;
        self.cursor_visible = true;
        self.cursor_blink_timer = 0.0;
    }
    
    /// Set focus to password field
    pub fn focus_password(&mut self) {
        self.account_focused = false;
        self.password_focused = true;
        self.cursor_visible = true;
        self.cursor_blink_timer = 0.0;
    }
    
    /// Clear focus from both fields
    pub fn clear_focus(&mut self) {
        self.account_focused = false;
        self.password_focused = false;
    }
    
    /// Check if mouse is over a button (given dialog position)
    pub fn update_button_hover(&mut self, mouse_x: f32, mouse_y: f32, dialog_x: f32, dialog_y: f32) {
        // OK 按钮区域: (227, 81), 大小 42x42
        let ok_x = dialog_x + 227.0;
        let ok_y = dialog_y + 81.0;
        self.ok_button_hovered = 
            mouse_x >= ok_x && mouse_x <= ok_x + 42.0 &&
            mouse_y >= ok_y && mouse_y <= ok_y + 42.0;
        
        // 新建账号按钮: (60, 163), 大小约 102x21
        let new_account_x = dialog_x + 60.0;
        let new_account_y = dialog_y + 163.0;
        self.new_account_button_hovered =
            mouse_x >= new_account_x && mouse_x <= new_account_x + 102.0 &&
            mouse_y >= new_account_y && mouse_y <= new_account_y + 21.0;
        
        // 修改密码按钮: (166, 163), 大小约 102x21
        let change_pass_x = dialog_x + 166.0;
        let change_pass_y = dialog_y + 163.0;
        self.change_password_button_hovered =
            mouse_x >= change_pass_x && mouse_x <= change_pass_x + 102.0 &&
            mouse_y >= change_pass_y && mouse_y <= change_pass_y + 21.0;
        
        // 关闭按钮: (166, 189), 大小约 102x21
        let close_x = dialog_x + 166.0;
        let close_y = dialog_y + 189.0;
        self.close_button_hovered =
            mouse_x >= close_x && mouse_x <= close_x + 102.0 &&
            mouse_y >= close_y && mouse_y <= close_y + 21.0;
    }
    
    /// Handle mouse click on dialog (returns action)
    pub fn handle_click(&mut self, mouse_x: f32, mouse_y: f32, dialog_x: f32, dialog_y: f32) -> DialogAction {
        // 检查输入框点击 (设置焦点)
        let account_box_x = dialog_x + 85.0;
        let account_box_y = dialog_y + 85.0;
        let account_box_clicked = 
            mouse_x >= account_box_x && mouse_x <= account_box_x + 136.0 &&
            mouse_y >= account_box_y && mouse_y <= account_box_y + 15.0;
        
        if account_box_clicked {
            self.focus_account();
            return DialogAction::None;
        }
        
        let password_box_x = dialog_x + 85.0;
        let password_box_y = dialog_y + 108.0;
        let password_box_clicked =
            mouse_x >= password_box_x && mouse_x <= password_box_x + 136.0 &&
            mouse_y >= password_box_y && mouse_y <= password_box_y + 15.0;
        
        if password_box_clicked {
            self.focus_password();
            return DialogAction::None;
        }
        
        // 检查按钮点击
        if self.ok_button_hovered {
            return DialogAction::Login;
        }
        
        if self.new_account_button_hovered {
            return DialogAction::NewAccount;
        }
        
        if self.change_password_button_hovered {
            return DialogAction::ChangePassword;
        }
        
        if self.close_button_hovered {
            return DialogAction::Close;
        }
        
        // 点击对话框其他区域，清除焦点
        self.clear_focus();
        DialogAction::None
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
}

/// Actions that can be triggered from the login dialog
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DialogAction {
    None,
    Login,
    NewAccount,
    ChangePassword,
    Close,
}

impl Default for LoginDialog {
    fn default() -> Self {
        // Default limits from C# Globals
        Self::new(3, 20, 3, 20)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_login_dialog_creation() {
        let dialog = LoginDialog::new(3, 20, 3, 20);
        assert!(!dialog.visible);
        assert!(dialog.enabled);
        assert!(!dialog.is_ok_button_enabled());
    }

    #[test]
    fn test_account_id_validation() {
        let mut dialog = LoginDialog::default();

        // Too short
        dialog.set_account_id("ab".to_string());
        assert!(!dialog.is_account_id_valid());

        // Valid
        dialog.set_account_id("ValidUser123".to_string());
        assert!(dialog.is_account_id_valid());

        // Invalid characters
        dialog.set_account_id("Invalid@User".to_string());
        assert!(!dialog.is_account_id_valid());

        // Too long
        dialog.set_account_id("ThisIsWayTooLongForAnAccountID".to_string());
        assert!(!dialog.is_account_id_valid());
    }

    #[test]
    fn test_password_validation() {
        let mut dialog = LoginDialog::default();

        // Too short
        dialog.set_password("ab".to_string());
        assert!(!dialog.is_password_valid());

        // Valid
        dialog.set_password("ValidPass123".to_string());
        assert!(dialog.is_password_valid());

        // Invalid characters
        dialog.set_password("Invalid!Pass".to_string());
        assert!(!dialog.is_password_valid());
    }

    #[test]
    fn test_ok_button_enabled() {
        let mut dialog = LoginDialog::default();

        // Both invalid
        assert!(!dialog.is_ok_button_enabled());

        // Only account valid
        dialog.set_account_id("ValidUser".to_string());
        assert!(!dialog.is_ok_button_enabled());

        // Both valid
        dialog.set_password("ValidPass".to_string());
        assert!(dialog.is_ok_button_enabled());
    }

    #[test]
    fn test_get_credentials() {
        let mut dialog = LoginDialog::default();
        dialog.set_account_id("TestUser".to_string());
        dialog.set_password("TestPass".to_string());

        let creds = dialog.get_credentials();
        assert!(creds.is_some());

        let (user, pass) = creds.unwrap();
        assert_eq!(user, "TestUser");
        assert_eq!(pass, "TestPass");
    }
}
