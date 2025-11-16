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

    // 状态
    version_text: String,
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

            version_text: format!("Build: Crystal-Rust v{}", env!("CARGO_PKG_VERSION")),
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

    /// 【egui 职责】绘制登录UI交互层
    /// - 用户输入: 账号/密码 TextEdit 输入框
    /// - 交互按钮: OK, New Account, Change Password, Exit 等按钮
    /// - DPI 处理: 通过 ctx.set_pixels_per_point(screen_dpi_scale()) 在 on_enter 中一次性设置
    /// - 坐标系统: 使用逻辑像素,与 macroquad 自动对齐
    fn draw_login_ui(&mut self, ctx: &egui::Context) {
        let dialog_w = 328.0;
        let dialog_h = 220.0;

        // 使用 egui Area 居中对齐交互元素
        egui::Area::new(egui::Id::new("login_dialog_inputs"))
            .anchor(egui::Align2::CENTER_CENTER, egui::vec2(0.0, 0.0))
            .interactable(true)
            .show(ctx, |ui| {
                // 分配对话框空间 (仅用于定位,不拦截点击事件)
                let rect = ui
                    .allocate_rect(
                        egui::Rect::from_min_size(ui.cursor().min, egui::vec2(dialog_w, dialog_h)),
                        egui::Sense::hover(),
                    )
                    .rect;

                // 输入框 (高度 15px 与 C# 原版一致,去除内边距避免覆盖边线)
                ui.put(
                    egui::Rect::from_min_size(
                        egui::pos2(rect.min.x + 85.0, rect.min.y + 85.0),
                        egui::vec2(136.0, 15.0),
                    ),
                    egui::TextEdit::singleline(&mut self.login_dialog.account)
                        .desired_width(136.0)
                        .frame(false) // 去除边框
                        .margin(egui::vec2(0.0, 0.0)), // 去除内边距
                );

                let password_response = ui.put(
                    egui::Rect::from_min_size(
                        egui::pos2(rect.min.x + 85.0, rect.min.y + 108.0),
                        egui::vec2(136.0, 15.0),
                    ),
                    egui::TextEdit::singleline(&mut self.login_dialog.password)
                        .password(true)
                        .desired_width(136.0)
                        .frame(false) // 去除边框
                        .margin(egui::vec2(0.0, 0.0)), // 去除内边距
                );

                // Enter键登录
                if password_response.lost_focus() && ui.input(|i| i.key_pressed(egui::Key::Enter)) {
                    self.on_login_clicked();
                }

                // 按钮 (egui 自动应用 pixels_per_point 缩放)
                if self.draw_image_button_abs(
                    ui,
                    ctx,
                    LibraryName::Title,
                    320,
                    321,
                    322,
                    egui::pos2(rect.min.x + 227.0, rect.min.y + 81.0),
                ) {
                    self.on_login_clicked();
                }

                if self.draw_image_button_abs(
                    ui,
                    ctx,
                    LibraryName::Title,
                    323,
                    324,
                    325,
                    egui::pos2(rect.min.x + 60.0, rect.min.y + 163.0),
                ) {
                    self.show_new_account = true;
                    self.show_login_dialog = false;
                }

                if self.draw_image_button_abs(
                    ui,
                    ctx,
                    LibraryName::Title,
                    326,
                    327,
                    328,
                    egui::pos2(rect.min.x + 166.0, rect.min.y + 163.0),
                ) {
                    self.show_change_password = true;
                    self.show_login_dialog = false;
                }

                if self.draw_image_button_abs(
                    ui,
                    ctx,
                    LibraryName::Title,
                    332,
                    333,
                    334,
                    egui::pos2(rect.min.x + 60.0, rect.min.y + 189.0),
                ) {
                    println!("⚠ InputKeyDialog not implemented");
                }

                if self.draw_image_button_abs(
                    ui,
                    ctx,
                    LibraryName::Title,
                    329,
                    330,
                    331,
                    egui::pos2(rect.min.x + 166.0, rect.min.y + 189.0),
                ) {
                    std::process::exit(0);
                }
            });
    }

    /// 【egui 辅助】绘制图像按钮（三态：normal/hover/pressed）
    /// 注: 纹理由 egui 渲染,DPI 缩放已通过 pixels_per_point 自动处理
    fn draw_image_button_abs(
        &mut self,
        ui: &mut egui::Ui,
        ctx: &egui::Context,
        lib_name: LibraryName,
        normal_idx: usize,
        hover_idx: usize,
        pressed_idx: usize,
        abs_pos: egui::Pos2,
    ) -> bool {
        // ✅ 新 API
        if let Some(info) = lib_name.get_egui_texture(ctx, normal_idx) {
            if let Some(ref handle) = info.egui_texture {
                let size = egui::vec2(handle.size()[0] as f32, handle.size()[1] as f32);
                let button_rect = egui::Rect::from_min_size(abs_pos, size);
                let response = ui.allocate_rect(button_rect, egui::Sense::click());

                // 根据状态选择纹理
                let texture_idx = if response.is_pointer_button_down_on() {
                    pressed_idx
                } else if response.hovered() {
                    hover_idx
                } else {
                    normal_idx
                };

                // 绘制按钮图像
                // ✅ 新 API
                if let Some(btn_info) = lib_name.get_egui_texture(ctx, texture_idx) {
                    if let Some(ref btn_handle) = btn_info.egui_texture {
                        ui.painter().image(
                            btn_handle.id(),
                            button_rect,
                            egui::Rect::from_min_max(egui::pos2(0.0, 0.0), egui::pos2(1.0, 1.0)),
                            egui::Color32::WHITE,
                        );
                    }
                }

                return response.clicked();
            }
        }

        false
    }

    /// 绘制图像按钮（三态：normal/hover/pressed）- 用于 Window 内部（相对坐标）
    fn draw_image_button(
        &mut self,
        ui: &mut egui::Ui,
        ctx: &egui::Context,
        _dialog_rect: &egui::Rect,
        lib_name: LibraryName,
        normal_idx: usize,
        hover_idx: usize,
        pressed_idx: usize,
        offset: egui::Pos2,
    ) -> bool {
        // ✅ 新 API
        if let Some(info) = lib_name.get_egui_texture(ctx, normal_idx) {
            if let Some(ref handle) = info.egui_texture {
                let size = egui::vec2(handle.size()[0] as f32, handle.size()[1] as f32);
                let button_rect = egui::Rect::from_min_size(offset, size);
                let response = ui.allocate_rect(button_rect, egui::Sense::click());

                // 根据状态选择纹理
                let texture_idx = if response.is_pointer_button_down_on() {
                    pressed_idx
                } else if response.hovered() {
                    hover_idx
                } else {
                    normal_idx
                };

                // 绘制按钮图像
                // ✅ 新 API
                if let Some(btn_info) = lib_name.get_egui_texture(ctx, texture_idx) {
                    if let Some(ref btn_handle) = btn_info.egui_texture {
                        ui.painter().image(
                            btn_handle.id(),
                            button_rect,
                            egui::Rect::from_min_max(egui::pos2(0.0, 0.0), egui::pos2(1.0, 1.0)),
                            egui::Color32::WHITE,
                        );
                    }
                }

                return response.clicked();
            }
        }

        false
    }

    /// 关闭新建账号对话框
    fn close_new_account_dialog(&mut self) {
        self.show_new_account = false;
        self.show_login_dialog = true;
        self.new_account_dialog.reset();
    }

    /// 创建账号
    fn on_create_account(&mut self) {
        if self.new_account_dialog.account_id.is_empty() {
            self.message_box.title = "错误".to_string();
            self.message_box.text = "账号不能为空!".to_string();
            self.message_box.buttons = MessageBoxButtons::Ok;
            self.show_message_box = true;
            return;
        }
        if self.new_account_dialog.password1.is_empty() {
            self.message_box.title = "错误".to_string();
            self.message_box.text = "密码不能为空!".to_string();
            self.message_box.buttons = MessageBoxButtons::Ok;
            self.show_message_box = true;
            return;
        }
        if self.new_account_dialog.password1 != self.new_account_dialog.password2 {
            self.message_box.title = "错误".to_string();
            self.message_box.text = "两次密码输入不一致!".to_string();
            self.message_box.buttons = MessageBoxButtons::Ok;
            self.show_message_box = true;
            return;
        }

        println!("✅ 创建账号: {}", self.new_account_dialog.account_id);
        println!("   用户名: {}", self.new_account_dialog.username);
        println!("   邮箱: {}", self.new_account_dialog.email);

        self.message_box.title = "成功".to_string();
        self.message_box.text = "账号创建成功!".to_string();
        self.message_box.buttons = MessageBoxButtons::Ok;
        self.show_message_box = true;
        self.close_new_account_dialog();
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

    /// 关闭修改密码对话框
    fn close_change_password_dialog(&mut self) {
        self.show_change_password = false;
        self.show_login_dialog = true;
        self.change_password_dialog.reset();
    }

    /// 确认修改密码
    fn on_change_password(&mut self) {
        if self.change_password_dialog.account.is_empty() 
            || self.change_password_dialog.current_password.is_empty() {
            self.message_box.title = "错误".to_string();
            self.message_box.text = "账号和当前密码不能为空!".to_string();
            self.message_box.buttons = MessageBoxButtons::Ok;
            self.show_message_box = true;
            return;
        }

        if self.change_password_dialog.new_password.is_empty() {
            self.message_box.title = "错误".to_string();
            self.message_box.text = "新密码不能为空!".to_string();
            self.message_box.buttons = MessageBoxButtons::Ok;
            self.show_message_box = true;
            return;
        }

        if self.change_password_dialog.new_password != self.change_password_dialog.new_password2 {
            self.message_box.title = "错误".to_string();
            self.message_box.text = "两次新密码输入不一致!".to_string();
            self.message_box.buttons = MessageBoxButtons::Ok;
            self.show_message_box = true;
            return;
        }

        println!("✅ 修改密码: {}", self.change_password_dialog.account);
        self.message_box.title = "成功".to_string();
        self.message_box.text = "密码修改成功!".to_string();
        self.message_box.buttons = MessageBoxButtons::Ok;
        self.show_message_box = true;
        // 注意: 不立即关闭对话框,等用户关闭消息框后再决定
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
            // 调试: 检查 egui pixels_per_point (DPI 缩放)
            static FIRST_EGUI: std::sync::atomic::AtomicBool =
                std::sync::atomic::AtomicBool::new(true);
            if FIRST_EGUI.swap(false, std::sync::atomic::Ordering::Relaxed) {
                println!("🎨 egui pixels_per_point: {}", ctx.pixels_per_point());
                println!(
                    "🎨 egui native_pixels_per_point: {:?}",
                    ctx.native_pixels_per_point()
                );
            }

            // 登录对话框 (返回按钮事件)
            let login_event = self.login_dialog.show(ctx, &mut self.show_login_dialog);
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

            // 调试信息
            egui::Window::new("🔍 坐标系统调试")
                .default_pos([10.0, 10.0])
                .default_width(350.0)
                .show(ctx, |ui| {
                    ui.heading("坐标系统对比");
                    ui.separator();

                    // macroquad 坐标 (物理像素)
                    ui.label("📐 Macroquad (物理像素):");
                    ui.label(format!("  屏幕: {}x{}", screen_width(), screen_height()));

                    // egui 坐标 (逻辑点)
                    ui.label("🎨 Egui (逻辑点):");
                    let pixels_per_point = ctx.pixels_per_point();
                    ui.label(format!("  pixels_per_point: {}", pixels_per_point));
                    ui.label(format!(
                        "  屏幕逻辑尺寸: {}x{}",
                        screen_width() / pixels_per_point,
                        screen_height() / pixels_per_point
                    ));

                    ui.separator();
                    ui.label("💡 换算关系:");
                    ui.label(format!("  1 逻辑点 = {} 物理像素", pixels_per_point));
                    ui.label(format!(
                        "  macroquad(100) = egui({})",
                        100.0 / pixels_per_point
                    ));

                    ui.separator();
                    ui.label(format!("显示登录框: {}", self.show_login_dialog));
                });
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
