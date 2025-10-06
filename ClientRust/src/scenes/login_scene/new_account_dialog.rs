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
    account_id_valid: bool,
    password1_valid: bool,
    password2_valid: bool,
    email_valid: bool,
    username_valid: bool,
    birth_date_valid: bool,
    question_valid: bool,
    answer_valid: bool,

    // Validation rules
    min_account_length: usize,
    max_account_length: usize,
    min_password_length: usize,
    max_password_length: usize,
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
    pub fn update(&mut self, _delta_time: f32) {
        // TODO: 添加光标闪烁逻辑
        // TODO: 添加输入焦点管理
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

    /// Validate password
    fn validate_password1(&mut self) {
        if self.registration.password.is_empty() {
            self.password1_valid = false;
        } else {
            let pattern = format!(
                r"^[A-Za-z0-9]{{{},{}}}$",
                self.min_password_length,
                self.max_password_length
            );
            if let Ok(regex) = Regex::new(&pattern) {
                self.password1_valid = regex.is_match(&self.registration.password);
            } else {
                self.password1_valid = false;
            }
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

    /// Validate email (optional)
    fn validate_email(&mut self) {
        if self.registration.email.is_empty() {
            self.email_valid = true;
        } else if self.registration.email.len() > 50 {
            self.email_valid = false;
        } else {
            // Simple email pattern
            let pattern = r"^[a-zA-Z0-9._%+-]+@[a-zA-Z0-9.-]+\.[a-zA-Z]{2,}$";
            if let Ok(regex) = Regex::new(pattern) {
                self.email_valid = regex.is_match(&self.registration.email);
            } else {
                self.email_valid = false;
            }
        }
        self.refresh_ok_button();
    }

    /// Validate username (optional, max 20 chars)
    fn validate_username(&mut self) {
        if self.registration.username.is_empty() {
            self.username_valid = true;
        } else {
            self.username_valid = self.registration.username.len() <= 20;
        }
        self.refresh_ok_button();
    }

    /// Validate birth date (optional)
    fn validate_birth_date(&mut self) {
        if self.registration.birth_date.is_empty() {
            self.birth_date_valid = true;
        } else {
            // Accept various date formats
            self.birth_date_valid = self.registration.birth_date.len() <= 10;
        }
        self.refresh_ok_button();
    }

    /// Validate secret question (optional, max 30 chars)
    fn validate_question(&mut self) {
        if self.registration.secret_question.is_empty() {
            self.question_valid = true;
        } else {
            self.question_valid = self.registration.secret_question.len() <= 30;
        }
        self.refresh_ok_button();
    }

    /// Validate secret answer (optional, max 30 chars)
    fn validate_answer(&mut self) {
        if self.registration.secret_answer.is_empty() {
            self.answer_valid = true;
        } else {
            self.answer_valid = self.registration.secret_answer.len() <= 30;
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