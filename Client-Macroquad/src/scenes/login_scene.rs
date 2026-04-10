// ============================================================================
// LoginScene - 登录界面 (纯 Native 版本 - macroquad 原生渲染)
// ============================================================================
// 对应原版: C# Client/MirScenes/LoginScene.cs
//
// 【渲染架构】纯 macroquad 原生渲染
// - 所有 UI 元素使用 macroquad 原生绘制
// - 无 egui 依赖
//
// ============================================================================

use crate::game::GameResult;
use crate::network::{NetContext, NetworkBuilder, NetworkEvent};
use crate::resources::LibraryName;
use crate::scenes::{Scene, SceneTransition};
use crate::ui::text_renderer::draw_text_cn;
use crate::ui::widgets::{draw_button, draw_input_box, draw_message_box};
use macroquad::prelude::*;

mod change_password_dialog;
mod new_account_dialog;

use change_password_dialog::ChangePasswordFocus;
use new_account_dialog::NewAccountFocus;

/// 登录场景 - 纯 Native 版本
pub struct LoginScene {
    // 登录信息
    account: String,
    password: String,
    #[allow(dead_code)]
    password_focus: bool,
    
    // 背景动画
    background_frame: usize,
    animation_playing: bool,
    frame_timer: f32,
    frame_delay: f32,
    
    // UI 状态
    cursor_visible: bool,
    cursor_timer: f32,
    input_focus: InputFocus,
    
    // 消息框
    show_message: bool,
    message_text: String,

    // 改密码对话框（对应原版 ChangePasswordDialog）
    show_change_password: bool,
    cp_account_id: String,
    cp_current_password: String,
    cp_new_password1: String,
    cp_new_password2: String,
    cp_focus: ChangePasswordFocus,
    cp_in_flight: bool,

    // 新建账号对话框（对应原版 NewAccountDialog）
    show_new_account: bool,
    na_account_id: String,
    na_password1: String,
    na_password2: String,
    na_user_name: String,
    na_birth_date: String,
    na_question: String,
    na_answer: String,
    na_email: String,
    na_focus: NewAccountFocus,
    na_in_flight: bool,

    // 网络
    net: Option<NetContext>,
    cfg: crate::network::NetworkRuntimeConfig,
    login_pending: bool,
    version_ok: bool,
}

#[derive(PartialEq, Clone, Copy)]
enum InputFocus {
    Account,
    Password,
    #[allow(dead_code)]
    None,
}

impl LoginScene {
    pub fn new() -> Self {
        Self {
            account: String::new(),
            password: String::new(),
            password_focus: false,
            
            background_frame: 0,
            animation_playing: false,
            frame_timer: 0.0,
            frame_delay: 0.1,
            
            cursor_visible: true,
            cursor_timer: 0.0,
            input_focus: InputFocus::Account,
            
            show_message: false,
            message_text: String::new(),

            show_change_password: false,
            cp_account_id: String::new(),
            cp_current_password: String::new(),
            cp_new_password1: String::new(),
            cp_new_password2: String::new(),
            cp_focus: ChangePasswordFocus::AccountId,
            cp_in_flight: false,

            show_new_account: false,
            na_account_id: String::new(),
            na_password1: String::new(),
            na_password2: String::new(),
            na_user_name: String::new(),
            na_birth_date: String::new(),
            na_question: String::new(),
            na_answer: String::new(),
            na_email: String::new(),
            na_focus: NewAccountFocus::AccountId,
            na_in_flight: false,

            net: None,
            cfg: crate::network::load_network_runtime_config(),
            login_pending: false,
            version_ok: false,
        }
    }

    fn ensure_network(&mut self) {
        if self.net.is_some() {
            return;
        }

        self.cfg = crate::network::load_network_runtime_config();
        let builder = NetworkBuilder::new(self.cfg.server_addr.clone())
            .with_mock(self.cfg.use_mock)
            .with_client_version_hash(self.cfg.client_version_hash);
        match builder.build() {
            Ok(net) => {
                self.net = Some(net);
                self.version_ok = self.cfg.use_mock;
            }
            Err(e) => {
                self.message_text = format!("网络初始化失败: {e}");
                self.show_message = true;
            }
        }
    }

    fn pump_network(&mut self) -> Option<Vec<mir2_shared::SelectInfo>> {
        let Some(net) = self.net.as_ref() else {
            return None;
        };

        let events = net.recv_all();
        if events.is_empty() {
            return None;
        }

        for ev in events {
            match ev {
                NetworkEvent::ClientVersionResponse { result } => {
                    if result == 1 {
                        self.version_ok = true;
                    } else {
                        self.message_text = "客户端版本不匹配（ClientVersion 被服务器拒绝）".to_string();
                        self.show_message = true;
                    }
                }
                NetworkEvent::Disconnected { reason } => {
                    self.message_text = format!("已断开连接: {reason}");
                    self.show_message = true;
                }
                NetworkEvent::LoginSuccess { characters } => {
                    return Some(characters);
                }
                NetworkEvent::LoginFailed { reason } => {
                    self.message_text = reason;
                    self.show_message = true;
                }
                NetworkEvent::NewAccountSuccess => {
                    self.na_in_flight = false;
                    self.message_text = "账号创建成功".to_string();
                    self.show_message = true;

                    // 便于后续直接登录：预填登录框
                    self.account = self.na_account_id.clone();
                    self.password = self.na_password1.clone();
                    self.close_new_account_dialog();
                }
                NetworkEvent::NewAccountFailed { reason } => {
                    self.na_in_flight = false;
                    self.message_text = reason;
                    self.show_message = true;
                }
                NetworkEvent::ChangePasswordSuccess => {
                    self.cp_in_flight = false;
                    self.message_text = "密码修改成功".to_string();
                    self.show_message = true;
                    self.close_change_password_dialog();
                }
                NetworkEvent::ChangePasswordFailed { reason } => {
                    self.cp_in_flight = false;
                    self.message_text = reason;
                    self.show_message = true;
                }
                _ => {}
            }
        }

        None
    }

    /// 绘制登录对话框背景
    fn draw_login_background(&self) -> (f32, f32, f32, f32) {
        let screen_w = screen_width();
        let screen_h = screen_height();

        // 原版固定对话框尺寸：328x220
        let dialog_w = 328.0;
        let dialog_h = 220.0;
        let dialog_x = (screen_w - dialog_w) / 2.0;
        let dialog_y = (screen_h - dialog_h) / 2.0;

        // 背景 Prguse[1084]
        if let Some(info) = LibraryName::Prguse.get_texture(1084) {
            if let Some(ref bg_tex) = info.image {
                draw_texture(bg_tex, dialog_x, dialog_y, WHITE);
            }
        }

        // 绘制标题 (Title 30)
        if let Some(info) = LibraryName::Title.get_texture(30) {
            if let Some(ref tex) = info.image {
                let w = tex.width();
                let x = dialog_x + (dialog_w - w) / 2.0;
                let y = dialog_y + 12.0;
                draw_texture(tex, x, y, WHITE);
            }
        }

        // 绘制账号标签 (Title 31)
        if let Some(info) = LibraryName::Title.get_texture(31) {
            if let Some(ref tex) = info.image {
                draw_texture(tex, dialog_x + 52.0, dialog_y + 83.0, WHITE);
            }
        }

        // 绘制密码标签 (Title 32)
        if let Some(info) = LibraryName::Title.get_texture(32) {
            if let Some(ref tex) = info.image {
                draw_texture(tex, dialog_x + 43.0, dialog_y + 105.0, WHITE);
            }
        }

        (dialog_w, dialog_h, dialog_x, dialog_y)
    }

    // 通用 UI 绘制（输入框/按钮/消息框）已抽到 crate::ui::widgets

    /// 登录按钮点击
    fn on_login_clicked(&mut self) {
        if self.account.is_empty() || self.password.is_empty() {
            self.message_text = "账号或密码不能为空!".to_string();
            self.show_message = true;
            return;
        }

        println!("🔐 Login: account={}", self.account);

        // 保存配置
        self.save_config();

        // 触发真实登录流程：连接服务器 + 登录
        self.ensure_network();
        self.login_pending = true;
    }

    /// 保存配置到本地文件
    fn save_config(&self) {
        use std::fs;
        use std::io::Write;
        use std::path::Path;

        fn upsert_ini_section(existing: &str, section_name: &str, replacement_section: &str) -> String {
            let mut out = String::with_capacity(existing.len() + replacement_section.len() + 16);
            let mut in_target = false;
            let mut replaced = false;

            for line in existing.lines() {
                let trimmed = line.trim();
                if trimmed.starts_with('[') && trimmed.ends_with(']') {
                    if in_target {
                        in_target = false;
                    }

                    let name = trimmed[1..trimmed.len().saturating_sub(1)].trim();
                    if name.eq_ignore_ascii_case(section_name) {
                        if !replaced {
                            out.push_str(replacement_section.trim_end_matches('\n'));
                            out.push('\n');
                            replaced = true;
                        }
                        in_target = true;
                        continue;
                    }
                }

                if !in_target {
                    out.push_str(line);
                    out.push('\n');
                }
            }

            if !replaced {
                if !out.is_empty() && !out.ends_with('\n') {
                    out.push('\n');
                }
                out.push_str(replacement_section.trim_end_matches('\n'));
                out.push('\n');
            }

            out
        }

        let timestamp = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_secs();

        let path = Path::new(env!("CARGO_MANIFEST_DIR")).join("config.ini");
        let login_section = format!(
            "[Login]\nAccount={}\nSavePassword=false\nLastLogin={}\nVersion={}\n",
            self.account,
            timestamp,
            env!("CARGO_PKG_VERSION")
        );

        let content = fs::read_to_string(&path).ok();
        let merged = match content {
            Some(existing) => upsert_ini_section(&existing, "Login", &login_section),
            None => login_section,
        };

        if let Ok(mut file) = fs::File::create(path) {
            let _ = file.write_all(merged.as_bytes());
            println!("✅ 配置已保存");
        }
    }

    /// 加载配置
    fn load_config(&mut self) {
        use std::fs;
        use std::path::Path;

        let path = Path::new(env!("CARGO_MANIFEST_DIR")).join("config.ini");
        if let Ok(content) = fs::read_to_string(path) {
            for line in content.lines() {
                if let Some(account) = line.strip_prefix("Account=") {
                    self.account = account.to_string();
                    println!("✅ 已加载账号: {}", account);
                }
            }
        }
    }

    /// 处理键盘输入
    fn handle_text_input(&mut self) {
        // 处理字符输入
        while let Some(ch) = get_char_pressed() {
            if ch.is_ascii() && !ch.is_control() {
                match self.input_focus {
                    InputFocus::Account => {
                        if self.account.len() < 20 {
                            self.account.push(ch);
                        }
                    }
                    InputFocus::Password => {
                        if self.password.len() < 20 {
                            self.password.push(ch);
                        }
                    }
                    InputFocus::None => {}
                }
            }
        }

        // 处理退格键
        if is_key_pressed(KeyCode::Backspace) {
            match self.input_focus {
                InputFocus::Account => {
                    self.account.pop();
                }
                InputFocus::Password => {
                    self.password.pop();
                }
                InputFocus::None => {}
            }
        }

        // Tab 切换焦点
        if is_key_pressed(KeyCode::Tab) {
            self.input_focus = match self.input_focus {
                InputFocus::Account => InputFocus::Password,
                InputFocus::Password => InputFocus::Account,
                InputFocus::None => InputFocus::Account,
            };
        }

        // Enter 登录
        if is_key_pressed(KeyCode::Enter) {
            self.on_login_clicked();
        }
    }
}

impl Scene for LoginScene {
    fn name(&self) -> &str {
        "登录界面"
    }

    fn on_enter(&mut self) -> GameResult {
        self.account.clear();
        self.password.clear();
        self.load_config();

        self.net = None;
        self.cfg = crate::network::load_network_runtime_config();
        self.login_pending = false;
        self.version_ok = false;
        self.animation_playing = false;
        self.background_frame = 0;
        println!("🎬 进入登录界面");
        Ok(())
    }

    fn on_exit(&mut self) -> GameResult {
        println!("🎬 离开登录界面");
        Ok(())
    }

    fn update(&mut self, dt: f32) -> GameResult<SceneTransition> {
        // 更新光标闪烁
        self.cursor_timer += dt;
        if self.cursor_timer >= 0.5 {
            self.cursor_timer = 0.0;
            self.cursor_visible = !self.cursor_visible;
        }

        // 更新背景动画
        if self.animation_playing {
            self.frame_timer += dt;
            if self.frame_timer >= self.frame_delay {
                self.frame_timer = 0.0;
                self.background_frame += 1;

                if self.background_frame >= 19 {
                    println!("✓ Login animation finished, switching to character select...");
                    return Ok(SceneTransition::CharacterSelect);
                }
            }
        }

        // 处理登录请求（在 update 中发包，避免 render 阶段做 IO）
        if self.login_pending {
            self.ensure_network();
            if let Some(net) = self.net.as_ref() {
                if !self.version_ok {
                    // 等待 ClientVersionResponse；保持 pending，版本 OK 后自动发送 LoginRequest
                    self.message_text = "正在校验版本...".to_string();
                    self.show_message = true;
                } else {
                    self.login_pending = false;
                    if let Err(e) = net.send(NetworkEvent::LoginRequest {
                        username: self.account.clone(),
                        password: self.password.clone(),
                    }) {
                        self.message_text = format!("发送 LoginRequest 失败: {e}");
                        self.show_message = true;
                    } else {
                        self.message_text = "正在登录...".to_string();
                        self.show_message = true;
                    }
                }
            }
        }

        // 轮询网络：等待登录成功
        if !self.animation_playing {
            if let Some(characters) = self.pump_network() {
                // 成功：把连接与角色列表移交给后续场景
                crate::network::set_global_characters(characters);
                if let Some(net) = self.net.take() {
                    crate::network::set_global_net(net);
                }

                self.show_message = false;
                self.animation_playing = true;
                self.background_frame = 0;
                self.frame_timer = 0.0;
            }
        }

        self.handle_input()?;

        Ok(SceneTransition::None)
    }

    fn render(&mut self) -> GameResult {
        clear_background(BLACK);

        // 绘制背景动画 (ChrSel 库)
        let frame_index = if self.animation_playing {
            self.background_frame
        } else {
            0
        };

        if let Some(info) = LibraryName::ChrSel.get_texture(frame_index) {
            if let Some(ref texture) = info.image {
                draw_texture(texture, 0.0, 0.0, WHITE);
            }
        }

        // 如果没有播放动画，绘制登录对话框
        if !self.animation_playing {
            // 改密码弹窗优先（原版会隐藏登录对话框）
            if self.show_change_password {
                self.draw_change_password_dialog();
                // 仍允许显示消息框
            } else if self.show_new_account {
                self.draw_new_account_dialog();
            } else {
                let (_dialog_w, dialog_h, dialog_x, dialog_y) = self.draw_login_background();
            
            // 绘制输入框
            // 原版坐标：AccountIDTextBox (85,85) size(136,15), PasswordTextBox (85,108) size(136,15)
            let input_x = dialog_x + 85.0;
            let input_w = 136.0;
            let input_h = 15.0;
            
            // 账号输入框
            let account_y = dialog_y + 85.0;
            draw_input_box(
                input_x,
                account_y,
                input_w,
                input_h,
                &self.account,
                false,
                self.input_focus == InputFocus::Account,
                self.cursor_visible,
                14.0,
            );
            
            // 密码输入框
            let password_y = dialog_y + 108.0;
            draw_input_box(
                input_x,
                password_y,
                input_w,
                input_h,
                &self.password,
                true,
                self.input_focus == InputFocus::Password,
                self.cursor_visible,
                14.0,
            );

            // 按钮：对齐原版 C# LoginDialog
            // OKButton Title[320-322] at (227,81) size 42x42
            let ok_enabled = !self.account.is_empty() && !self.password.is_empty();
            if draw_button(
                LibraryName::Title,
                dialog_x + 227.0,
                dialog_y + 81.0,
                320,
                321,
                322,
                ok_enabled,
            ) {
                self.on_login_clicked();
            }

            // AccountButton Title[323-325] at (60,163)
            if draw_button(
                LibraryName::Title,
                dialog_x + 60.0,
                dialog_y + 163.0,
                323,
                324,
                325,
                true,
            ) {
                self.open_new_account_dialog();
            }

            // PassButton Title[326-328] at (166,163)
            if draw_button(
                LibraryName::Title,
                dialog_x + 166.0,
                dialog_y + 163.0,
                326,
                327,
                328,
                true,
            ) {
                self.open_change_password_dialog();
            }

            // ViewKeyButton Title[332-334] at (60,189)
            if draw_button(
                LibraryName::Title,
                dialog_x + 60.0,
                dialog_y + 189.0,
                332,
                333,
                334,
                true,
            ) {
                println!("🔎 查看密钥");
            }

            // CloseButton Title[329-331] at (166,189)
            if draw_button(
                LibraryName::Title,
                dialog_x + 166.0,
                dialog_y + 189.0,
                329,
                330,
                331,
                true,
            ) {
                std::process::exit(0);
            }

            // 左下角显示当前服务器/模式（不增加交互，仅提示）
            let status = if self.cfg.use_mock {
                format!("服务器: {}  模式: Mock", self.cfg.server_addr)
            } else {
                format!("服务器: {}", self.cfg.server_addr)
            };
            draw_text_cn(&status, dialog_x + 10.0, dialog_y + dialog_h - 10.0, 12.0, LIGHTGRAY);
            
                // 处理点击输入框切换焦点
                let (mx, my) = mouse_position();
                if is_mouse_button_pressed(MouseButton::Left) {
                    if mx >= input_x && mx <= input_x + input_w {
                        if my >= account_y && my <= account_y + input_h {
                            self.input_focus = InputFocus::Account;
                        } else if my >= password_y && my <= password_y + input_h {
                            self.input_focus = InputFocus::Password;
                        }
                    }
                }
            }
        }

        // 绘制消息框
        if self.show_message {
            draw_message_box(&self.message_text);
            
            // 点击任意位置关闭消息框
            if is_mouse_button_pressed(MouseButton::Left) {
                self.show_message = false;
            }
        }

        Ok(())
    }

    fn handle_input(&mut self) -> GameResult {
        if is_key_pressed(KeyCode::Escape) {
            if self.show_message {
                self.show_message = false;
            } else if self.show_new_account {
                self.close_new_account_dialog();
            } else if self.show_change_password {
                self.close_change_password_dialog();
            } else {
                std::process::exit(0);
            }
        }

        // 处理文本输入
        if !self.animation_playing && !self.show_message {
            if self.show_change_password {
                self.handle_change_password_text_input();
            } else if self.show_new_account {
                self.handle_new_account_text_input();
            } else {
                self.handle_text_input();
            }
        }

        Ok(())
    }
}
