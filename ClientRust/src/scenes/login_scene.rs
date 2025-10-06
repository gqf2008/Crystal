// LoginScene - Login scene implementation
// Mirrors Client/MirScenes/LoginScene.cs

use mir2_shared::packets::CharacterSummary;

use super::{Scene, SceneType};
use crate::network::game_client::GameEvent;

// LoginScene 的内嵌对话框（对应 C# LoginScene 的内嵌类）
pub mod login_dialog;
pub mod new_account_dialog;
pub mod change_password_dialog;
pub mod message_box;

pub use login_dialog::LoginDialog;
pub use new_account_dialog::{NewAccountDialog, NewAccountResult, AccountRegistration};
pub use change_password_dialog::{ChangePasswordDialog, ChangePasswordResult};
pub use message_box::MessageBox;

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
    
    // Animation state
    pub background_frame: usize,
    pub animation_timer: f32,
    pub animation_paused: bool,  // 动画暂停标志
    
    // Button hover states
    pub ok_button_hovered: bool,
    pub account_button_hovered: bool,
    pub pass_button_hovered: bool,
    pub close_button_hovered: bool,
    
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
    pub message_box: Option<MessageBox>,
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
            background_frame: 0,
            animation_timer: 0.0,
            animation_paused: false,
            ok_button_hovered: false,
            account_button_hovered: false,
            pass_button_hovered: false,
            close_button_hovered: false,
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
            message_box: None,
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
    
    /// Show a message box
    pub fn show_message<S: Into<String>>(&mut self, message: S) {
        let mut msg_box = MessageBox::new(message.into());
        msg_box.show();
        self.message_box = Some(msg_box);
    }
    
    /// Show a message box with custom title
    pub fn show_message_with_title<S: Into<String>, T: Into<String>>(&mut self, message: S, title: T) {
        let mut msg_box = MessageBox::with_title(message.into(), title.into());
        msg_box.show();
        self.message_box = Some(msg_box);
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
            // 显示错误消息框（除了成功状态）
            if result != 0 {
                self.show_message(message);
            }
        } else {
            let msg = format!("Unknown login result code {}", result);
            self.record_status(msg.clone());
            self.show_message(msg);
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
    
    /// 绘制登录输入框的文本和光标
    fn draw_login_input(&self, _ctx: &mut ggez::Context, canvas: &mut crate::graphics::Canvas) {
        use ggez::graphics::{Text, DrawParam, Color as GgezColor};
        
        let center_x = 1024.0 / 2.0;
        let center_y = 768.0 / 2.0;
        let dialog_x = center_x - 164.0;
        let dialog_y = center_y - 110.0;
        
        // 账号输入框文本位置 (C# 原版: AccountIDTextBox.Location = (85, 85))
        let account_text_x = dialog_x + 85.0;
        let account_text_y = dialog_y + 85.0;
        
        // 密码输入框文本位置 (C# 原版: PasswordTextBox.Location = (85, 108))
        let password_text_x = dialog_x + 85.0;
        let password_text_y = dialog_y + 108.0;
        
        // 绘制账号文本
        if !self.login_dialog.account_id.is_empty() {
            let account_text = Text::new(&self.login_dialog.account_id);
            let account_params = DrawParam::default()
                .dest([account_text_x, account_text_y])
                .color(GgezColor::from_rgb(255, 255, 255));
            canvas.draw(&account_text, account_params);
        }
        
        // 绘制密码文本 (用 * 替代)
        if !self.login_dialog.password.is_empty() {
            let password_masked = "*".repeat(self.login_dialog.password.len());
            let password_text = Text::new(&password_masked);
            let password_params = DrawParam::default()
                .dest([password_text_x, password_text_y])
                .color(GgezColor::from_rgb(255, 255, 255));
            canvas.draw(&password_text, password_params);
        }
        
        // 绘制光标
        if self.login_dialog.cursor_visible {
            let cursor_text = Text::new("|");
            let cursor_color = GgezColor::from_rgb(255, 255, 255);
            
            if self.login_dialog.account_focused {
                // 账号输入框光标 (使用6像素每字符，更接近实际字体宽度)
                let cursor_x = account_text_x + (self.login_dialog.account_id.len() as f32 * 6.0);
                let cursor_params = DrawParam::default()
                    .dest([cursor_x, account_text_y])
                    .color(cursor_color);
                canvas.draw(&cursor_text, cursor_params);
            } else if self.login_dialog.password_focused {
                // 密码输入框光标 (密码用*替代，每个*也是6像素宽)
                let cursor_x = password_text_x + (self.login_dialog.password.len() as f32 * 6.0);
                let cursor_params = DrawParam::default()
                    .dest([cursor_x, password_text_y])
                    .color(cursor_color);
                canvas.draw(&cursor_text, cursor_params);
            }
        }
    }
    
    /// 绘制消息框
    fn draw_message_box(&self, ctx: &mut ggez::Context, canvas: &mut crate::graphics::Canvas, msg_box: &MessageBox) {
        use ggez::graphics::{Text, DrawParam, Color as GgezColor, Mesh, DrawMode, Rect};
        use crate::graphics::{get_library, LibraryName};
        
        // 1. 绘制半透明背景遮罩 (让用户聚焦在消息框上)
        if let Ok(bg_mesh) = Mesh::new_rectangle(
            ctx,
            DrawMode::fill(),
            Rect::new(0.0, 0.0, 1024.0, 768.0),
            GgezColor::from_rgba(0, 0, 0, 128),  // 半透明黑色
        ) {
            canvas.draw(&bg_mesh, DrawParam::default());
        }
        
        // 2. 获取 Prguse 库以绘制消息框背景
        // 对应 C# MirMessageBox: Index = 360, Library = Libraries.Prguse
        if let Some(lib_arc) = get_library(LibraryName::Prguse) {
            if let Ok(mut lib) = lib_arc.try_lock() {
                let msg_box_index = 360;
                
                // 获取消息框背景图片尺寸
                let (box_width, box_height) = if let Ok(info) = lib.get_image_info(msg_box_index) {
                    (info.width as f32, info.height as f32)
                } else {
                    tracing::warn!("无法获取消息框背景图片 (索引 360)");
                    (460.0, 200.0)  // 默认尺寸
                };
                
                // 消息框位置 (居中显示，对应 C# Location = new Point((Settings.ScreenWidth - Size.Width) / 2, ...))
                let box_x = (1024.0 - box_width) / 2.0;
                let box_y = (768.0 - box_height) / 2.0;
                
                // 绘制消息框背景图片 (对应 C# DrawImage = true)
                let _ = lib.draw_to_canvas(ctx, canvas, msg_box_index, box_x, box_y, false);
                
                // 3. 绘制消息内容 (对应 C# Label: Location = new Point(35, 35), Size = new Size(390, 110))
                let text_x = box_x + 35.0;
                let text_y = box_y + 35.0;
                let message_lines: Vec<&str> = msg_box.message.lines().collect();
                for (i, line) in message_lines.iter().enumerate() {
                    let line_text = Text::new(*line);
                    let line_params = DrawParam::default()
                        .dest([text_x, text_y + (i as f32 * 20.0)])
                        .color(GgezColor::WHITE);
                    canvas.draw(&line_text, line_params);
                }
                
                // 4. 绘制 OK 按钮 (对应 C# OKButton: Location = new Point(360, 157))
                // Index = 200 (Normal), HoverIndex = 201, PressedIndex = 202, Library = Libraries.Title
                if let Some(title_arc) = get_library(LibraryName::Title) {
                    if let Ok(mut title_lib) = title_arc.try_lock() {
                        let button_index = if msg_box.ok_button_hovered { 201 } else { 200 };
                        let button_x = box_x + 360.0;
                        let button_y = box_y + 157.0;
                        let _ = title_lib.draw_to_canvas(ctx, canvas, button_index, button_x, button_y, false);
                    }
                }
            }
        }
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
        
        // 显示登录对话框
        self.login_dialog.show();
        
        // TODO: Play intro music
        self.connect_to_server();
    }
    
    fn update(&mut self, delta_time: f32) {
        // 更新背景动画 (C# 原版: AnimationCount=19, AnimationDelay=100ms)
        // 如果没有暂停，则更新动画
        if !self.animation_paused {
            self.animation_timer += delta_time;
            if self.animation_timer >= 0.1 {  // 100ms per frame
                self.animation_timer = 0.0;
                self.background_frame = (self.background_frame + 1) % 19;  // 19 frames loop
            }
        }
        
        // 更新登录对话框 (光标闪烁)
        if self.login_dialog.visible {
            self.login_dialog.update(delta_time);
        }
        
        // 更新新建账号对话框
        if let Some(dialog) = &mut self.new_account_dialog {
            if dialog.visible {
                dialog.update(delta_time);
            }
        }
        
        // 更新修改密码对话框
        if let Some(dialog) = &mut self.change_password_dialog {
            if dialog.visible {
                dialog.update(delta_time);
            }
        }
        
        // 更新消息框 (自动关闭计时器)
        if let Some(msg_box) = &mut self.message_box {
            if msg_box.update(delta_time) {
                // 自动关闭
                self.message_box = None;
            }
        }
    }
    
    fn draw(&self, ctx: &mut ggez::Context, canvas: &mut crate::graphics::Canvas, _ggez_manager: &crate::graphics::GgezManager) {
        use crate::graphics::libraries::{get_library, LibraryName};
        use ggez::graphics::{Text, DrawParam, Color as GgezColor};
        
        // 1. 绘制登录背景动画 (C# 原版: ChrSel.lib 索引 0-18, 共19帧)
        if let Some(lib_arc) = get_library(LibraryName::ChrSel) {
            if let Ok(mut lib) = lib_arc.try_lock() {
                // 使用动画帧索引 (0-18)
                let frame_index = self.background_frame.min(18);
                let _ = lib.draw_to_canvas(ctx, canvas, frame_index, 0.0, 0.0, false);
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
                
                // OK/登录按钮 (索引 320/321/322 = normal/hover/pressed)
                // C# 位置: (227, 81), 大小: 42x42
                let ok_index = if self.ok_button_hovered { 321 } else { 320 };
                let _ = lib.draw_to_canvas(ctx, canvas, ok_index, dialog_x + 227.0, dialog_y + 81.0, false);
                
                // "新建账号" 按钮 (索引 323/324/325)
                // C# 位置: (60, 163)
                let account_index = if self.account_button_hovered { 324 } else { 323 };
                let _ = lib.draw_to_canvas(ctx, canvas, account_index, dialog_x + 60.0, dialog_y + 163.0, false);
                
                // "修改密码" 按钮 (索引 326/327/328)
                // C# 位置: (166, 163)
                let pass_index = if self.pass_button_hovered { 327 } else { 326 };
                let _ = lib.draw_to_canvas(ctx, canvas, pass_index, dialog_x + 166.0, dialog_y + 163.0, false);
                
                // "关闭" 按钮 (索引 329/330/331)
                // C# 位置: (166, 189)
                let close_index = if self.close_button_hovered { 330 } else { 329 };
                let _ = lib.draw_to_canvas(ctx, canvas, close_index, dialog_x + 166.0, dialog_y + 189.0, false);
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
        
        // 3.5 绘制登录对话框的输入框文本和光标
        if self.login_dialog.visible {
            self.draw_login_input(ctx, canvas);
        }
        
        // 4. 绘制消息框 (MessageBox) - 最后绘制，显示在最上层
        if let Some(msg_box) = &self.message_box {
            if msg_box.visible {
                self.draw_message_box(ctx, canvas, msg_box);
            }
        }
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
    
    fn handle_mouse_move(&mut self, x: i32, y: i32) {
        // 优先处理 MessageBox 悬停
        if let Some(msg_box) = &mut self.message_box {
            if msg_box.visible {
                use crate::graphics::{get_library, LibraryName};
                let (box_width, box_height) = if let Some(lib_arc) = get_library(LibraryName::Prguse) {
                    if let Ok(mut lib) = lib_arc.try_lock() {
                        if let Ok(info) = lib.get_image_info(360) {
                            (info.width as f32, info.height as f32)
                        } else {
                            (460.0, 200.0)
                        }
                    } else {
                        (460.0, 200.0)
                    }
                } else {
                    (460.0, 200.0)
                };
                
                let fx = x as f32;
                let fy = y as f32;
                let box_x = (1024.0 - box_width) / 2.0;
                let box_y = (768.0 - box_height) / 2.0;
                let button_x = box_x + 360.0;
                let button_y = box_y + 157.0;
                
                let (button_w, button_h) = if let Some(title_arc) = get_library(LibraryName::Title) {
                    if let Ok(mut title_lib) = title_arc.try_lock() {
                        if let Ok(info) = title_lib.get_image_info(200) {
                            (info.width as f32, info.height as f32)
                        } else {
                            (42.0, 42.0)
                        }
                    } else {
                        (42.0, 42.0)
                    }
                } else {
                    (42.0, 42.0)
                };
                
                let hovered = fx >= button_x && fx < button_x + button_w 
                           && fy >= button_y && fy < button_y + button_h;
                
                // 调试信息:只在悬停状态变化时打印
                if hovered != msg_box.ok_button_hovered {
                    tracing::debug!(
                        "OK button hover changed: {} -> {} | Mouse: ({:.1}, {:.1}) | Button: ({:.1}, {:.1}) size: ({:.1}, {:.1})",
                        msg_box.ok_button_hovered, hovered, fx, fy, button_x, button_y, button_w, button_h
                    );
                }
                
                msg_box.ok_button_hovered = hovered;
                return; // MessageBox 阻止其他交互
            }
        }
        
        if self.login_dialog.visible {
            let center_x = 1024.0 / 2.0;
            let center_y = 768.0 / 2.0;
            let dialog_x = center_x - 164.0;
            let dialog_y = center_y - 110.0;
            
            let fx = x as f32;
            let fy = y as f32;
            
            // OK 按钮区域: (227, 81, 42, 42)
            let ok_btn_x = dialog_x + 227.0;
            let ok_btn_y = dialog_y + 81.0;
            self.ok_button_hovered = fx >= ok_btn_x && fx <= ok_btn_x + 42.0
                                  && fy >= ok_btn_y && fy <= ok_btn_y + 42.0;
            
            // 新建账号按钮区域 - 需要从图像获取实际尺寸，暂时假设 ~100x30
            let acc_btn_x = dialog_x + 60.0;
            let acc_btn_y = dialog_y + 163.0;
            self.account_button_hovered = fx >= acc_btn_x && fx <= acc_btn_x + 100.0
                                       && fy >= acc_btn_y && fy <= acc_btn_y + 30.0;
            
            // 修改密码按钮区域
            let pass_btn_x = dialog_x + 166.0;
            let pass_btn_y = dialog_y + 163.0;
            self.pass_button_hovered = fx >= pass_btn_x && fx <= pass_btn_x + 100.0
                                    && fy >= pass_btn_y && fy <= pass_btn_y + 30.0;
            
            // 关闭按钮区域
            let close_btn_x = dialog_x + 166.0;
            let close_btn_y = dialog_y + 189.0;
            self.close_button_hovered = fx >= close_btn_x && fx <= close_btn_x + 100.0
                                     && fy >= close_btn_y && fy <= close_btn_y + 30.0;
        }
    }
    
    fn handle_mouse_button(&mut self, button: super::MouseButton, pressed: bool, x: i32, y: i32) {
        if pressed && button == super::MouseButton::Left {
            // 优先处理 MessageBox (如果显示)
            if let Some(msg_box) = &mut self.message_box {
                if msg_box.visible {
                    // 获取消息框实际尺寸 (对应 C# Index=360 的图片尺寸)
                    // 使用与绘制时相同的计算方式
                    use crate::graphics::{get_library, LibraryName};
                    let (box_width, box_height) = if let Some(lib_arc) = get_library(LibraryName::Prguse) {
                        if let Ok(mut lib) = lib_arc.try_lock() {
                            if let Ok(info) = lib.get_image_info(360) {
                                (info.width as f32, info.height as f32)
                            } else {
                                (460.0, 200.0) // 默认值
                            }
                        } else {
                            (460.0, 200.0)
                        }
                    } else {
                        (460.0, 200.0)
                    };
                    
                    let box_x = (1024.0 - box_width) / 2.0;
                    let box_y = (768.0 - box_height) / 2.0;
                    
                    // OK按钮位置: (360, 157) 相对于消息框
                    let button_x = box_x + 360.0;
                    let button_y = box_y + 157.0;
                    
                    // 获取按钮尺寸 (Title库索引200)
                    let (button_w, button_h) = if let Some(title_arc) = get_library(LibraryName::Title) {
                        if let Ok(mut title_lib) = title_arc.try_lock() {
                            if let Ok(info) = title_lib.get_image_info(200) {
                                (info.width as f32, info.height as f32)
                            } else {
                                (42.0, 42.0) // C# 按钮默认尺寸
                            }
                        } else {
                            (42.0, 42.0)
                        }
                    } else {
                        (42.0, 42.0)
                    };
                    
                    let fx = x as f32;
                    let fy = y as f32;
                    
                    // 检查是否点击了OK按钮
                    if fx >= button_x && fx < button_x + button_w && fy >= button_y && fy < button_y + button_h {
                        tracing::debug!("MessageBox OK button clicked");
                        self.message_box = None;
                    }
                    return; // MessageBox 阻止其他交互
                }
            }
            
            tracing::debug!("LoginScene click at ({}, {}) with {:?}", x, y, button);
            
            // 处理登录对话框点击
            if self.login_dialog.visible {
                let center_x = 1024.0 / 2.0;
                let center_y = 768.0 / 2.0;
                let dialog_x = center_x - 164.0;
                let dialog_y = center_y - 110.0;
                
                // 账号输入框区域: (85, 85, 136, 15) - C# 原版坐标
                let account_box_x = dialog_x + 85.0;
                let account_box_y = dialog_y + 85.0;
                let account_box_w = 136.0;
                let account_box_h = 15.0;
                
                // 密码输入框区域: (85, 108, 136, 15) - C# 原版坐标
                let password_box_x = dialog_x + 85.0;
                let password_box_y = dialog_y + 108.0;
                let password_box_w = 136.0;
                let password_box_h = 15.0;
                
                let fx = x as f32;
                let fy = y as f32;
                
                // 检查是否点击了账号输入框
                if fx >= account_box_x && fx <= account_box_x + account_box_w
                    && fy >= account_box_y && fy <= account_box_y + account_box_h {
                    self.login_dialog.account_focused = true;
                    self.login_dialog.password_focused = false;
                    tracing::debug!("Account input box focused");
                }
                // 检查是否点击了密码输入框
                else if fx >= password_box_x && fx <= password_box_x + password_box_w
                    && fy >= password_box_y && fy <= password_box_y + password_box_h {
                    self.login_dialog.account_focused = false;
                    self.login_dialog.password_focused = true;
                    tracing::debug!("Password input box focused");
                }
            }
        }
    }
    
    fn handle_key_press(&mut self, key: super::KeyCode, _modifiers: super::ModifiersState) -> bool {
        use super::KeyCode;
        
        // 优先处理 MessageBox
        if self.message_box.is_some() {
            match key {
                KeyCode::Escape => {
                    self.message_box = None;
                    return true;
                }
                _ => return true, // MessageBox 显示时阻止其他按键
            }
        }
        
        // 处理 LoginDialog 按键
        if self.login_dialog.visible {
            match key {
                KeyCode::Enter => {
                    self.submit_login();
                    return true;
                }
                KeyCode::Tab => {
                    self.login_dialog.handle_tab();
                    return true;
                }
                KeyCode::Backspace => {
                    self.login_dialog.handle_backspace();
                    return true;
                }
                KeyCode::KeyM => {
                    // 测试：按 M 键显示消息框
                    self.show_message("这是一个测试消息框!\n\n您可以点击 OK 按钮关闭它。\n或按 ESC 键关闭。");
                    return true;
                }
                KeyCode::Space => {
                    // 空格键：暂停/播放背景动画
                    self.animation_paused = !self.animation_paused;
                    let status = if self.animation_paused {
                        "背景动画已暂停 (再按空格继续)"
                    } else {
                        "背景动画已恢复播放"
                    };
                    tracing::debug!("{}", status);
                    return true;
                }
                _ => {}
            }
        }
        
        // TODO: 处理其他对话框的按键
        
        false
    }
    
    fn handle_text_input(&mut self, character: char) {
        // MessageBox 显示时不处理文本输入
        if self.message_box.is_some() {
            return;
        }
        
        // 处理 LoginDialog 文本输入
        if self.login_dialog.visible {
            self.login_dialog.handle_text_input(character);
        }
        
        // TODO: 处理其他对话框的文本输入
    }
}
