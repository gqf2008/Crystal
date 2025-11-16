// 新建账号对话框组件
use macroquad::prelude::*;
use egui_macroquad::egui;
use crate::resources::LibraryName;
use super::Dialog;

/// 新建账号对话框
pub struct NewAccountDialog {
    // 输入字段
    pub account_id: String,
    pub password1: String,
    pub password2: String,
    pub email: String,
    pub username: String,
    pub birthdate: String,
    pub question: String,
    pub answer: String,
    
    // 对话框 ID
    id: String,
}

/// 新建账号结果
#[derive(Debug, Clone, PartialEq)]
pub enum NewAccountResult {
    None,
    Create,
    Cancel,
}

impl NewAccountDialog {
    /// 创建新建账号对话框
    pub fn new() -> Self {
        Self {
            account_id: String::new(),
            password1: String::new(),
            password2: String::new(),
            email: String::new(),
            username: String::new(),
            birthdate: String::new(),
            question: String::new(),
            answer: String::new(),
            id: "new_account_dialog".to_string(),
        }
    }
    
    /// 重置所有字段
    pub fn reset(&mut self) {
        self.account_id.clear();
        self.password1.clear();
        self.password2.clear();
        self.email.clear();
        self.username.clear();
        self.birthdate.clear();
        self.question.clear();
        self.answer.clear();
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

impl Dialog for NewAccountDialog {
    fn show(&mut self, ctx: &egui::Context, open: &mut bool) {
        if !*open {
            return;
        }
        
        let dialog_w = 588.0; // Prguse[63] 实际尺寸
        let dialog_h = 460.0;
        
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
                
                // 绘制背景
                if let Some(info) = LibraryName::Prguse.get_egui_texture(ctx, 63) {
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
                ui.put(
                    egui::Rect::from_min_size(
                        egui::pos2(rect.min.x + 226.0, rect.min.y + 103.0),
                        egui::vec2(136.0, 15.0),
                    ),
                    egui::TextEdit::singleline(&mut self.account_id)
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
                    egui::TextEdit::singleline(&mut self.password1)
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
                    egui::TextEdit::singleline(&mut self.password2)
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
                    egui::TextEdit::singleline(&mut self.username)
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
                    egui::TextEdit::singleline(&mut self.birthdate)
                        .hint_text("生日")
                        .desired_width(136.0)
                        .frame(false)
                        .margin(egui::vec2(0.0, 0.0)),
                );
                
                ui.put(
                    egui::Rect::from_min_size(
                        egui::pos2(rect.min.x + 226.0, rect.min.y + 250.0),
                        egui::vec2(136.0, 15.0),
                    ),
                    egui::TextEdit::singleline(&mut self.question)
                        .hint_text("密保问题")
                        .desired_width(136.0)
                        .frame(false)
                        .margin(egui::vec2(0.0, 0.0)),
                );
                
                ui.put(
                    egui::Rect::from_min_size(
                        egui::pos2(rect.min.x + 226.0, rect.min.y + 276.0),
                        egui::vec2(136.0, 15.0),
                    ),
                    egui::TextEdit::singleline(&mut self.answer)
                        .hint_text("答案")
                        .desired_width(136.0)
                        .frame(false)
                        .margin(egui::vec2(0.0, 0.0)),
                );
                
                ui.put(
                    egui::Rect::from_min_size(
                        egui::pos2(rect.min.x + 226.0, rect.min.y + 311.0),
                        egui::vec2(136.0, 15.0),
                    ),
                    egui::TextEdit::singleline(&mut self.email)
                        .hint_text("邮箱")
                        .desired_width(136.0)
                        .frame(false)
                        .margin(egui::vec2(0.0, 0.0)),
                );
                
                // 按钮 (C# 原版: OK=135,425  Cancel=409,425)
                // OK按钮: Title[200/201/202]
                if self.draw_image_button(ui, ctx, 200, 201, 202, 
                    egui::pos2(rect.min.x + 135.0, rect.min.y + 425.0)) {
                    // TODO: 验证输入并创建账号
                    *open = false;
                }
                
                // Cancel按钮: Title[203/204/205]
                if self.draw_image_button(ui, ctx, 203, 204, 205,
                    egui::pos2(rect.min.x + 409.0, rect.min.y + 425.0)) {
                    *open = false;
                }
            });
    }
}
