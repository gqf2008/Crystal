// LoginScene - Login scene implementation
// Mirrors Client/MirScenes/LoginScene.cs

use super::scene_trait::{Scene, SceneType, MouseButton, KeyCode};
use crate::network::game_client::GameEvent;
use crate::network::protocol::CharacterSummary;

#[derive(Debug, Clone)]
pub struct BanInfo {
    pub reason: String,
    pub expiry_date: i64,
}

/// Login scene state
#[derive(Debug)]
pub struct LoginScene {
    // Network connection
    pub connecting: bool,
    pub connect_attempts: u32,
    
    // UI state
    pub username: String,
    pub password: String,
    pub remember_account: bool,
    pub version_checked: bool,
    pub version_valid: bool,
    pub login_enabled: bool,
    pub require_password_change: bool,
    pub ready_for_character_select: bool,
    
    // Status tracking
    pub last_status: Option<String>,
    pub message_log: Vec<String>,
    pub last_login_result: Option<u8>,
    pub last_new_account_result: Option<u8>,
    pub last_change_password_result: Option<u8>,
    pub login_ban_info: Option<BanInfo>,
    pub password_change_ban_info: Option<BanInfo>,
    pub characters: Vec<CharacterSummary>,
    
    // Dialogs (TODO: implement when dialog system ready)
    // login_dialog: LoginDialog,
    // new_account_dialog: Option<NewAccountDialog>,
    // change_password_dialog: Option<ChangePasswordDialog>,
}

impl LoginScene {
    /// Create new login scene
    pub fn new() -> Self {
        Self {
            connecting: false,
            connect_attempts: 0,
            username: String::new(),
            password: String::new(),
            remember_account: false,
            version_checked: false,
            version_valid: false,
            login_enabled: false,
            require_password_change: false,
            ready_for_character_select: false,
            last_status: None,
            message_log: Vec::new(),
            last_login_result: None,
            last_new_account_result: None,
            last_change_password_result: None,
            login_ban_info: None,
            password_change_ban_info: None,
            characters: Vec::new(),
        }
    }
    
    pub fn record_status<S: Into<String>>(&mut self, message: S) {
        let message = message.into();
        self.last_status = Some(message.clone());
        self.message_log.push(message);
    }

    fn handle_client_version_response(&mut self, result: u8) {
        self.version_checked = true;
        self.connecting = false;
        match result {
            0 => {
                self.version_valid = false;
                self.login_enabled = false;
                self.record_status("Wrong version, please update your game. Connection closed.");
            }
            1 => {
                self.version_valid = true;
                self.login_enabled = true;
                self.record_status("Client version accepted by server. Login dialog unlocked.");
            }
            other => {
                self.version_valid = false;
                self.login_enabled = false;
                self.record_status(format!("Unknown client version response: {}", other));
            }
        }
    }

    fn handle_login_response(&mut self, result: u8) {
        self.last_login_result = Some(result);
        self.connecting = false;
        self.login_enabled = true;
        self.require_password_change = result == 5;
        self.ready_for_character_select = false;
        self.characters.clear();
        if let Some(message) = Self::login_result_message(result) {
            self.record_status(message);
        } else {
            self.record_status(format!("Unknown login result code {}", result));
        }
    }

    fn handle_login_success(&mut self, characters: &[CharacterSummary]) {
        self.connecting = false;
        self.login_enabled = false;
        self.version_checked = true;
        self.version_valid = true;
        self.require_password_change = false;
        self.ready_for_character_select = true;
        self.characters = characters.to_vec();
        self.record_status(format!(
            "Login successful. {} character(s) available.",
            self.characters.len()
        ));
    }

    fn handle_login_ban(&mut self, reason: &str, expiry_date: i64) {
        self.connecting = false;
        self.login_enabled = true;
        self.require_password_change = false;
        self.ready_for_character_select = false;
        let info = BanInfo {
            reason: reason.to_string(),
            expiry_date,
        };
        self.login_ban_info = Some(info.clone());
        self.record_status(Self::ban_message("Login", &info));
    }

    fn handle_new_account_response(&mut self, result: u8) {
        self.last_new_account_result = Some(result);
        if let Some(message) = Self::new_account_result_message(result) {
            self.record_status(message);
        } else {
            self.record_status(format!("Unknown new account result code {}", result));
        }
    }

    fn handle_change_password_response(&mut self, result: u8) {
        self.last_change_password_result = Some(result);
        if let Some(message) = Self::change_password_result_message(result) {
            self.record_status(message);
        } else {
            self.record_status(format!("Unknown change password result code {}", result));
        }
    }

    fn handle_change_password_ban(&mut self, reason: &str, expiry_date: i64) {
        let info = BanInfo {
            reason: reason.to_string(),
            expiry_date,
        };
        self.password_change_ban_info = Some(info.clone());
        self.record_status(Self::ban_message("Password change", &info));
    }

    fn login_result_message(result: u8) -> Option<&'static str> {
        match result {
            0 => Some("Logging in is currently disabled."),
            1 => Some("Your AccountID is not acceptable."),
            2 => Some("Your Password is not acceptable."),
            3 => Some("No account with that ID exists."),
            4 => Some("Incorrect password for that account ID."),
            5 => Some("The account's password must be changed before logging in."),
            _ => None,
        }
    }

    fn new_account_result_message(result: u8) -> Option<&'static str> {
        match result {
            0 => Some("Account creation is currently disabled."),
            1 => Some("Your AccountID is not acceptable."),
            2 => Some("Your Password is not acceptable."),
            3 => Some("Your E-Mail Address is not acceptable."),
            4 => Some("Your User Name is not acceptable."),
            5 => Some("Your Secret Question is not acceptable."),
            6 => Some("Your Secret Answer is not acceptable."),
            7 => Some("An account with this ID already exists."),
            8 => Some("Your account was created successfully."),
            _ => None,
        }
    }

    fn change_password_result_message(result: u8) -> Option<&'static str> {
        match result {
            0 => Some("Password changing is currently disabled."),
            1 => Some("Your AccountID is not acceptable."),
            2 => Some("The current password is not acceptable."),
            3 => Some("Your new password is not acceptable."),
            4 => Some("No account with that ID exists."),
            5 => Some("Incorrect password for that account ID."),
            6 => Some("Your password was changed successfully."),
            _ => None,
        }
    }

    fn ban_message(prefix: &str, info: &BanInfo) -> String {
        let duration_text = match Self::ban_duration_components(info.expiry_date) {
            Some((hours, minutes, seconds)) => format!(
                "Duration remaining: {} hours, {} minutes, {} seconds",
                hours, minutes, seconds
            ),
            None => "Duration remaining: expired or unspecified".to_string(),
        };
        format!(
            "{} ban active. Reason: {}. {}. Expiry ticks: {}.",
            prefix, info.reason, duration_text, info.expiry_date
        )
    }

    fn ban_duration_components(expiry_ticks: i64) -> Option<(i64, i64, i64)> {
        const UNIX_EPOCH_TICKS: i64 = 621_355_968_000_000_000;
        let ticks_since_epoch = expiry_ticks.checked_sub(UNIX_EPOCH_TICKS)?;
        let seconds = ticks_since_epoch / 10_000_000;
        let remainder_ticks = ticks_since_epoch % 10_000_000;
        let seconds = u64::try_from(seconds).ok()?;
        let mut duration = std::time::Duration::from_secs(seconds);
        if remainder_ticks > 0 {
            let nanos = u64::try_from(remainder_ticks * 100).ok()?;
            duration += std::time::Duration::from_nanos(nanos);
        }
        let expiry_time = std::time::SystemTime::UNIX_EPOCH + duration;
        let now = std::time::SystemTime::now();
        let remaining = expiry_time.duration_since(now).ok()?;
        let total_seconds = remaining.as_secs() as i64;
        let hours = total_seconds / 3600;
        let minutes = (total_seconds % 3600) / 60;
        let seconds = total_seconds % 60;
        Some((hours, minutes, seconds))
    }
    
    /// Attempt to connect to server
    pub fn connect_to_server(&mut self) {
        self.connecting = true;
        self.connect_attempts += 1;
        self.login_enabled = false;
        self.version_checked = false;
        self.version_valid = false;
        self.ready_for_character_select = false;
        
        // TODO: Actual network connection
        let status = format!(
            "Attempting to connect to server (attempt {})",
            self.connect_attempts
        );
        println!("{}", status);
        self.record_status(status);
    }
    
    /// Submit login credentials
    pub fn submit_login(&mut self) {
        if self.username.is_empty() || self.password.is_empty() {
            let status = "Username and password required";
            println!("{}", status);
            self.record_status(status);
            return;
        }
        
        // TODO: Send login packet
        self.connecting = true;
        self.login_enabled = false;
        self.ready_for_character_select = false;
        self.last_login_result = None;
        self.require_password_change = false;
        let status = format!("Submitting login for {}", self.username);
        println!("{}", status);
        self.record_status(status);
    }
    
    /// Open new account dialog
    pub fn open_new_account_dialog(&mut self) {
        let status = "Opening new account dialog";
        println!("{}", status);
        self.record_status(status);
        // TODO: Show new account dialog
    }
    
    /// Open change password dialog
    pub fn open_change_password_dialog(&mut self) {
        let status = "Opening change password dialog";
        println!("{}", status);
        self.record_status(status);
        // TODO: Show change password dialog
    }
}

impl Default for LoginScene {
    fn default() -> Self {
        Self::new()
    }
}

impl Scene for LoginScene {
    fn scene_type(&self) -> SceneType {
        SceneType::Login
    }
    
    fn initialize(&mut self) {
        println!("LoginScene::initialize");
        // TODO: Load login UI
        // TODO: Play intro music
        self.connect_to_server();
    }
    
    fn update(&mut self, _delta_time: f32) {
        // TODO: Update connection status
        // TODO: Update animations
    }
    
    fn draw(&self) {
        // TODO: Draw login background
        // TODO: Draw login dialogs
        // TODO: Draw version info
    }
    
    fn process_event(&mut self, event: &GameEvent) {
        match event {
            GameEvent::Connected => {
                let status = "Connected to server!";
                println!("{}", status);
                self.connecting = false;
                self.record_status(status);
            }
            GameEvent::Disconnected { reason } => {
                let status = format!("Disconnected: {}", reason);
                println!("{}", status);
                self.connecting = false;
                self.login_enabled = false;
                self.ready_for_character_select = false;
                self.record_status(status);
            }
            GameEvent::SystemMessage { message } => {
                println!("System: {}", message);
                self.record_status(message.clone());
                // TODO: Display in UI
            }
            GameEvent::ClientVersionResponse { result } => {
                self.handle_client_version_response(*result);
            }
            GameEvent::LoginResponse { result } => {
                self.handle_login_response(*result);
            }
            GameEvent::LoginBanned { reason, expiry_date } => {
                self.handle_login_ban(reason, *expiry_date);
            }
            GameEvent::LoginSuccess { characters } => {
                self.handle_login_success(characters);
            }
            GameEvent::NewAccountResponse { result } => {
                self.handle_new_account_response(*result);
            }
            GameEvent::ChangePasswordResponse { result } => {
                self.handle_change_password_response(*result);
            }
            GameEvent::ChangePasswordBanned { reason, expiry_date } => {
                self.handle_change_password_ban(reason, *expiry_date);
            }
            _ => {
                // Ignore other events in login scene
            }
        }
    }
    
    fn on_mouse_move(&mut self, _x: i32, _y: i32) {
        // TODO: Update hover states
    }
    
    fn on_mouse_click(&mut self, x: i32, y: i32, button: MouseButton) {
        println!("LoginScene click at ({}, {}) with {:?}", x, y, button);
        // TODO: Handle dialog clicks
    }
    
    fn on_key_press(&mut self, key: KeyCode) {
        match key {
            KeyCode::Enter => {
                self.submit_login();
            }
            KeyCode::Escape => {
                // TODO: Close dialog or exit
            }
            _ => {}
        }
    }
    
    fn show(&mut self) {
        println!("LoginScene::show");
        // TODO: Show login UI
        // TODO: Start music
    }
    
    fn hide(&mut self) {
        println!("LoginScene::hide");
        // TODO: Hide login UI
    }
    
    fn dispose(&mut self) {
        println!("LoginScene::dispose");
        // TODO: Cleanup resources
        // TODO: Stop music
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::network::protocol::CharacterSummary;

    #[test]
    fn test_login_scene_creation() {
        let scene = LoginScene::new();
        assert_eq!(scene.scene_type(), SceneType::Login);
        assert!(!scene.connecting);
        assert_eq!(scene.connect_attempts, 0);
        assert!(!scene.version_checked);
        assert!(!scene.login_enabled);
        assert!(scene.message_log.is_empty());
        assert!(scene.characters.is_empty());
    }

    #[test]
    fn test_login_validation() {
        let mut scene = LoginScene::new();
        
        // Empty credentials should fail
        scene.submit_login(); // Should print error
        assert_eq!(scene.message_log.len(), 1);
        assert_eq!(
            scene.last_status.as_deref(),
            Some("Username and password required")
        );
        
        // Set credentials
        scene.username = "testuser".to_string();
        scene.password = "testpass".to_string();
        scene.submit_login(); // Should proceed
        assert_eq!(scene.message_log.len(), 2);
        assert!(scene
            .last_status
            .as_deref()
            .unwrap()
            .contains("Submitting login for"));
        assert!(scene.connecting);
        assert!(!scene.login_enabled);
    }

    #[test]
    fn test_client_version_enables_login() {
        let mut scene = LoginScene::new();
        let event = GameEvent::ClientVersionResponse { result: 1 };
        scene.process_event(&event);
        assert!(scene.version_checked);
        assert!(scene.version_valid);
        assert!(scene.login_enabled);
        assert_eq!(
            scene.last_status.as_deref(),
            Some("Client version accepted by server. Login dialog unlocked.")
        );
    }

    #[test]
    fn test_login_success_populates_characters() {
        let mut scene = LoginScene::new();
        let characters = vec![CharacterSummary {
            index: 1,
            name: "Hero".to_string(),
            level: 10,
            class: 2,
            gender: 1,
            last_access: 0,
        }];
        let event = GameEvent::LoginSuccess {
            characters: characters.clone(),
        };
        scene.process_event(&event);
        assert!(scene.ready_for_character_select);
        assert!(!scene.login_enabled);
        assert_eq!(scene.characters.len(), 1);
        assert_eq!(scene.characters[0].name, "Hero");
    }

    #[test]
    fn test_login_ban_records_status() {
        let mut scene = LoginScene::new();
        let event = GameEvent::LoginBanned {
            reason: "Testing".to_string(),
            expiry_date: 0,
        };
        scene.process_event(&event);
        assert!(scene.login_ban_info.is_some());
        assert!(scene
            .last_status
            .as_deref()
            .unwrap()
            .contains("Login ban active"));
    }
}
