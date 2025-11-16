use crate::resources::libraries::LibraryName;
use egui_macroquad::egui;
use macroquad::prelude::*;

/// 登录对话框按钮事件
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum LoginDialogEvent {
    None,
    Login,
    NewAccount,
    ChangePassword,
}

/// 登录对话框
pub struct LoginDialog {
    id: String,
    pub account: String,
    pub password: String,
    pub remember_password: bool,
}

impl LoginDialog {
    pub fn new() -> Self {
        Self {
            id: "login_dialog".to_string(),
            account: String::new(),
            password: String::new(),
            remember_password: false,
        }
    }
    
    /// 重置对话框
    pub fn reset(&mut self) {
        self.password.clear();
    }
    
    /// 绘制图像按钮
    fn draw_image_button(
        &self,
        ui: &mut egui::Ui,
        ctx: &egui::Context,
        normal_idx: usize,
        hover_idx: usize,
        pressed_idx: usize,
        abs_pos: egui::Pos2,
    ) -> bool {
        if let Some(info) = LibraryName::Title.get_egui_texture(ctx, normal_idx) {
            if let Some(ref handle) = info.egui_texture {
                let texture_size = handle.size_vec2();
                let button_rect = egui::Rect::from_min_size(abs_pos, texture_size);
                
                let button_id = format!("{}_{}", self.id, normal_idx);
                let response = ui.interact(button_rect, egui::Id::new(button_id), egui::Sense::click());
                
                let texture_idx = if response.is_pointer_button_down_on() {
                    pressed_idx
                } else if response.hovered() {
                    hover_idx
                } else {
                    normal_idx
                };
                
                if let Some(btn_info) = LibraryName::Title.get_egui_texture(ctx, texture_idx) {
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
    
    /// 显示登录对话框并返回事件
    pub fn show(&mut self, ctx: &egui::Context, open: &mut bool) -> LoginDialogEvent {
        if !*open {
            return LoginDialogEvent::None;
        }
        
        let mut event = LoginDialogEvent::None;
        let dialog_w = 328.0; // Prguse[1084] 实际尺寸
        let dialog_h = 220.0;
        
        // ESC键关闭（退出程序）
        if ctx.input(|i| i.key_pressed(egui::Key::Escape)) {
            // 登录对话框的ESC由外部处理
            return LoginDialogEvent::None;
        }
        
        egui::Area::new(egui::Id::new(&self.id))
            .anchor(egui::Align2::CENTER_CENTER, egui::vec2(0.0, 0.0))
            .interactable(true)
            .order(egui::Order::Foreground)
            .show(ctx, |ui| {
                let rect = ui.allocate_rect(
                    egui::Rect::from_min_size(ui.cursor().min, egui::vec2(dialog_w, dialog_h)),
                    egui::Sense::hover(),
                ).rect;
                
                // 绘制背景 Prguse[1084]
                if let Some(info) = LibraryName::Prguse.get_egui_texture(ctx, 1084) {
                    if let Some(ref handle) = info.egui_texture {
                        ui.painter().image(
                            handle.id(),
                            rect,
                            egui::Rect::from_min_max(egui::pos2(0.0, 0.0), egui::pos2(1.0, 1.0)),
                            egui::Color32::WHITE,
                        );
                    }
                }
                
                // 绘制标题 Title[30] - 居中对齐
                if let Some(info) = LibraryName::Title.get_egui_texture(ctx, 30) {
                    if let Some(ref handle) = info.egui_texture {
                        let size = handle.size_vec2();
                        let title_rect = egui::Rect::from_min_size(
                            egui::pos2(rect.min.x + (dialog_w - size.x) / 2.0, rect.min.y + 12.0),
                            size,
                        );
                        ui.painter().image(
                            handle.id(),
                            title_rect,
                            egui::Rect::from_min_max(egui::pos2(0.0, 0.0), egui::pos2(1.0, 1.0)),
                            egui::Color32::WHITE,
                        );
                    }
                }
                
                // 绘制ID标签 Title[31] at (52, 83)
                if let Some(info) = LibraryName::Title.get_egui_texture(ctx, 31) {
                    if let Some(ref handle) = info.egui_texture {
                        let size = handle.size_vec2();
                        let title_rect = egui::Rect::from_min_size(
                            egui::pos2(rect.min.x + 52.0, rect.min.y + 83.0),
                            size,
                        );
                        ui.painter().image(
                            handle.id(),
                            title_rect,
                            egui::Rect::from_min_max(egui::pos2(0.0, 0.0), egui::pos2(1.0, 1.0)),
                            egui::Color32::WHITE,
                        );
                    }
                }
                
                // 绘制密码标签 Title[32] at (43, 105)
                if let Some(info) = LibraryName::Title.get_egui_texture(ctx, 32) {
                    if let Some(ref handle) = info.egui_texture {
                        let size = handle.size_vec2();
                        let title_rect = egui::Rect::from_min_size(
                            egui::pos2(rect.min.x + 43.0, rect.min.y + 105.0),
                            size,
                        );
                        ui.painter().image(
                            handle.id(),
                            title_rect,
                            egui::Rect::from_min_max(egui::pos2(0.0, 0.0), egui::pos2(1.0, 1.0)),
                            egui::Color32::WHITE,
                        );
                    }
                }
                
                // 账号输入框 (C# 原版: 85, 85)
                ui.put(
                    egui::Rect::from_min_size(
                        egui::pos2(rect.min.x + 85.0, rect.min.y + 85.0),
                        egui::vec2(136.0, 15.0),
                    ),
                    egui::TextEdit::singleline(&mut self.account)
                        .hint_text("账号")
                        .desired_width(136.0)
                        .frame(false)
                        .margin(egui::vec2(0.0, 0.0)),
                );
                
                // 密码输入框 (C# 原版: 85, 108)
                ui.put(
                    egui::Rect::from_min_size(
                        egui::pos2(rect.min.x + 85.0, rect.min.y + 108.0),
                        egui::vec2(136.0, 15.0),
                    ),
                    egui::TextEdit::singleline(&mut self.password)
                        .password(true)
                        .hint_text("密码")
                        .desired_width(136.0)
                        .frame(false)
                        .margin(egui::vec2(0.0, 0.0)),
                );
                
                // 按钮 (C# 原版位置)
                // OK按钮: Title[320/321/322] at (227, 81)
                if self.draw_image_button(ui, ctx, 320, 321, 322,
                    egui::pos2(rect.min.x + 227.0, rect.min.y + 81.0)) {
                    event = LoginDialogEvent::Login;
                }
                
                // New Account按钮: Title[323/324/325] at (60, 163)  
                if self.draw_image_button(ui, ctx, 323, 324, 325,
                    egui::pos2(rect.min.x + 60.0, rect.min.y + 163.0)) {
                    event = LoginDialogEvent::NewAccount;
                }
                
                // Change Password按钮: Title[326/327/328] at (166, 163)
                if self.draw_image_button(ui, ctx, 326, 327, 328,
                    egui::pos2(rect.min.x + 166.0, rect.min.y + 163.0)) {
                    event = LoginDialogEvent::ChangePassword;
                }
                
                // Exit按钮: Title[329/330/331] at (166, 189)
                if self.draw_image_button(ui, ctx, 329, 330, 331,
                    egui::pos2(rect.min.x + 166.0, rect.min.y + 189.0)) {
                    *open = false;
                }
                
                // InputKey按钮: Title[332/333/334] at (60, 189)
                // TODO: 实现激活码输入功能
                self.draw_image_button(ui, ctx, 332, 333, 334,
                    egui::pos2(rect.min.x + 60.0, rect.min.y + 189.0));
            });
        
        event
    }
}
