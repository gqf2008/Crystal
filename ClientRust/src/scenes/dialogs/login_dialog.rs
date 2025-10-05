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
