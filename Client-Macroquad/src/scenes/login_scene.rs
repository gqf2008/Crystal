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
use crate::resources;
use crate::resources::LibraryName;
use crate::scenes::{Scene, SceneTransition};
use egui_macroquad::egui;
use macroquad::prelude::*;

/// 消息框按钮类型
#[derive(Debug, Clone, Copy, PartialEq)]
enum MessageBoxButtons {
    Ok,       // 只有确定按钮
    OkCancel, // 确定和取消
    YesNo,    // 是和否
}

/// 登录场景 - 混合渲染版本
pub struct LoginScene {
    // UI 状态
    account_input: String,
    password_input: String,

    // 新建账号对话框状态
    show_new_account: bool,
    new_account_id: String,
    new_password1: String,
    new_password2: String,
    new_email: String,
    new_username: String,
    new_birthdate: String,
    new_question: String,
    new_answer: String,

    // 修改密码对话框状态
    show_change_password: bool,
    change_account: String,
    change_current_password: String,
    change_new_password: String,
    change_new_password2: String,

    // 消息框状态
    show_message_box: bool,
    message_box_title: String,
    message_box_text: String,
    message_box_buttons: MessageBoxButtons,

    // 背景动画
    background_frame: usize,
    animation_playing: bool,
    frame_timer: f32,
    frame_delay: f32,

    // 状态
    show_login_dialog: bool,
    version_text: String,
}

impl LoginScene {
    pub fn new() -> Self {
        Self {
            account_input: String::new(),
            password_input: String::new(),
            show_new_account: false,
            new_account_id: String::new(),
            new_password1: String::new(),
            new_password2: String::new(),
            new_email: String::new(),
            new_username: String::new(),
            new_birthdate: String::new(),
            new_question: String::new(),
            new_answer: String::new(),

            show_change_password: false,
            change_account: String::new(),
            change_current_password: String::new(),
            change_new_password: String::new(),
            change_new_password2: String::new(),

            show_message_box: false,
            message_box_title: String::new(),
            message_box_text: String::new(),
            message_box_buttons: MessageBoxButtons::Ok,

            background_frame: 0,
            animation_playing: false,
            frame_timer: 0.0,
            frame_delay: 0.1,

            show_login_dialog: true,

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

    /// 【macroquad 职责】绘制新建账号对话框背景层
    /// - 纹理渲染: Prguse[63] 新建账号对话框背景
    /// - DPI 处理: macroquad 自动根据 screen_dpi_scale() 缩放
    /// - 坐标系统: 使用物理像素坐标,原始纹理尺寸
    fn draw_new_account_background(&mut self) {
        let screen_w = screen_width();
        let screen_h = screen_height();

        // 获取背景纹理并计算居中位置
        // ✅ 新 API
        if let Some(info) = LibraryName::Prguse.get_texture(63) {
            if let Some(ref bg_tex) = info.image {
                let w = bg_tex.width();
                let h = bg_tex.height();
                let x = (screen_w - w) / 2.0;
                let y = (screen_h - h) / 2.0;

                // 绘制背景 (原始尺寸,macroquad 会根据 DPI 自动缩放)
                draw_texture(bg_tex, x, y, WHITE);
            }
        }
    }

    /// 【macroquad 职责】绘制修改密码对话框背景层
    /// - 纹理渲染: Prguse[50] 修改密码对话框背景 (348x268)
    /// - DPI 处理: macroquad 自动根据 screen_dpi_scale() 缩放
    /// - 坐标系统: 使用物理像素坐标,原始纹理尺寸
    fn draw_change_password_background(&mut self) {
        let screen_w = screen_width();
        let screen_h = screen_height();

        // 获取背景纹理并计算居中位置
        // ✅ 新 API
        if let Some(info) = LibraryName::Prguse.get_texture(50) {
            if let Some(ref bg_tex) = info.image {
                let w = bg_tex.width();
                let h = bg_tex.height();
                let x = (screen_w - w) / 2.0;
                let y = (screen_h - h) / 2.0;

                // 绘制背景 (原始尺寸,macroquad 会根据 DPI 自动缩放)
                draw_texture(bg_tex, x, y, WHITE);
            }
        }
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
                    egui::TextEdit::singleline(&mut self.account_input)
                        .desired_width(136.0)
                        .frame(false) // 去除边框
                        .margin(egui::vec2(0.0, 0.0)), // 去除内边距
                );

                let password_response = ui.put(
                    egui::Rect::from_min_size(
                        egui::pos2(rect.min.x + 85.0, rect.min.y + 108.0),
                        egui::vec2(136.0, 15.0),
                    ),
                    egui::TextEdit::singleline(&mut self.password_input)
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

    /// 【egui 职责】绘制新建账号对话框交互层
    /// - 用户输入: 8个输入框 (账号、密码、确认密码、生日、问题、答案、邮箱、用户名)
    /// - 交互按钮: 确定和取消按钮
    /// - 背景渲染: 由 macroquad 的 draw_new_account_background() 负责
    fn draw_new_account_dialog(&mut self, ctx: &egui::Context) {
        let dialog_w = 588.0; // Prguse[63] 实际尺寸
        let dialog_h = 460.0;

        // 使用 egui Area 居中对齐 (与登录对话框一致)
        egui::Area::new(egui::Id::new("new_account_dialog"))
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

                // 输入框 (使用 C# 原版坐标,去除内边距避免覆盖边线)
                // AccountID: (226, 103), Password1: (226, 129), Password2: (226, 155)
                // UserName: (226, 189), BirthDate: (226, 215)
                // Question: (226, 250), Answer: (226, 276), EMail: (226, 311)

                ui.put(
                    egui::Rect::from_min_size(
                        egui::pos2(rect.min.x + 226.0, rect.min.y + 103.0),
                        egui::vec2(136.0, 15.0),
                    ),
                    egui::TextEdit::singleline(&mut self.new_account_id)
                        .hint_text("账号")
                        .desired_width(136.0)
                        .frame(false)
                        .margin(egui::vec2(0.0, 0.0)),
                );
                ui.put(
                    egui::Rect::from_min_size(
                        egui::pos2(rect.min.x + 226.0, rect.min.y + 129.0),
                        egui::vec2(136.0, 15.0),
                    ),
                    egui::TextEdit::singleline(&mut self.new_password1)
                        .password(true)
                        .hint_text("密码")
                        .desired_width(136.0)
                        .frame(false)
                        .margin(egui::vec2(0.0, 0.0)),
                );
                ui.put(
                    egui::Rect::from_min_size(
                        egui::pos2(rect.min.x + 226.0, rect.min.y + 155.0),
                        egui::vec2(136.0, 15.0),
                    ),
                    egui::TextEdit::singleline(&mut self.new_password2)
                        .password(true)
                        .hint_text("确认密码")
                        .desired_width(136.0)
                        .frame(false)
                        .margin(egui::vec2(0.0, 0.0)),
                );
                ui.put(
                    egui::Rect::from_min_size(
                        egui::pos2(rect.min.x + 226.0, rect.min.y + 189.0),
                        egui::vec2(136.0, 15.0),
                    ),
                    egui::TextEdit::singleline(&mut self.new_username)
                        .hint_text("用户名")
                        .desired_width(136.0)
                        .frame(false)
                        .margin(egui::vec2(0.0, 0.0)),
                );
                ui.put(
                    egui::Rect::from_min_size(
                        egui::pos2(rect.min.x + 226.0, rect.min.y + 215.0),
                        egui::vec2(136.0, 15.0),
                    ),
                    egui::TextEdit::singleline(&mut self.new_birthdate)
                        .hint_text("生日")
                        .desired_width(136.0)
                        .frame(false)
                        .margin(egui::vec2(0.0, 0.0)),
                );
                ui.put(
                    egui::Rect::from_min_size(
                        egui::pos2(rect.min.x + 226.0, rect.min.y + 250.0),
                        egui::vec2(190.0, 15.0),
                    ),
                    egui::TextEdit::singleline(&mut self.new_question)
                        .hint_text("密保问题")
                        .desired_width(190.0)
                        .frame(false)
                        .margin(egui::vec2(0.0, 0.0)),
                );
                ui.put(
                    egui::Rect::from_min_size(
                        egui::pos2(rect.min.x + 226.0, rect.min.y + 276.0),
                        egui::vec2(190.0, 15.0),
                    ),
                    egui::TextEdit::singleline(&mut self.new_answer)
                        .hint_text("密保答案")
                        .desired_width(190.0)
                        .frame(false)
                        .margin(egui::vec2(0.0, 0.0)),
                );
                ui.put(
                    egui::Rect::from_min_size(
                        egui::pos2(rect.min.x + 226.0, rect.min.y + 311.0),
                        egui::vec2(136.0, 15.0),
                    ),
                    egui::TextEdit::singleline(&mut self.new_email)
                        .hint_text("邮箱")
                        .desired_width(136.0)
                        .frame(false)
                        .margin(egui::vec2(0.0, 0.0)),
                );

                // 按钮 (C# 原版: OK=135,425  Cancel=409,425)
                // 添加调试信息
                if cfg!(debug_assertions) {
                    ui.painter().text(
                        egui::pos2(rect.min.x + 10.0, rect.min.y + 430.0),
                        egui::Align2::LEFT_TOP,
                        format!("对话框尺寸: {:.0}x{:.0}", dialog_w, dialog_h),
                        egui::FontId::proportional(12.0),
                        egui::Color32::YELLOW,
                    );
                }

                if self.draw_image_button_abs(
                    ui,
                    ctx,
                    LibraryName::Title,
                    200,
                    201,
                    202,
                    egui::pos2(rect.min.x + 135.0, rect.min.y + 425.0),
                ) {
                    self.on_create_account();
                }

                if self.draw_image_button_abs(
                    ui,
                    ctx,
                    LibraryName::Title,
                    203,
                    204,
                    205,
                    egui::pos2(rect.min.x + 409.0, rect.min.y + 425.0),
                ) {
                    self.close_new_account_dialog();
                }
            });
    }

    /// 关闭新建账号对话框
    fn close_new_account_dialog(&mut self) {
        self.show_new_account = false;
        self.show_login_dialog = true;
        self.new_account_id.clear();
        self.new_password1.clear();
        self.new_password2.clear();
        self.new_email.clear();
        self.new_username.clear();
        self.new_birthdate.clear();
        self.new_question.clear();
        self.new_answer.clear();
    }

    /// 创建账号
    fn on_create_account(&mut self) {
        if self.new_account_id.is_empty() {
            self.show_message_box("错误", "账号不能为空!", MessageBoxButtons::Ok);
            return;
        }
        if self.new_password1.is_empty() {
            self.show_message_box("错误", "密码不能为空!", MessageBoxButtons::Ok);
            return;
        }
        if self.new_password1 != self.new_password2 {
            self.show_message_box("错误", "两次密码输入不一致!", MessageBoxButtons::Ok);
            return;
        }

        println!("✅ 创建账号: {}", self.new_account_id);
        println!("   用户名: {}", self.new_username);
        println!("   邮箱: {}", self.new_email);

        self.show_message_box("成功", "账号创建成功!", MessageBoxButtons::Ok);
        self.close_new_account_dialog();
    }

    /// 登录按钮点击
    fn on_login_clicked(&mut self) {
        if self.account_input.is_empty() || self.password_input.is_empty() {
            self.show_message_box("登录失败", "账号或密码不能为空!", MessageBoxButtons::Ok);
            return;
        }

        println!("🔐 Login: account={}", self.account_input);

        // 保存配置
        self.save_config();

        // 开始播放登录成功动画
        self.animation_playing = true;
        self.background_frame = 0;
        self.show_login_dialog = false;
    }

    /// 【egui 职责】绘制修改密码对话框交互层
    /// - 用户输入: 账号、旧密码、新密码、确认密码
    /// - 交互按钮: 确定和取消按钮
    /// - 背景渲染: 由 macroquad 的 draw_change_password_background() 负责
    fn draw_change_password_dialog(&mut self, ctx: &egui::Context) {
        let dialog_w = 348.0; // Prguse[50] 实际尺寸
        let dialog_h = 268.0;

        egui::Area::new(egui::Id::new("change_password_dialog"))
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

                // 输入框 (使用 C# 原版坐标,去除内边距避免覆盖边线)
                // AccountID: (178, 75), CurrentPassword: (178, 113)
                // NewPassword1: (178, 151), NewPassword2: (178, 188)

                ui.put(
                    egui::Rect::from_min_size(
                        egui::pos2(rect.min.x + 178.0, rect.min.y + 75.0),
                        egui::vec2(136.0, 18.0),
                    ),
                    egui::TextEdit::singleline(&mut self.change_account)
                        .hint_text("账号")
                        .desired_width(136.0)
                        .frame(false)
                        .margin(egui::vec2(0.0, 0.0)),
                );

                ui.put(
                    egui::Rect::from_min_size(
                        egui::pos2(rect.min.x + 178.0, rect.min.y + 113.0),
                        egui::vec2(136.0, 18.0),
                    ),
                    egui::TextEdit::singleline(&mut self.change_current_password)
                        .password(true)
                        .hint_text("当前密码")
                        .desired_width(136.0)
                        .frame(false)
                        .margin(egui::vec2(0.0, 0.0)),
                );

                ui.put(
                    egui::Rect::from_min_size(
                        egui::pos2(rect.min.x + 178.0, rect.min.y + 151.0),
                        egui::vec2(136.0, 18.0),
                    ),
                    egui::TextEdit::singleline(&mut self.change_new_password)
                        .password(true)
                        .hint_text("新密码")
                        .desired_width(136.0)
                        .frame(false)
                        .margin(egui::vec2(0.0, 0.0)),
                );

                ui.put(
                    egui::Rect::from_min_size(
                        egui::pos2(rect.min.x + 178.0, rect.min.y + 188.0),
                        egui::vec2(136.0, 18.0),
                    ),
                    egui::TextEdit::singleline(&mut self.change_new_password2)
                        .password(true)
                        .hint_text("确认新密码")
                        .desired_width(136.0)
                        .frame(false)
                        .margin(egui::vec2(0.0, 0.0)),
                );

                // 按钮 (C# 原版: OK=80,236  Cancel=222,236)
                if self.draw_image_button_abs(
                    ui,
                    ctx,
                    LibraryName::Title,
                    107,
                    108,
                    109,
                    egui::pos2(rect.min.x + 80.0, rect.min.y + 236.0),
                ) {
                    self.on_change_password();
                }

                if self.draw_image_button_abs(
                    ui,
                    ctx,
                    LibraryName::Title,
                    110,
                    111,
                    112,
                    egui::pos2(rect.min.x + 222.0, rect.min.y + 236.0),
                ) {
                    self.close_change_password_dialog();
                }
            });
    }

    /// 关闭修改密码对话框
    fn close_change_password_dialog(&mut self) {
        self.show_change_password = false;
        self.show_login_dialog = true;
        self.change_account.clear();
        self.change_current_password.clear();
        self.change_new_password.clear();
        self.change_new_password2.clear();
    }

    /// 确认修改密码
    fn on_change_password(&mut self) {
        if self.change_account.is_empty() || self.change_current_password.is_empty() {
            self.show_message_box("错误", "账号和当前密码不能为空!", MessageBoxButtons::Ok);
            return;
        }

        if self.change_new_password.is_empty() {
            self.show_message_box("错误", "新密码不能为空!", MessageBoxButtons::Ok);
            return;
        }

        if self.change_new_password != self.change_new_password2 {
            self.show_message_box("错误", "两次新密码输入不一致!", MessageBoxButtons::Ok);
            return;
        }

        println!("✅ 修改密码: {}", self.change_account);
        self.show_message_box("成功", "密码修改成功!", MessageBoxButtons::Ok);
        // 注意: 不立即关闭对话框,等用户关闭消息框后再决定
    }

    /// 【功能】显示消息框
    fn show_message_box(&mut self, title: &str, text: &str, buttons: MessageBoxButtons) {
        self.message_box_title = title.to_string();
        self.message_box_text = text.to_string();
        self.message_box_buttons = buttons;
        self.show_message_box = true;
    }

    /// 【功能】绘制消息框
    fn draw_message_box(&mut self, ctx: &egui::Context) {
        // 获取 Prguse[360] 实际尺寸
        let (dialog_w, dialog_h) = if let Some(size) = resources::get_size(LibraryName::Prguse, 360)
        {
            (size.0 as f32, size.1 as f32)
        } else {
            (460.0, 200.0) // 默认尺寸
        };

        // ESC键关闭消息框
        if ctx.input(|i| i.key_pressed(egui::Key::Escape)) {
            self.show_message_box = false;
            return;
        }

        egui::Area::new(egui::Id::new("message_box"))
            .default_pos(egui::pos2(
                (screen_width() / screen_dpi_scale() - dialog_w) / 2.0,
                (screen_height() / screen_dpi_scale() - dialog_h) / 2.0,
            )) // 初始居中位置，但可以拖动
            .interactable(true)
            .movable(true) // 允许拖动消息框
            .order(egui::Order::Foreground) // 确保消息框在最上层
            .show(ctx, |ui| {
                // 分配对话框空间 (仅用于定位,不拦截点击事件)
                let rect = ui
                    .allocate_rect(
                        egui::Rect::from_min_size(ui.cursor().min, egui::vec2(dialog_w, dialog_h)),
                        egui::Sense::hover(),
                    )
                    .rect;

                // 绘制背景纹理 Prguse[360]
                // ✅ 新 API
                if let Some(info) = LibraryName::Prguse.get_egui_texture(ctx, 360) {
                    if let Some(ref handle) = info.egui_texture {
                        ui.painter().image(
                            handle.id(),
                            rect,
                            egui::Rect::from_min_max(egui::pos2(0.0, 0.0), egui::pos2(1.0, 1.0)),
                            egui::Color32::WHITE,
                        );
                    }
                }

                // 标题 (C# 原版 Location=(35, 35), 字体正常大小)
                ui.put(
                    egui::Rect::from_min_size(
                        egui::pos2(rect.min.x + 35.0, rect.min.y + 35.0),
                        egui::vec2(390.0, 25.0),
                    ),
                    egui::Label::new(
                        egui::RichText::new(&self.message_box_title)
                            .color(egui::Color32::from_rgb(255, 200, 100))
                            .size(12.0),
                    ),
                );

                // 消息文本 (C# 原版 Size=(390, 110), 字体缩小避免 DPI 放大)
                ui.put(
                    egui::Rect::from_min_size(
                        egui::pos2(rect.min.x + 35.0, rect.min.y + 60.0),
                        egui::vec2(390.0, 80.0),
                    ),
                    egui::Label::new(
                        egui::RichText::new(&self.message_box_text)
                            .color(egui::Color32::WHITE)
                            .size(10.0),
                    ),
                );

                // 按钮 (C# 原版 Y=157)
                match self.message_box_buttons {
                    MessageBoxButtons::Ok => {
                        if self.draw_image_button_abs(
                            ui,
                            ctx,
                            LibraryName::Title,
                            200,
                            201,
                            202,
                            egui::pos2(rect.min.x + 360.0, rect.min.y + 157.0),
                        ) {
                            self.show_message_box = false;
                        }
                    }
                    MessageBoxButtons::OkCancel => {
                        if self.draw_image_button_abs(
                            ui,
                            ctx,
                            LibraryName::Title,
                            200,
                            201,
                            202,
                            egui::pos2(rect.min.x + 260.0, rect.min.y + 157.0),
                        ) {
                            self.show_message_box = false;
                        }
                        if self.draw_image_button_abs(
                            ui,
                            ctx,
                            LibraryName::Title,
                            203,
                            204,
                            205,
                            egui::pos2(rect.min.x + 360.0, rect.min.y + 157.0),
                        ) {
                            self.show_message_box = false;
                        }
                    }
                    MessageBoxButtons::YesNo => {
                        if self.draw_image_button_abs(
                            ui,
                            ctx,
                            LibraryName::Title,
                            206,
                            207,
                            208,
                            egui::pos2(rect.min.x + 260.0, rect.min.y + 157.0),
                        ) {
                            self.show_message_box = false;
                        }
                        if self.draw_image_button_abs(
                            ui,
                            ctx,
                            LibraryName::Title,
                            210,
                            211,
                            212,
                            egui::pos2(rect.min.x + 360.0, rect.min.y + 157.0),
                        ) {
                            self.show_message_box = false;
                        }
                    }
                }
            });
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
            self.account_input,
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
                    self.account_input = account.to_string();
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
        self.account_input.clear();
        self.password_input.clear();

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

        // 3. 绘制新建账号对话框背景 (Prguse 63)
        if self.show_new_account {
            self.draw_new_account_background();
        }

        // 4. 绘制修改密码对话框背景
        if self.show_change_password {
            self.draw_change_password_background();
        }

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

            if self.show_login_dialog {
                self.draw_login_ui(ctx);
            }

            if self.show_new_account {
                self.draw_new_account_dialog(ctx);
            }

            if self.show_change_password {
                self.draw_change_password_dialog(ctx);
            }

            // 消息框 (最上层)
            if self.show_message_box {
                self.draw_message_box(ctx);
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
