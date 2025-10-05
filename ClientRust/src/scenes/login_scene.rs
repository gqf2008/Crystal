// LoginScene - Login scene implementation
// Mirrors Client/MirScenes/LoginScene.cs

use mir2_shared::packets::CharacterSummary;

use super::{Scene, SceneType};
use crate::network::game_client::GameEvent;

// LoginScene 的内嵌对话框（对应 C# LoginScene 的内嵌类）
pub mod login_dialog;
pub mod new_account_dialog;
pub mod change_password_dialog;

pub use login_dialog::LoginDialog;
pub use new_account_dialog::{NewAccountDialog, NewAccountResult, AccountRegistration};
pub use change_password_dialog::{ChangePasswordDialog, ChangePasswordResult};

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
    
    // Dialogs
    pub login_dialog: LoginDialog,
    pub new_account_dialog: Option<NewAccountDialog>,
    pub change_password_dialog: Option<ChangePasswordDialog>,
}

impl LoginScene {
    /// Create new login scene
    pub fn new() -> Self {
        Self {
            connecting: false,
            connect_attempts: 0,
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
            login_dialog: LoginDialog::default(),
            new_account_dialog: None,
            change_password_dialog: None,
        }
    }
    
    /// Load settings
    pub fn load_settings(&mut self, account_id: String, password: String) {
        self.login_dialog.load_from_settings(account_id, password);
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
        if let Some((username, _password)) = self.login_dialog.get_credentials() {
            // TODO: Send login packet with username and password
            self.connecting = true;
            self.login_enabled = false;
            self.ready_for_character_select = false;
            self.last_login_result = None;
            self.require_password_change = false;
            let status = format!("Submitting login for {}", username);
            println!("{}", status);
            self.record_status(status);
        } else {
            let status = "Username and password required";
            println!("{}", status);
            self.record_status(status);
        }
    }
    
    /// Open new account dialog
    pub fn open_new_account_dialog(&mut self) {
        self.login_dialog.hide();
        let mut dialog = NewAccountDialog::default();
        dialog.show();
        self.new_account_dialog = Some(dialog);
        self.record_status("Opening new account dialog");
    }
    
    /// Open change password dialog
    pub fn open_change_password_dialog(&mut self, autofill_id: Option<String>, autofill_password: Option<String>) {
        self.login_dialog.hide();
        let mut dialog = ChangePasswordDialog::default();
        dialog.show(autofill_id, autofill_password);
        self.change_password_dialog = Some(dialog);
        self.record_status("Opening change password dialog");
    }
    
    /// Close new account dialog
    pub fn close_new_account_dialog(&mut self) {
        self.new_account_dialog = None;
        self.login_dialog.show();
    }
    
    /// Close change password dialog
    pub fn close_change_password_dialog(&mut self) {
        self.change_password_dialog = None;
        self.login_dialog.show();
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
    
    fn handle_mouse_move(&mut self, _x: i32, _y: i32) {
        // TODO: Update hover states
    }
    
    fn handle_mouse_button(&mut self, button: winit::event::MouseButton, pressed: bool, x: i32, y: i32) {
        if pressed {
            tracing::debug!("LoginScene click at ({}, {}) with {:?}", x, y, button);
            // TODO: Handle dialog clicks
        }
    }
    
    fn handle_key_press(&mut self, key: super::KeyCode, _modifiers: super::ModifiersState) -> bool {
        use super::KeyCode;
        
        match key {
            KeyCode::Enter => {
                self.submit_login();
                true
            }
            KeyCode::Escape => {
                // TODO: Close dialog or exit
                true
            }
            _ => false
        }
    }
}
