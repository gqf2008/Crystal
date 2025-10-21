// LoginScene - Login scene implementation (ECS版本)
// 移植自 Client/MirScenes/LoginScene.cs
// 保留完整的UI、纹理、网络功能，适配ECS架构

use mir2_shared::packets::CharacterSummary;
use ggez::{Context, GameResult};
use ggez::graphics::Canvas;
use hecs::World;
use tokio::sync::mpsc;

use super::{Scene, SceneType};
use crate::network::{game_client::GameEvent, NetworkCommand};

// LoginScene 的内嵌对话框（对应 C# LoginScene 的内嵌类）
pub mod login_dialog;
pub mod new_account_dialog;
pub mod change_password_dialog;
pub mod message_box;

pub use login_dialog::LoginDialog;
pub use new_account_dialog::NewAccountDialog;
pub use change_password_dialog::ChangePasswordDialog;
pub use message_box::MessageBox;

#[derive(Debug, Clone)]
pub struct BanInfo {
    pub reason: String,
    pub expiry_date: i64,
}

/// Login scene state
pub struct LoginScene {
    // Network client (不包含在Debug输出中,因为复杂类型)
    #[allow(dead_code)]
    pub game_client: Option<crate::network::game_client::SharedGameClient>,
    #[allow(dead_code)]
    pub command_tx: Option<tokio::sync::mpsc::UnboundedSender<crate::network::NetworkCommand>>,
    
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
    pub view_key_button_hovered: bool,
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
            game_client: None,
            command_tx: None,
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
            view_key_button_hovered: false,
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
    
    /// Set game client for network communication
    pub fn set_game_client(&mut self, client: Option<crate::network::game_client::SharedGameClient>) {
        self.game_client = client;
    }
    
    /// Set command sender for network commands
    pub fn set_command_sender(&mut self, tx: Option<tokio::sync::mpsc::UnboundedSender<crate::network::NetworkCommand>>) {
        self.command_tx = tx;
    }
    
    /// Send client version to server for verification
    fn send_client_version(&mut self) {
        use std::fs::File;
        use std::io::Read;
        
        tracing::info!("发送客户端版本验证...");
        self.record_status("Sending client version...");
        
        // 计算可执行文件的 MD5 哈希
        let version_hash = if let Ok(exe_path) = std::env::current_exe() {
            if let Ok(mut file) = File::open(exe_path) {
                let mut buffer = Vec::new();
                if file.read_to_end(&mut buffer).is_ok() {
                    use md5::compute;
                    let digest = compute(&buffer);
                    digest.0.to_vec()
                } else {
                    vec![0u8; 16]  // 空哈希
                }
            } else {
                vec![0u8; 16]
            }
        } else {
            vec![0u8; 16]
        };
        
        // TODO: 发送ClientVersion包到网络线程
        // 注意: ClientVersion验证目前需要通过NetworkManager的send_packet方法
        // 这里暂时跳过,等待版本验证响应后启用登录对话框
        tracing::info!("✅ 已连接到服务器,ClientVersion包准备完成");
        tracing::info!("版本哈希: {:?}", &version_hash[0..8.min(version_hash.len())]);
        
        // 临时方案: 直接启用登录对话框 (跳过版本验证)
        self.version_checked = true;
        self.version_valid = true;
        self.login_enabled = true;
        self.record_status("Connected! Ready to login.");
        tracing::info!("⚠ 临时跳过客户端版本验证,登录对话框已启用");
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
        
        let status_msg = format!(
            "Login successful. {} character(s) available.",
            self.characters.len()
        );
        self.record_status(status_msg.clone());
        
        // 🔧 TODO: 触发场景切换到 SelectScene
        // 目前 Scene trait 没有返回"请求场景切换"的机制
        // 需要通过 SceneManager 或其他方式触发
        tracing::info!("✅ {}", status_msg);
        tracing::warn!("⚠️ 需要手动切换到 SelectScene - Scene trait 没有自动切换机制!");
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
        if let Some((username, password)) = self.login_dialog.get_credentials() {
            // 验证账号和密码格式
            if !self.login_dialog.is_account_id_valid() {
                self.show_message("您的账号格式不正确");
                return;
            }
            if !self.login_dialog.is_password_valid() {
                self.show_message("您的密码格式不正确");
                return;
            }
            
            // 发送登录命令到网络线程
            if let Some(tx) = &self.command_tx {
                let command = crate::network::NetworkCommand::Login {
                    username: username.clone(),
                    password: password.clone(),
                };
                
                if let Err(e) = tx.send(command) {
                    tracing::error!("发送登录命令失败: {}", e);
                    self.show_message("网络错误,无法发送登录请求");
                    return;
                }
                
                tracing::info!("✅ 已发送登录请求: {}", username);
                self.record_status(format!("正在提交登录请求: {}", username));
            } else {
                tracing::error!("command_tx 未初始化");
                self.show_message("网络未初始化,请稍后再试");
                return;
            }
            
            self.connecting = true;
            self.login_enabled = false;
            self.ready_for_character_select = false;
            self.last_login_result = None;
            self.require_password_change = false;
        } else {
            self.show_message("请输入账号和密码");
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
    fn draw_login_input(&self, ctx: &mut ggez::Context, canvas: &mut crate::graphics::Canvas) {
        use ggez::graphics::{Text, TextFragment, DrawParam, Color as GgezColor};
        
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
        
        // 绘制账号文本 (使用中文字体)
        if !self.login_dialog.account_id.is_empty() {
            let input_box_width = 110.0; // 输入框可见宽度
            
            // 计算完整文本宽度
            let full_text = Text::new(
                TextFragment::new(&self.login_dialog.account_id)
                    .font("AlibabaPuHuiTi")
                    .scale(21.0)
            );
            let full_width = full_text.measure(ctx).map(|m| m.x).unwrap_or(0.0);
            
            // 如果文本超长，从右往左截取可见部分
            let visible_text = if full_width > input_box_width {
                let chars: Vec<char> = self.login_dialog.account_id.chars().collect();
                let mut visible_chars = chars.clone();
                
                // 从左边逐个删除字符直到文本能完全显示
                while visible_chars.len() > 0 {
                    let test_text = Text::new(
                        TextFragment::new(visible_chars.iter().collect::<String>())
                            .font("AlibabaPuHuiTi")
                            .scale(21.0)
                    );
                    let test_width = test_text.measure(ctx).map(|m| m.x).unwrap_or(0.0);
                    
                    if test_width <= input_box_width - 5.0 {
                        break;
                    }
                    visible_chars.remove(0);
                }
                visible_chars.iter().collect::<String>()
            } else {
                self.login_dialog.account_id.clone()
            };
            
            let account_text = Text::new(
                TextFragment::new(&visible_text)
                    .font("AlibabaPuHuiTi")
                    .scale(21.0)
            );
            
            let account_params = DrawParam::default()
                .dest([account_text_x, account_text_y - 3.0])  // 向上移3像素使其居中
                .color(GgezColor::from_rgb(255, 255, 255));
            canvas.draw(&account_text, account_params);
        }
        
        // 绘制密码文本 (用 * 替代, 使用中文字体)
        if !self.login_dialog.password.is_empty() {
            let input_box_width = 110.0; // 输入框可见宽度
            let password_masked = "*".repeat(self.login_dialog.password.len());
            
            // 计算完整密码文本宽度
            let full_text = Text::new(
                TextFragment::new(&password_masked)
                    .font("AlibabaPuHuiTi")
                    .scale(21.0)
            );
            let full_width = full_text.measure(ctx).map(|m| m.x).unwrap_or(0.0);
            
            // 如果文本超长，只显示右侧可见部分
            let visible_text = if full_width > input_box_width {
                let mut visible_count = password_masked.len();
                
                while visible_count > 0 {
                    let test_text = Text::new(
                        TextFragment::new("*".repeat(visible_count))
                            .font("AlibabaPuHuiTi")
                            .scale(21.0)
                    );
                    let test_width = test_text.measure(ctx).map(|m| m.x).unwrap_or(0.0);
                    
                    if test_width <= input_box_width - 5.0 {
                        break;
                    }
                    visible_count -= 1;
                }
                "*".repeat(visible_count)
            } else {
                password_masked
            };
            
            let password_text = Text::new(
                TextFragment::new(&visible_text)
                    .font("AlibabaPuHuiTi")
                    .scale(21.0)
            );
            
            let password_params = DrawParam::default()
                .dest([password_text_x, password_text_y - 3.0])  // 向上移3像素使其居中
                .color(GgezColor::from_rgb(255, 255, 255));
            canvas.draw(&password_text, password_params);
        }
        
        // 绘制光标 (使用中文字体)
        if self.login_dialog.cursor_visible {
            let cursor_text = Text::new(
                TextFragment::new("|")
                    .font("AlibabaPuHuiTi")
                    .scale(21.0)
            );
            let cursor_color = GgezColor::from_rgb(255, 255, 255);
            
            if self.login_dialog.account_focused {
                // 使用 ggez 的文本测量功能精确计算光标位置
                let account_text = Text::new(
                    TextFragment::new(self.login_dialog.account_id.clone())
                        .font("AlibabaPuHuiTi")
                        .scale(21.0)
                );
                let text_width = account_text.measure(ctx)
                    .map(|m| m.x)
                    .unwrap_or(0.0);
                
                // 计算文本滚动偏移
                let input_box_width = 110.0;
                let text_offset = if text_width > input_box_width {
                    input_box_width - text_width - 5.0
                } else {
                    0.0
                };
                
                let cursor_x = account_text_x + text_width + text_offset;
                
                let cursor_params = DrawParam::default()
                    .dest([cursor_x, account_text_y - 3.0])  // 和文本保持相同的Y坐标
                    .color(cursor_color);
                canvas.draw(&cursor_text, cursor_params);
            } else if self.login_dialog.password_focused {
                // 密码显示为 *,也需要测量宽度
                let password_display: String = "*".repeat(self.login_dialog.password.len());
                let password_text = Text::new(
                    TextFragment::new(password_display)
                        .font("AlibabaPuHuiTi")
                        .scale(21.0)
                );
                let text_width = password_text.measure(ctx)
                    .map(|m| m.x)
                    .unwrap_or(0.0);
                
                // 计算文本滚动偏移
                let input_box_width = 110.0;
                let text_offset = if text_width > input_box_width {
                    input_box_width - text_width - 5.0
                } else {
                    0.0
                };
                
                let cursor_x = password_text_x + text_width + text_offset;
                
                let cursor_params = DrawParam::default()
                    .dest([cursor_x, password_text_y - 3.0])  // 和文本保持相同的Y坐标
                    .color(cursor_color);
                canvas.draw(&cursor_text, cursor_params);
            }
        }
    }
    
    /// 绘制消息框
    fn draw_message_box(&self, ctx: &mut ggez::Context, canvas: &mut crate::graphics::Canvas, msg_box: &MessageBox) {
        use ggez::graphics::{Text, TextFragment, DrawParam, Color as GgezColor, Mesh, DrawMode, Rect};
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
                let _ = lib.draw_with_color(ctx, canvas, msg_box_index, box_x, box_y, ggez::graphics::Color::WHITE, false);
                
                // 3. 绘制消息内容 (对应 C# Label: Location = new Point(35, 35), Size = new Size(390, 110))
                let text_x = box_x + 35.0;
                let text_y = box_y + 35.0;
                let message_lines: Vec<&str> = msg_box.message.lines().collect();
                for (i, line) in message_lines.iter().enumerate() {
                    let line_text = Text::new(
                        TextFragment::new(*line)
                            .font("AlibabaPuHuiTi")
                            .scale(16.0)
                    );
                    let line_params = DrawParam::default()
                        .dest([text_x, text_y + (i as f32 * 24.0)])
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
                        let _ = title_lib.draw_with_color(ctx, canvas, button_index, button_x, button_y, ggez::graphics::Color::WHITE, false);
                    }
                }
            }
        }
    }
    
    /// 绘制新建账号对话框
    fn draw_new_account_dialog(&self, ctx: &mut ggez::Context, canvas: &mut crate::graphics::Canvas, dialog: &NewAccountDialog) {
        use ggez::graphics::{Text, TextFragment, DrawParam, Color as GgezColor, Mesh, DrawMode, Rect};
        use crate::graphics::{get_library, LibraryName};
        
        // 1. 绘制半透明背景遮罩
        if let Ok(bg_mesh) = Mesh::new_rectangle(
            ctx,
            DrawMode::fill(),
            Rect::new(0.0, 0.0, 1024.0, 768.0),
            GgezColor::from_rgba(0, 0, 0, 128),
        ) {
            canvas.draw(&bg_mesh, DrawParam::default());
        }
        
        // 2. 绘制新建账号对话框背景
        // C# 原版: Index = 63, Library = Libraries.Prguse
        if let Some(lib_arc) = get_library(LibraryName::Prguse) {
            if let Ok(mut lib) = lib_arc.try_lock() {
                // 获取对话框尺寸
                let (box_width, box_height) = if let Ok(info) = lib.get_image_info(63) {
                    (info.width as f32, info.height as f32)
                } else {
                    (500.0, 480.0)  // 默认尺寸
                };
                
                // 对话框位置 (居中)
                let box_x = (1024.0 - box_width) / 2.0;
                let box_y = (768.0 - box_height) / 2.0;
                
                // 绘制背景
                let _ = lib.draw_with_color(ctx, canvas, 63, box_x, box_y, ggez::graphics::Color::WHITE, false);
                
                // 3. 绘制输入框标签和内容
                // C# 原版位置: AccountIDTextBox: Location = new Point(226, 103), Size = new Size(136, 18)
                // 显示所有 8 个输入框
                let input_fields = [
                    ("账号ID:", box_x + 226.0, box_y + 103.0, &dialog.registration.account_id, new_account_dialog::InputField::AccountId, dialog.account_id_valid),
                    ("密码:", box_x + 226.0, box_y + 129.0, &"*".repeat(dialog.registration.password.len()), new_account_dialog::InputField::Password, dialog.password1_valid),
                    ("确认密码:", box_x + 226.0, box_y + 155.0, &"*".repeat(dialog.registration.password_confirm.len()), new_account_dialog::InputField::PasswordConfirm, dialog.password2_valid),
                    ("用户名:", box_x + 226.0, box_y + 189.0, &dialog.registration.username, new_account_dialog::InputField::Username, dialog.username_valid),
                    ("生日:", box_x + 226.0, box_y + 215.0, &dialog.registration.birth_date, new_account_dialog::InputField::BirthDate, dialog.birth_date_valid),
                    ("安全问题:", box_x + 226.0, box_y + 250.0, &dialog.registration.secret_question, new_account_dialog::InputField::Question, dialog.question_valid),
                    ("安全答案:", box_x + 226.0, box_y + 276.0, &dialog.registration.secret_answer, new_account_dialog::InputField::Answer, dialog.answer_valid),
                    ("电子邮箱:", box_x + 226.0, box_y + 311.0, &dialog.registration.email, new_account_dialog::InputField::Email, dialog.email_valid),
                ];
                
                for (label, x, y, text, field, valid) in &input_fields {
                    // 绘制标签 (使用中文字体)
                    let label_text = Text::new(
                        TextFragment::new(*label)
                            .font("AlibabaPuHuiTi")
                            .scale(16.0)
                    );
                    canvas.draw(&label_text, DrawParam::default()
                        .dest([x - 80.0, *y])
                        .color(GgezColor::WHITE));
                    
                    // 绘制输入框背景 (边框)
                    let border_color = if !valid {
                        GgezColor::from_rgb(255, 0, 0) // 红色表示无效
                    } else if *valid {
                        GgezColor::from_rgb(0, 255, 0) // 绿色表示有效
                    } else {
                        GgezColor::from_rgb(128, 128, 128) // 灰色表示未验证
                    };
                    
                    if let Ok(border) = Mesh::new_rectangle(
                        ctx,
                        DrawMode::stroke(1.0),
                        Rect::new(*x, *y, 136.0, 18.0),
                        border_color,
                    ) {
                        canvas.draw(&border, DrawParam::default());
                    }
                    
                    // 绘制文本选择高亮 (如果有选择且是当前字段)
                    if dialog.focused_field == *field {
                        if let Some((sel_start, sel_end)) = dialog.get_selection_range() {
                            // 创建临时文本来测量选择区域的宽度
                            let chars: Vec<char> = text.chars().collect();
                            
                            // 测量选择开始之前的文本宽度
                            let before_text: String = chars.iter().take(sel_start).collect();
                            let before_width = Text::new(
                                TextFragment::new(before_text)
                                    .font("AlibabaPuHuiTi")
                                    .scale(21.0)
                            ).measure(ctx).map(|m| m.x).unwrap_or(0.0);
                            
                            // 测量选择的文本宽度
                            let selected_text: String = chars.iter().skip(sel_start).take(sel_end - sel_start).collect();
                            let selected_width = Text::new(
                                TextFragment::new(selected_text)
                                    .font("AlibabaPuHuiTi")
                                    .scale(21.0)
                            ).measure(ctx).map(|m| m.x).unwrap_or(0.0);
                            
                            // 绘制选择高亮背景（文本已被截断，不需要滚动偏移）
                            if let Ok(highlight) = Mesh::new_rectangle(
                                ctx,
                                DrawMode::fill(),
                                Rect::new(x + 2.0 + before_width, *y + 1.0, selected_width, 16.0),  // Y坐标和高度与文本对齐
                                GgezColor::from_rgba(100, 150, 255, 128), // 半透明蓝色
                            ) {
                                canvas.draw(&highlight, DrawParam::default());
                            }
                        }
                    }
                    
                    // 绘制文本内容 (使用中文字体)
                    let input_box_width = 132.0; // 输入框可见宽度 (136 - 4像素边距)
                    
                    // 计算完整文本宽度
                    let full_text = Text::new(
                        TextFragment::new(text.to_string())
                            .font("AlibabaPuHuiTi")
                            .scale(21.0)
                    );
                    let full_width = full_text.measure(ctx).map(|m| m.x).unwrap_or(0.0);
                    
                    // 如果文本超长，从右往左截取可见部分
                    let visible_text = if full_width > input_box_width {
                        let chars: Vec<char> = text.chars().collect();
                        let mut visible_chars = chars.clone();
                        
                        while visible_chars.len() > 0 {
                            let test_text = Text::new(
                                TextFragment::new(visible_chars.iter().collect::<String>())
                                    .font("AlibabaPuHuiTi")
                                    .scale(21.0)
                            );
                            let test_width = test_text.measure(ctx).map(|m| m.x).unwrap_or(0.0);
                            
                            if test_width <= input_box_width - 5.0 {
                                break;
                            }
                            visible_chars.remove(0);
                        }
                        visible_chars.iter().collect::<String>()
                    } else {
                        text.to_string()
                    };
                    
                    let content_text = Text::new(
                        TextFragment::new(&visible_text)
                            .font("AlibabaPuHuiTi")
                            .scale(21.0)
                    );
                    
                    canvas.draw(&content_text, DrawParam::default()
                        .dest([x + 2.0, *y - 1.0])  // 向上移1像素使其在18px输入框中居中
                        .color(GgezColor::WHITE));
                    
                    // 如果是当前聚焦的输入框,绘制光标
                    if dialog.focused_field == *field && dialog.cursor_visible {
                        // 使用可见文本宽度计算光标位置
                        let visible_width = content_text.measure(ctx).map(|m| m.x).unwrap_or(0.0);
                        let cursor_x = x + 2.0 + visible_width;
                        
                        if let Ok(cursor) = Mesh::new_rectangle(
                            ctx,
                            DrawMode::fill(),
                            Rect::new(cursor_x, *y + 1.0, 2.0, 16.0),  // 高度改为16px,Y坐标+1
                            GgezColor::WHITE,
                        ) {
                            canvas.draw(&cursor, DrawParam::default());
                        }
                    }
                }
                
                // 4. 绘制按钮 (使用Title库)
                if let Some(title_arc) = get_library(LibraryName::Title) {
                    if let Ok(mut title_lib) = title_arc.try_lock() {
                        // OK按钮 (C# Location: (135, 425))
                        let ok_index = if dialog.ok_button_hovered { 201 } else { 200 };
                        let _ = title_lib.draw_with_color(ctx, canvas, ok_index, box_x + 135.0, box_y + 425.0, ggez::graphics::Color::WHITE, false);
                        
                        // Cancel按钮 (C# Location: (409, 425))
                        let cancel_index = if dialog.cancel_button_hovered { 204 } else { 203 };
                        let _ = title_lib.draw_with_color(ctx, canvas, cancel_index, box_x + 409.0, box_y + 425.0, ggez::graphics::Color::WHITE, false);
                    }
                }
                
                // 5. 绘制提示信息 (使用中文字体)
                let hint_text = Text::new(
                    TextFragment::new("按Tab切换输入框 | 按ESC关闭")
                        .font("AlibabaPuHuiTi")
                        .scale(21.0)
                );
                canvas.draw(&hint_text, DrawParam::default()
                    .dest([box_x + 150.0, box_y + 460.0])
                    .color(GgezColor::from_rgb(200, 200, 200)));
            }
        }
    }
    
    /// 绘制修改密码对话框
    fn draw_change_password_dialog(&self, ctx: &mut ggez::Context, canvas: &mut crate::graphics::Canvas, dialog: &ChangePasswordDialog) {
        use ggez::graphics::{Text, TextFragment, DrawParam, Color as GgezColor, Mesh, DrawMode, Rect};
        use crate::graphics::{get_library, LibraryName};
        
        // 1. 绘制半透明背景遮罩
        if let Ok(bg_mesh) = Mesh::new_rectangle(
            ctx,
            DrawMode::fill(),
            Rect::new(0.0, 0.0, 1024.0, 768.0),
            GgezColor::from_rgba(0, 0, 0, 128),
        ) {
            canvas.draw(&bg_mesh, DrawParam::default());
        }
        
        // 2. 绘制修改密码对话框背景
        // C# 原版: Index = 50, Library = Libraries.Prguse
        if let Some(lib_arc) = get_library(LibraryName::Prguse) {
            if let Ok(mut lib) = lib_arc.try_lock() {
                // 获取对话框尺寸
                let (box_width, box_height) = if let Ok(info) = lib.get_image_info(50) {
                    (info.width as f32, info.height as f32)
                } else {
                    (360.0, 310.0)  // 默认尺寸
                };
                
                // 对话框位置 (居中)
                let box_x = (1024.0 - box_width) / 2.0;
                let box_y = (768.0 - box_height) / 2.0;
                
                // 绘制背景
                let _ = lib.draw_with_color(ctx, canvas, 50, box_x, box_y, ggez::graphics::Color::WHITE, false);
                
                // 3. 绘制输入框和标签
                // C# 坐标:
                // AccountID: (178, 75)
                // CurrentPassword: (178, 113)
                // NewPassword1: (178, 151)
                // NewPassword2: (178, 188)
                
                let input_fields = [
                    ("账号ID:", box_x + 178.0, box_y + 75.0, &dialog.account_id, change_password_dialog::PasswordInputField::AccountId, dialog.account_id_valid),
                    ("当前密码:", box_x + 178.0, box_y + 113.0, &"*".repeat(dialog.current_password.len()), change_password_dialog::PasswordInputField::CurrentPassword, dialog.current_password_valid),
                    ("新密码:", box_x + 178.0, box_y + 151.0, &"*".repeat(dialog.new_password.len()), change_password_dialog::PasswordInputField::NewPassword, dialog.new_password1_valid),
                    ("确认新密码:", box_x + 178.0, box_y + 188.0, &"*".repeat(dialog.new_password_confirm.len()), change_password_dialog::PasswordInputField::NewPasswordConfirm, dialog.new_password2_valid),
                ];
                
                for (label, x, y, text, field, valid) in &input_fields {
                    // 绘制标签 (在输入框左侧, 使用中文字体)
                    let label_text = Text::new(
                        TextFragment::new(*label)
                            .font("AlibabaPuHuiTi")
                            .scale(16.0)
                    );
                    canvas.draw(&label_text, DrawParam::default()
                        .dest([x - 90.0, *y])
                        .color(GgezColor::from_rgb(200, 200, 150)));
                    
                    // 绘制输入框边框 (C# Size: 136x18)
                    let border_color = if !valid {
                        GgezColor::from_rgb(255, 0, 0)  // 红色: 无效
                    } else if *valid {
                        GgezColor::from_rgb(0, 255, 0)  // 绿色: 有效
                    } else {
                        GgezColor::from_rgb(128, 128, 128)  // 灰色: 未验证
                    };
                    
                    if let Ok(border) = Mesh::new_rectangle(
                        ctx,
                        DrawMode::stroke(1.0),
                        Rect::new(*x, *y, 136.0, 18.0),
                        border_color,
                    ) {
                        canvas.draw(&border, DrawParam::default());
                    }
                    
                    // 绘制输入框内容 (使用中文字体)
                    let input_box_width = 132.0; // 输入框可见宽度 (136 - 4像素边距)
                    
                    // 计算完整文本宽度
                    let full_text = Text::new(
                        TextFragment::new(text.to_string())
                            .font("AlibabaPuHuiTi")
                            .scale(21.0)
                    );
                    let full_width = full_text.measure(ctx).map(|m| m.x).unwrap_or(0.0);
                    
                    // 如果文本超长，从右往左截取可见部分
                    let visible_text = if full_width > input_box_width {
                        let chars: Vec<char> = text.chars().collect();
                        let mut visible_chars = chars.clone();
                        
                        while visible_chars.len() > 0 {
                            let test_text = Text::new(
                                TextFragment::new(visible_chars.iter().collect::<String>())
                                    .font("AlibabaPuHuiTi")
                                    .scale(21.0)
                            );
                            let test_width = test_text.measure(ctx).map(|m| m.x).unwrap_or(0.0);
                            
                            if test_width <= input_box_width - 5.0 {
                                break;
                            }
                            visible_chars.remove(0);
                        }
                        visible_chars.iter().collect::<String>()
                    } else {
                        text.to_string()
                    };
                    
                    let content_text = Text::new(
                        TextFragment::new(&visible_text)
                            .font("AlibabaPuHuiTi")
                            .scale(21.0)
                    );
                    
                    canvas.draw(&content_text, DrawParam::default()
                        .dest([x + 2.0, *y - 1.0])  // 向上移1像素使其在18px输入框中居中
                        .color(GgezColor::WHITE));
                    
                    // 绘制光标 (如果该输入框获得焦点)
                    if dialog.focused_field == *field && dialog.cursor_visible {
                        // 使用可见文本宽度计算光标位置
                        let visible_width = content_text.measure(ctx).map(|m| m.x).unwrap_or(0.0);
                        let cursor_x = x + 2.0 + visible_width;
                        
                        if let Ok(cursor) = Mesh::new_rectangle(
                            ctx,
                            DrawMode::fill(),
                            Rect::new(cursor_x, *y + 1.0, 2.0, 16.0),  // 高度改为16px,Y坐标+1
                            GgezColor::WHITE,
                        ) {
                            canvas.draw(&cursor, DrawParam::default());
                        }
                    }
                }
                
                // 4. 绘制按钮 (使用Title库)
                if let Some(title_arc) = get_library(LibraryName::Title) {
                    if let Ok(mut title_lib) = title_arc.try_lock() {
                        // OK按钮 (C# Location: (80, 236))
                        // C# 使用: Index 107 (normal), 108 (hover), 109 (pressed)
                        let ok_index = if dialog.ok_button_hovered { 108 } else { 107 };
                        let _ = title_lib.draw_with_color(ctx, canvas, ok_index, box_x + 80.0, box_y + 236.0, ggez::graphics::Color::WHITE, false);
                        
                        // Cancel按钮 (C# Location: (222, 236))
                        // C# 使用: Index 110 (normal), 111 (hover), 112 (pressed)
                        let cancel_index = if dialog.cancel_button_hovered { 111 } else { 110 };
                        let _ = title_lib.draw_with_color(ctx, canvas, cancel_index, box_x + 222.0, box_y + 236.0, ggez::graphics::Color::WHITE, false);
                    }
                }
                
                // 5. 绘制提示文本 (使用中文字体)
                let hint_text = Text::new(
                    TextFragment::new("按Tab切换输入框 | 按ESC关闭")
                        .font("AlibabaPuHuiTi")
                        .scale(21.0)
                );
                canvas.draw(&hint_text, DrawParam::default()
                    .dest([box_x + 80.0, box_y + 280.0])
                    .color(GgezColor::from_rgb(200, 200, 200)));
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
    /// 更新场景逻辑（ECS版本 - 返回场景切换请求）
    fn update(
        &mut self, 
        ctx: &mut Context, 
        _world: &mut World,
        _network_tx: &mpsc::UnboundedSender<NetworkCommand>
    ) -> GameResult<Option<SceneType>> {
        // 初始化（首次调用）
        if !self.login_dialog.visible && self.new_account_dialog.is_none() && self.change_password_dialog.is_none() {
            self.login_dialog.show();
            tracing::info!("LoginScene 初始化完成");
        }
        // 更新背景动画 (C# 原版: AnimationCount=19, AnimationDelay=100ms)
        // 如果没有暂停，则更新动画
        let delta_time=ctx.time.delta().as_secs_f32();
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
        
        // 检查是否应该切换到角色选择场景
        if self.ready_for_character_select {
            return Ok(Some(SceneType::Select));
        }
        
        Ok(None)
    }
    
    /// 绘制场景（ECS版本 - 添加World参数）
    fn draw(&mut self, ctx: &mut Context, canvas: &mut Canvas, _world: &World) -> GameResult {
        use crate::graphics::libraries::{get_library, LibraryName};
        use ggez::graphics::{Text, TextFragment, DrawParam, Color as GgezColor};
        
        // 🔧 清除Canvas,防止之前帧的残留
        use ggez::graphics::{Rect, DrawMode, Mesh, Color};
        let (screen_width, screen_height) = ctx.gfx.drawable_size();
        let clear_color = Color::from_rgb(0, 0, 0); // 黑色背景
        let clear_rect = Rect::new(0.0, 0.0, screen_width, screen_height);
        if let Ok(clear_mesh) = Mesh::new_rectangle(ctx, DrawMode::fill(), clear_rect, clear_color) {
            canvas.draw(&clear_mesh, DrawParam::default());
        }
        
        // 1. 绘制登录背景动画 (C# 原版: ChrSel.lib 索引 0-18, 共19帧)
        if let Some(lib_arc) = get_library(LibraryName::ChrSel) {
            if let Ok(mut lib) = lib_arc.try_lock() {
                // 使用动画帧索引 (0-18)
                let frame_index = self.background_frame.min(18);
                if let Err(e) = lib.draw_with_color(ctx, canvas, frame_index, 0.0, 0.0, ggez::graphics::Color::WHITE, false) {
                    tracing::error!("❌ 绘制背景失败 (帧{}): {}", frame_index, e);
                }
            } else {
                tracing::warn!("⚠ 无法锁定 ChrSel 库");
            }
        } else {
            tracing::warn!("⚠ ChrSel 库未加载");
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
                let _ = lib.draw_with_color(ctx, canvas, 1084, dialog_x, dialog_y, ggez::graphics::Color::WHITE, false);
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
                let _ = lib.draw_with_color(ctx, canvas, 30, dialog_x + 114.0, dialog_y + 12.0, ggez::graphics::Color::WHITE, false);
                
                // "账号ID" 标签 (索引 31)
                // C# 位置: (52, 83)
                let _ = lib.draw_with_color(ctx, canvas, 31, dialog_x + 52.0, dialog_y + 83.0, ggez::graphics::Color::WHITE, false);
                
                // "密码" 标签 (索引 32)
                // C# 位置: (43, 105)
                let _ = lib.draw_with_color(ctx, canvas, 32, dialog_x + 43.0, dialog_y + 105.0, ggez::graphics::Color::WHITE, false);
                
                // OK/登录按钮 (索引 320/321/322 = normal/hover/pressed)
                // C# 位置: (227, 81), 大小: 42x42
                let ok_index = if self.ok_button_hovered { 321 } else { 320 };
                let _ = lib.draw_with_color(ctx, canvas, ok_index, dialog_x + 227.0, dialog_y + 81.0, ggez::graphics::Color::WHITE, false);
                
                // "新建账号" 按钮 (索引 323/324/325)
                // C# 位置: (60, 163)
                let account_index = if self.account_button_hovered { 324 } else { 323 };
                let _ = lib.draw_with_color(ctx, canvas, account_index, dialog_x + 60.0, dialog_y + 163.0, ggez::graphics::Color::WHITE, false);
                
                // "修改密码" 按钮 (索引 326/327/328)
                // C# 位置: (166, 163)
                let pass_index = if self.pass_button_hovered { 327 } else { 326 };
                let _ = lib.draw_with_color(ctx, canvas, pass_index, dialog_x + 166.0, dialog_y + 163.0, ggez::graphics::Color::WHITE, false);
                
                // "查看密钥" 按钮 (索引 332/333/334)
                // C# 位置: (60, 189)
                let view_key_index = if self.view_key_button_hovered { 333 } else { 332 };
                let _ = lib.draw_with_color(ctx, canvas, view_key_index, dialog_x + 60.0, dialog_y + 189.0, ggez::graphics::Color::WHITE, false);
                
                // "关闭" 按钮 (索引 329/330/331)
                // C# 位置: (166, 189)
                let close_index = if self.close_button_hovered { 330 } else { 329 };
                let _ = lib.draw_with_color(ctx, canvas, close_index, dialog_x + 166.0, dialog_y + 189.0, ggez::graphics::Color::WHITE, false);
            }
        }
        
        // 3. 绘制文本信息 (使用中文字体)
        // 版本信息
        let version_text = Text::new(
            TextFragment::new("Crystal v1.0 - Ggez Edition")
                .font("AlibabaPuHuiTi")
                .scale(21.0)
        );
        let version_params = DrawParam::default()
            .dest([10.0, 10.0])
            .color(GgezColor::from_rgb(200, 200, 255));
        canvas.draw(&version_text, version_params);
        
        // 连接状态
        if self.connecting {
            let status_text = Text::new(
                TextFragment::new(format!("正在连接服务器... (尝试 {})", self.connect_attempts))
                    .font("AlibabaPuHuiTi")
                    .scale(21.0)
            );
            let status_params = DrawParam::default()
                .dest([10.0, 740.0]) // 底部
                .color(GgezColor::from_rgb(255, 255, 100));
            canvas.draw(&status_text, status_params);
        } else if let Some(status) = &self.last_status {
            let status_text = Text::new(
                TextFragment::new(status.as_str())
                    .font("AlibabaPuHuiTi")
                    .scale(21.0)
            );
            let status_params = DrawParam::default()
                .dest([10.0, 740.0])
                .color(GgezColor::from_rgb(100, 255, 100));
            canvas.draw(&status_text, status_params);
        }
        
        // FPS 和调试信息 (可选)
        let fps = ctx.time.fps();
        let debug_text = Text::new(
            TextFragment::new(format!("FPS: {:.1}", fps))
                .font("AlibabaPuHuiTi")
                .scale(21.0)
        );
        let debug_params = DrawParam::default()
            .dest([950.0, 10.0])
            .color(GgezColor::from_rgb(255, 255, 255));
        canvas.draw(&debug_text, debug_params);
        
        // 3.5 绘制登录对话框的输入框文本和光标
        if self.login_dialog.visible {
            self.draw_login_input(ctx, canvas);
        }
        
        // 3.6 绘制新建账号对话框
        if let Some(new_account) = &self.new_account_dialog {
            if new_account.visible {
                self.draw_new_account_dialog(ctx, canvas, new_account);
            }
        }
        
        // 3.7 绘制修改密码对话框
        if let Some(change_pass) = &self.change_password_dialog {
            if change_pass.visible {
                self.draw_change_password_dialog(ctx, canvas, change_pass);
            }
        }
        
        // 4. 绘制消息框 (MessageBox) - 最后绘制，显示在最上层
        if let Some(msg_box) = &self.message_box {
            if msg_box.visible {
                self.draw_message_box(ctx, canvas, msg_box);
            }
        }
        
        Ok(())
    }
    
    /// 鼠标按下事件
    fn on_mouse_down(
        &mut self,
        _ctx: &mut Context,
        _world: &mut World,
        button: ggez::winit::event::MouseButton,
        x: f32,
        y: f32,
        network_tx: &mpsc::UnboundedSender<NetworkCommand>,
    ) -> GameResult {
        use ggez::winit::event::MouseButton;
        
        // 只处理左键点击
        if button != MouseButton::Left {
            return Ok(());
        }
        
        // 计算对话框位置（与绘制时相同）
        let center_x = 1024.0 / 2.0;
        let center_y = 768.0 / 2.0;
        let dialog_x = center_x - 164.0;
        let dialog_y = center_y - 110.0;
        
        // 检查各个按钮的点击区域
        // OK/登录按钮 (227, 81), 大小约 42x42
        if x >= dialog_x + 227.0 && x <= dialog_x + 227.0 + 42.0 &&
           y >= dialog_y + 81.0 && y <= dialog_y + 81.0 + 42.0 {
            // 登录按钮点击
            println!("🔘 登录按钮被点击");
            if self.login_enabled {
                if let Some((account, password)) = self.login_dialog.get_credentials() {
                    println!("✅ 发送登录请求: 账号={}, 密码长度={}", account, password.len());
                    
                    // 发送登录命令（NetworkManager会自动连接服务器）
                    let _ = network_tx.send(NetworkCommand::Login { 
                        username: account, 
                        password 
                    });
                    
                    self.login_enabled = false; // 禁用登录按钮防止重复点击
                    self.connecting = true; // 设置连接状态
                }
            }
        }
        // "新建账号" 按钮 (60, 163), 大小约 90x30
        else if x >= dialog_x + 60.0 && x <= dialog_x + 60.0 + 90.0 &&
                y >= dialog_y + 163.0 && y <= dialog_y + 163.0 + 30.0 {
            println!("🔘 新建账号按钮被点击");
            self.open_new_account_dialog();
        }
        // "修改密码" 按钮 (166, 163), 大小约 90x30
        else if x >= dialog_x + 166.0 && x <= dialog_x + 166.0 + 90.0 &&
                y >= dialog_y + 163.0 && y <= dialog_y + 163.0 + 30.0 {
            println!("🔘 修改密码按钮被点击");
            self.open_change_password_dialog(None, None);
        }
        // "查看密钥" 按钮 (60, 189), 大小约 90x30
        else if x >= dialog_x + 60.0 && x <= dialog_x + 60.0 + 90.0 &&
                y >= dialog_y + 189.0 && y <= dialog_y + 189.0 + 30.0 {
            println!("🔘 查看密钥按钮被点击");
            // TODO: 实现密钥查看功能
        }
        // "关闭" 按钮 (166, 189), 大小约 90x30
        else if x >= dialog_x + 166.0 && x <= dialog_x + 166.0 + 90.0 &&
                y >= dialog_y + 189.0 && y <= dialog_y + 189.0 + 30.0 {
            println!("🔘 关闭按钮被点击");
            // TODO: 关闭游戏
        }
        // 账号输入框点击 (约 100, 83), 大小约 120x20
        else if x >= dialog_x + 100.0 && x <= dialog_x + 100.0 + 120.0 &&
                y >= dialog_y + 83.0 && y <= dialog_y + 83.0 + 20.0 {
            println!("🔘 账号输入框被点击");
            self.login_dialog.focus_account();
        }
        // 密码输入框点击 (约 100, 105), 大小约 120x20
        else if x >= dialog_x + 100.0 && x <= dialog_x + 100.0 + 120.0 &&
                y >= dialog_y + 105.0 && y <= dialog_y + 105.0 + 20.0 {
            println!("🔘 密码输入框被点击");
            self.login_dialog.focus_password();
        }
        
        Ok(())
    }
    
    /// 鼠标移动事件（用于悬停效果）
    fn on_mouse_move(
        &mut self,
        _ctx: &mut Context,
        _world: &mut World,
        x: f32,
        y: f32,
    ) -> GameResult {
        // 计算对话框位置
        let center_x = 1024.0 / 2.0;
        let center_y = 768.0 / 2.0;
        let dialog_x = center_x - 164.0;
        let dialog_y = center_y - 110.0;
        
        // 检查各个按钮的悬停状态
        self.ok_button_hovered = 
            x >= dialog_x + 227.0 && x <= dialog_x + 227.0 + 42.0 &&
            y >= dialog_y + 81.0 && y <= dialog_y + 81.0 + 42.0;
        
        self.account_button_hovered = 
            x >= dialog_x + 60.0 && x <= dialog_x + 60.0 + 90.0 &&
            y >= dialog_y + 163.0 && y <= dialog_y + 163.0 + 30.0;
        
        self.pass_button_hovered = 
            x >= dialog_x + 166.0 && x <= dialog_x + 166.0 + 90.0 &&
            y >= dialog_y + 163.0 && y <= dialog_y + 163.0 + 30.0;
        
        self.view_key_button_hovered = 
            x >= dialog_x + 60.0 && x <= dialog_x + 60.0 + 90.0 &&
            y >= dialog_y + 189.0 && y <= dialog_y + 189.0 + 30.0;
        
        self.close_button_hovered = 
            x >= dialog_x + 166.0 && x <= dialog_x + 166.0 + 90.0 &&
            y >= dialog_y + 189.0 && y <= dialog_y + 189.0 + 30.0;
        
        Ok(())
    }
    
    /// 键盘按下事件
    fn on_key_down(
        &mut self,
        _ctx: &mut Context,
        _world: &mut World,
        input: ggez::input::keyboard::KeyInput,
        network_tx: &mpsc::UnboundedSender<NetworkCommand>,
    ) -> GameResult<Option<SceneType>> {
        use ggez::winit::keyboard::KeyCode;
        
        // 检查物理键码
        if let ggez::winit::event::KeyEvent {
            physical_key: ggez::winit::keyboard::PhysicalKey::Code(keycode),
            text,
            ..
        } = &input.event {
            match keycode {
                KeyCode::Backspace => {
                    // 退格键
                    if self.login_dialog.visible {
                        self.login_dialog.handle_backspace();
                    }
                }
                KeyCode::Enter => {
                    // 回车键 - 提交登录
                    if self.login_dialog.visible && self.login_enabled {
                        if let Some((account, password)) = self.login_dialog.get_credentials() {
                            println!("✅ [回车] 发送登录请求: 账号={}, 密码长度={}", account, password.len());
                            
                            // 发送登录命令
                            let _ = network_tx.send(NetworkCommand::Login { 
                                username: account, 
                                password 
                            });
                            
                            self.login_enabled = false;
                            self.connecting = true;
                        }
                    }
                }
                KeyCode::Tab => {
                    // Tab键 - 切换输入焦点
                    if self.login_dialog.visible {
                        self.login_dialog.handle_tab();
                    }
                }
                _ => {
                    // 处理文本输入
                    if let Some(text_str) = text {
                        if self.login_dialog.visible {
                            for ch in text_str.chars() {
                                self.login_dialog.handle_text_input(ch);
                            }
                        }
                    }
                }
            }
        }
        
        Ok(None)
    }
    
    fn as_any_mut(&mut self) -> &mut dyn std::any::Any {
        self
    }
}

// ============================================================================
// 辅助方法 - 处理网络事件（从GameApp调用）
// ============================================================================

impl LoginScene {
    /// 处理网络事件（由GameApp调用）
    pub fn handle_network_event(&mut self, event: &GameEvent) {
        match event {
            GameEvent::Connected => {
                let status = "Connected to server successfully!";
                println!("✅ {}", status);
                self.connecting = false;
                self.record_status(status);
                
                // 发送客户端版本验证
                self.send_client_version();
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
    
}
