// 修改密码对话框组件
use macroquad::prelude::*;
use egui_macroquad::egui;
use crate::resources::LibraryName;
use super::Dialog;

/// 修改密码对话框
pub struct ChangePasswordDialog {
    // 输入字段
    pub account: String,
    pub current_password: String,
    pub new_password: String,
    pub new_password2: String,
    
    // 对话框 ID
    id: String,
}

impl ChangePasswordDialog {
    /// 创建修改密码对话框
    pub fn new() -> Self {
        Self {
            account: String::new(),
            current_password: String::new(),
            new_password: String::new(),
            new_password2: String::new(),
            id: "change_password_dialog".to_string(),
        }
    }
    
    /// 重置所有字段
    pub fn reset(&mut self) {
        self.account.clear();
        self.current_password.clear();
        self.new_password.clear();
        self.new_password2.clear();
    }
    
    /// 绘制图像按钮(辅助方法)
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
}

impl Dialog for ChangePasswordDialog {
    fn show(&mut self, ctx: &egui::Context, open: &mut bool) {
        if !*open {
            return;
        }
        
        let dialog_w = 588.0; // Prguse[66] 实际尺寸
        let dialog_h = 308.0;
        
        // ESC键关闭对话框
        if ctx.input(|i| i.key_pressed(egui::Key::Escape)) {
            *open = false;
            return;
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
                
                // 绘制背景 Prguse[66]
                if let Some(info) = LibraryName::Prguse.get_egui_texture(ctx, 66) {
                    if let Some(ref handle) = info.egui_texture {
                        ui.painter().image(
                            handle.id(),
                            rect,
                            egui::Rect::from_min_max(egui::pos2(0.0, 0.0), egui::pos2(1.0, 1.0)),
                            egui::Color32::WHITE,
                        );
                    }
                }
                
                // 输入框 (使用原版坐标)
                // Account: (226, 103)
                ui.put(
                    egui::Rect::from_min_size(
                        egui::pos2(rect.min.x + 226.0, rect.min.y + 103.0),
                        egui::vec2(136.0, 15.0),
                    ),
                    egui::TextEdit::singleline(&mut self.account)
                        .hint_text("账号")
                        .desired_width(136.0)
                        .frame(false)
                        .margin(egui::vec2(0.0, 0.0)),
                );
                
                // Current Password: (226, 129)
                ui.put(
                    egui::Rect::from_min_size(
                        egui::pos2(rect.min.x + 226.0, rect.min.y + 129.0),
                        egui::vec2(136.0, 15.0),
                    ),
                    egui::TextEdit::singleline(&mut self.current_password)
                        .password(true)
                        .hint_text("当前密码")
                        .desired_width(136.0)
                        .frame(false)
                        .margin(egui::vec2(0.0, 0.0)),
                );
                
                // New Password: (226, 163)
                ui.put(
                    egui::Rect::from_min_size(
                        egui::pos2(rect.min.x + 226.0, rect.min.y + 163.0),
                        egui::vec2(136.0, 15.0),
                    ),
                    egui::TextEdit::singleline(&mut self.new_password)
                        .password(true)
                        .hint_text("新密码")
                        .desired_width(136.0)
                        .frame(false)
                        .margin(egui::vec2(0.0, 0.0)),
                );
                
                // Confirm New Password: (226, 189)
                ui.put(
                    egui::Rect::from_min_size(
                        egui::pos2(rect.min.x + 226.0, rect.min.y + 189.0),
                        egui::vec2(136.0, 15.0),
                    ),
                    egui::TextEdit::singleline(&mut self.new_password2)
                        .password(true)
                        .hint_text("确认新密码")
                        .desired_width(136.0)
                        .frame(false)
                        .margin(egui::vec2(0.0, 0.0)),
                );
                
                // 按钮
                // Change: Title[209] at (160, 258)
                if self.draw_image_button(ui, ctx, 209, 209, 209,
                    egui::pos2(rect.min.x + 160.0, rect.min.y + 258.0)) {
                    // TODO: 验证输入并修改密码
                    *open = false;
                }
                
                // Cancel: Title[194/195/196] at (240, 258)
                if self.draw_image_button(ui, ctx, 194, 195, 196,
                    egui::pos2(rect.min.x + 240.0, rect.min.y + 258.0)) {
                    *open = false;
                }
            });
    }
}
