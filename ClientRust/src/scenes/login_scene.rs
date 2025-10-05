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
    
    fn draw(&self, ctx: &mut ggez::Context, canvas: &mut crate::graphics::Canvas, _ggez_manager: &crate::graphics::GgezManager) {
        use crate::graphics::libraries::{get_library, LibraryName};
        use ggez::graphics::{Text, DrawParam, Color as GgezColor};
        
        // 1. 绘制登录背景 (C# 原版使用 ChrSel.lib 索引 0)
        // ChrSel.lib 索引 0-17 是 1024x768 的登录背景动画 (19帧)
        if let Some(lib_arc) = get_library(LibraryName::ChrSel) {
            if let Ok(mut lib) = lib_arc.try_lock() {
                // 暂时使用静态背景 (索引 0),后续可以实现动画
                let _ = lib.draw_to_canvas(ctx, canvas, 0, 0.0, 0.0, false);
            }
        }
        
        // 2. 绘制登录对话框 (C# 原版: Prguse.lib 索引 1084)
        // 对话框大小: 328x220, 居中显示
        if let Some(lib_arc) = get_library(LibraryName::Prguse) {
            if let Ok(mut lib) = lib_arc.try_lock() {
                let center_x = 1024.0 / 2.0; // 屏幕中心 = 512
                let center_y = 768.0 / 2.0;  // 屏幕中心 = 384
                
                // 登录对话框 (328x220)
                let dialog_x = center_x - 164.0; // 328/2 = 164
                let dialog_y = center_y - 110.0; // 220/2 = 110
                let _ = lib.draw_to_canvas(ctx, canvas, 1084, dialog_x, dialog_y, false);
            }
        }
        
        // 3. 绘制 UI 元素 (C# 原版: Title.lib)
        if let Some(lib_arc) = get_library(LibraryName::Title) {
            if let Ok(mut lib) = lib_arc.try_lock() {
                let center_x = 1024.0 / 2.0;
                let center_y = 768.0 / 2.0;
                let dialog_x = center_x - 164.0;
                let dialog_y = center_y - 110.0;
                
                // 标题 "登录" (索引 30)
                // C# 原版位置: (Size.Width - TitleLabel.Size.Width)/2, 12
                // 假设标题宽度约 100px, 对话框宽 328
                let _ = lib.draw_to_canvas(ctx, canvas, 30, dialog_x + 114.0, dialog_y + 12.0, false);
                
                // "账号ID" 标签 (索引 31)
                // C# 位置: (52, 83)
                let _ = lib.draw_to_canvas(ctx, canvas, 31, dialog_x + 52.0, dialog_y + 83.0, false);
                
                // "密码" 标签 (索引 32)
                // C# 位置: (43, 105)
                let _ = lib.draw_to_canvas(ctx, canvas, 32, dialog_x + 43.0, dialog_y + 105.0, false);
                
                // OK/登录按钮 (索引 320, 大小 42x42)
                // C# 位置: (227, 81)
                let _ = lib.draw_to_canvas(ctx, canvas, 320, dialog_x + 227.0, dialog_y + 81.0, false);
                
                // "新建账号" 按钮 (索引 323)
                // C# 位置: (60, 163)
                let _ = lib.draw_to_canvas(ctx, canvas, 323, dialog_x + 60.0, dialog_y + 163.0, false);
                
                // "修改密码" 按钮 (索引 326)
                // C# 位置: (166, 163)
                let _ = lib.draw_to_canvas(ctx, canvas, 326, dialog_x + 166.0, dialog_y + 163.0, false);
                
                // "关闭" 按钮 (索引 329)
                // C# 位置: (166, 189)
                let _ = lib.draw_to_canvas(ctx, canvas, 329, dialog_x + 166.0, dialog_y + 189.0, false);
            }
        }
        
        // 3. 绘制文本信息
        // 版本信息
        let version_text = Text::new("Crystal v1.0 - Ggez Edition");
        let version_params = DrawParam::default()
            .dest([10.0, 10.0])
            .color(GgezColor::from_rgb(200, 200, 255));
        canvas.draw(&version_text, version_params);
        
        // 连接状态
        if self.connecting {
            let status_text = Text::new(format!("正在连接服务器... (尝试 {})", self.connect_attempts));
            let status_params = DrawParam::default()
                .dest([10.0, 740.0]) // 底部
                .color(GgezColor::from_rgb(255, 255, 100));
            canvas.draw(&status_text, status_params);
        } else if let Some(status) = &self.last_status {
            let status_text = Text::new(status.as_str());
            let status_params = DrawParam::default()
                .dest([10.0, 740.0])
                .color(GgezColor::from_rgb(100, 255, 100));
            canvas.draw(&status_text, status_params);
        }
        
        // FPS 和调试信息 (可选)
        let fps = ctx.time.fps();
        let debug_text = Text::new(format!("FPS: {:.1}", fps));
        let debug_params = DrawParam::default()
            .dest([950.0, 10.0])
            .color(GgezColor::from_rgb(255, 255, 255));
        canvas.draw(&debug_text, debug_params);
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
    
    fn handle_mouse_button(&mut self, button: super::MouseButton, pressed: bool, x: i32, y: i32) {
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
