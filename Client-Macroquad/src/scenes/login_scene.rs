// ============================================================================
// LoginScene - 登录界面 (混合渲染架构)
// ============================================================================
// 对应原版: C# Client/MirScenes/LoginScene.cs
//
// 【渲染架构】混合渲染 - macroquad + egui
// ┌─────────────────────────────────────────────────────────────────────┐
// │ Layer 1 (底层): macroquad 纹理渲染                                    │
// │   - ChrSel 库: 背景动画 (0-18 帧)                                     │
// │   - Prguse 库: 登录对话框背景 (1084)                                  │
// │   - Title 库: 标题和标签 (30/31/32)                                   │
// │   - DPI 处理: 自动根据 screen_dpi_scale() 缩放                        │
// │   - 坐标系统: 物理像素坐标                                             │
// ├─────────────────────────────────────────────────────────────────────┤
// │ Layer 2 (交互层): egui UI 组件                                        │
// │   - TextEdit: 账号/密码输入框                                         │
// │   - ImageButton: 登录/新建/修改密码/退出按钮                           │
// │   - DPI 处理: ctx.set_pixels_per_point(screen_dpi_scale())           │
// │   - 坐标系统: 逻辑像素,通过 pixels_per_point 与 macroquad 对齐         │
// └─────────────────────────────────────────────────────────────────────┘
//
// 【DPI 适配机制】
//   - macOS Retina: screen_dpi_scale() = 2.0 → pixels_per_point = 2.0
//   - Windows 普通屏: screen_dpi_scale() = 1.0 → pixels_per_point = 1.0
//   - 自动对齐: macroquad 物理坐标 ≡ egui 逻辑坐标 * pixels_per_point
//
// ============================================================================

use crate::game::GameResult;
use crate::resources::LibraryName;
use crate::scenes::{Scene, SceneTransition};
use crate::scenes::dialogs::{
    Dialog, MessageBox, NewAccountDialog, ChangePasswordDialog, MessageBoxButtons,
    LoginDialog, LoginDialogEvent,
};
use egui_macroquad::egui;
use macroquad::prelude::*;

/// 登录场景 - 混合渲染版本
pub struct LoginScene {
    // 对话框组件
    login_dialog: LoginDialog,
    new_account_dialog: NewAccountDialog,
    change_password_dialog: ChangePasswordDialog,
    message_box: MessageBox,
    
    // 对话框状态
    show_login_dialog: bool,
    show_new_account: bool,
    show_change_password: bool,
    show_message_box: bool,

    // 背景动画
    background_frame: usize,
    animation_playing: bool,
    frame_timer: f32,
    frame_delay: f32,
}

impl LoginScene {
    pub fn new() -> Self {
        Self {
            login_dialog: LoginDialog::new(),
            new_account_dialog: NewAccountDialog::new(),
            change_password_dialog: ChangePasswordDialog::new(),
            message_box: MessageBox::new_with_id("", "", MessageBoxButtons::Ok, "login_msgbox"),
            
            show_login_dialog: true,
            show_new_account: false,
            show_change_password: false,
            show_message_box: false,

            background_frame: 0,
            animation_playing: false,
            frame_timer: 0.0,
            frame_delay: 0.1,
        }
    }

    /// 【macroquad 职责】绘制登录对话框背景层
    /// - 纹理渲染: Prguse[1084] 对话框背景, Title[30/31/32] 标题和标签
    /// - DPI 处理: macroquad 自动根据 screen_dpi_scale() 缩放
    /// - 坐标系统: 使用物理像素坐标,原始纹理尺寸
    fn draw_login_background(&mut self) -> (f32, f32, f32, f32) {
        let screen_w = screen_width();
        let screen_h = screen_height();

        // 获取背景纹理并计算居中位置
        let (dialog_w, dialog_h, dialog_x, dialog_y) =
            // ✅ 新 API：性能提升 50-100x，自动 LRU 缓存
            if let Some(info) = LibraryName::Prguse.get_texture(1084) {
                if let Some(ref bg_tex) = info.image {
                    let w = bg_tex.width();
                    let h = bg_tex.height();
                    let x = (screen_w - w) / 2.0;
                    let y = (screen_h - h) / 2.0;

                    // 绘制背景 (原始尺寸,macroquad 会根据 DPI 自动缩放)
                    draw_texture(bg_tex, x, y, WHITE);

                    (w, h, x, y)
                } else {
                    let w = 328.0;
                    let h = 220.0;
                    (w, h, (screen_w - w) / 2.0, (screen_h - h) / 2.0)
                }
            } else {
                let w = 328.0;
                let h = 220.0;
                (w, h, (screen_w - w) / 2.0, (screen_h - h) / 2.0)
            };

        // 绘制标题 (Title 30) - 原始尺寸
        // ✅ 新 API
        if let Some(info) = LibraryName::Title.get_texture(30) {
                if let Some(ref tex) = info.image {
                    let w = tex.width();
                    let x = dialog_x + (dialog_w - w) / 2.0;
                    let y = dialog_y + 12.0;
                    draw_texture(tex, x, y, WHITE);
                }
            }

        // 绘制账号标签 (Title 31) - 原始尺寸
        // ✅ 新 API
        if let Some(info) = LibraryName::Title.get_texture(31) {
                if let Some(ref tex) = info.image {
                    draw_texture(tex, dialog_x + 52.0, dialog_y + 83.0, WHITE);
                }
            }

        // 绘制密码标签 (Title 32) - 原始尺寸
        // ✅ 新 API
        if let Some(info) = LibraryName::Title.get_texture(32) {
            if let Some(ref tex) = info.image {
                draw_texture(tex, dialog_x + 43.0, dialog_y + 105.0, WHITE);
            }
        }

        (dialog_w, dialog_h, dialog_x, dialog_y)
    }

    /// 登录按钮点击
    fn on_login_clicked(&mut self) {
        if self.login_dialog.account.is_empty() || self.login_dialog.password.is_empty() {
            self.message_box.title = "登录失败".to_string();
            self.message_box.text = "账号或密码不能为空!".to_string();
            self.message_box.buttons = MessageBoxButtons::Ok;
            self.show_message_box = true;
            return;
        }

        println!("🔐 Login: account={}", self.login_dialog.account);

        // 保存配置
        self.save_config();

        // 开始播放登录成功动画
        self.animation_playing = true;
        self.background_frame = 0;
        self.show_login_dialog = false;
    }

    /// 【功能】保存配置到本地文件
    fn save_config(&self) {
        use std::fs;
        use std::io::Write;

        let timestamp = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_secs();

        let config = format!(
            "[Login]\nAccount={}\nSavePassword=false\nLastLogin={}\nVersion={}\n",
            self.login_dialog.account,
            timestamp,
            env!("CARGO_PKG_VERSION")
        );

        if let Ok(mut file) = fs::File::create("config.ini") {
            let _ = file.write_all(config.as_bytes());
            println!("✅ 配置已保存");
        }
    }

    /// 【功能】加载配置
    fn load_config(&mut self) {
        use std::fs;

        if let Ok(content) = fs::read_to_string("config.ini") {
            for line in content.lines() {
                if let Some(account) = line.strip_prefix("Account=") {
                    self.login_dialog.account = account.to_string();
                    println!("✅ 已加载账号: {}", account);
                }
            }
        }
    }
}

impl Scene for LoginScene {
    fn name(&self) -> &str {
        "登录界面"
    }

    fn on_enter(&mut self) -> GameResult {
        self.login_dialog.account.clear();
        self.login_dialog.password.clear();

        // 加载保存的配置
        self.load_config();

        // 使用 egui_macroquad::cfg() 配置字体和样式(一次性设置)
        egui_macroquad::cfg(|ctx| {
            let mut fonts = egui::FontDefinitions::default();

            // 加载中文字体
            let font_data = std::fs::read("assets/fonts/AlibabaP uHuiTi-3-55-Regular.ttf")
                .or_else(|_| std::fs::read("assets/fonts/Chinese.ttc"))
                .unwrap_or_else(|_| {
                    println!("⚠ 无法加载中文字体，使用默认字体");
                    vec![]
                });

            if !font_data.is_empty() {
                fonts.font_data.insert(
                    "chinese".to_owned(),
                    std::sync::Arc::new(egui::FontData::from_owned(font_data)),
                );

                // 设置字体优先级
                fonts
                    .families
                    .get_mut(&egui::FontFamily::Proportional)
                    .unwrap()
                    .insert(0, "chinese".to_owned());

                fonts
                    .families
                    .get_mut(&egui::FontFamily::Monospace)
                    .unwrap()
                    .insert(0, "chinese".to_owned());
            }

            ctx.set_fonts(fonts);

            // 设置 DPI 缩放 - 使 egui 与 macroquad 坐标系统对齐
            // macroquad 会根据系统 DPI 自动处理,egui 也需要同步
            let dpi_scale = screen_dpi_scale();
            ctx.set_pixels_per_point(dpi_scale);

            // 设置全局字体大小
            let mut style = (*ctx.style()).clone();
            style.text_styles = [
                (
                    egui::TextStyle::Heading,
                    egui::FontId::new(24.0, egui::FontFamily::Proportional),
                ),
                (
                    egui::TextStyle::Body,
                    egui::FontId::new(16.0, egui::FontFamily::Proportional),
                ),
                (
                    egui::TextStyle::Monospace,
                    egui::FontId::new(14.0, egui::FontFamily::Monospace),
                ),
                (
                    egui::TextStyle::Button,
                    egui::FontId::new(16.0, egui::FontFamily::Proportional),
                ),
                (
                    egui::TextStyle::Small,
                    egui::FontId::new(12.0, egui::FontFamily::Proportional),
                ),
            ]
            .into();
            ctx.set_style(style);
        });

        println!("🎬 进入登录界面");
        Ok(())
    }

    fn on_exit(&mut self) -> GameResult {
        println!("🎬 离开登录界面");
        Ok(())
    }

    fn update(&mut self, dt: f32) -> GameResult<SceneTransition> {
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

        self.handle_input()?;

        Ok(SceneTransition::None)
    }

    fn render(&mut self) -> GameResult {
        clear_background(BLACK);

        // ========== 【macroquad 渲染层】 ==========
        // 1. 绘制背景动画 (ChrSel 库)
        let frame_index = if self.animation_playing {
            self.background_frame
        } else {
            0
        };

        // ✅ 新 API
        if let Some(info) = LibraryName::ChrSel.get_texture(frame_index) {
            if let Some(ref texture) = info.image {
                draw_texture(texture, 0.0, 0.0, WHITE);
            }
        }

        // 2. 绘制登录对话框背景纹理 (Prguse 和 Title 库)
        if self.show_login_dialog {
            self.draw_login_background();
        }

        // 3. 对话框背景现在由 Dialog trait 内部处理
        // (NewAccountDialog 和 ChangePasswordDialog 在 show() 中绘制背景)

        // ========== 【egui 交互层】 ==========
        egui_macroquad::ui(|ctx| {
            // 登录对话框 (使用 Dialog trait)
            self.login_dialog.show(ctx, &mut self.show_login_dialog);
            
            // 检查登录对话框事件
            let login_event = self.login_dialog.take_event();
            match login_event {
                LoginDialogEvent::Login => {
                    self.on_login_clicked();
                },
                LoginDialogEvent::NewAccount => {
                    self.show_new_account = true;
                    self.show_login_dialog = false;
                },
                LoginDialogEvent::ChangePassword => {
                    self.show_change_password = true;
                    self.show_login_dialog = false;
                },
                LoginDialogEvent::None => {},
            }

            // 其他对话框 (使用 Dialog trait)
            self.new_account_dialog.show(ctx, &mut self.show_new_account);
            self.change_password_dialog.show(ctx, &mut self.show_change_password);
            self.message_box.show(ctx, &mut self.show_message_box);
            
            // 检查对话框关闭后恢复登录对话框显示
            // 仅当没有动画播放且所有对话框都关闭时，才显示登录对话框
            if !self.animation_playing 
                && !self.show_new_account 
                && !self.show_change_password 
                && !self.show_message_box {
                self.show_login_dialog = true;
            }
        });

        // 绘制 egui
        egui_macroquad::draw();

        Ok(())
    }

    fn handle_input(&mut self) -> GameResult {
        if is_key_pressed(KeyCode::Escape) {
            std::process::exit(0);
        }
        Ok(())
    }
}
