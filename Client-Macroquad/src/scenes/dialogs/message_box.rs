// 公共消息框组件
use macroquad::prelude::*;
use egui_macroquad::egui;
use crate::resources::LibraryName;

/// 消息框按钮类型
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MessageBoxButtons {
    Ok,
    OkCancel,
    YesNo,
}

/// 消息框结果
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MessageBoxResult {
    None,
    Ok,
    Cancel,
    Yes,
    No,
}

/// 消息框组件
pub struct MessageBox {
    pub title: String,
    pub text: String,
    pub buttons: MessageBoxButtons,
    pub result: MessageBoxResult,
    pub visible: bool,
    id: String,  // 唯一标识符，用于 egui Area ID
}

impl MessageBox {
    /// 创建新消息框(带自定义 ID)
    /// 
    /// ID 在创建时固定,后续修改 title/text 不会影响 egui widget ID
    pub fn new_with_id(title: &str, text: &str, buttons: MessageBoxButtons, id: &str) -> Self {
        Self {
            title: title.to_string(),
            text: text.to_string(),
            buttons,
            result: MessageBoxResult::None,
            visible: false,
            id: format!("message_box_{}", id),
        }
    }
    
    /// 创建新消息框(使用时间戳生成唯一 ID)
    /// 
    /// ⚠️ 每次调用都会生成新的 ID!
    /// 如果需要复用同一个消息框实例并修改内容,请使用 new_with_id()
    pub fn new(title: &str, text: &str, buttons: MessageBoxButtons) -> Self {
        use std::time::{SystemTime, UNIX_EPOCH};
        let timestamp = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_micros();
        let id = format!("{:x}", timestamp);
        Self::new_with_id(title, text, buttons, &id)
    }

    /// 显示消息框
    pub fn show(&mut self) {
        self.visible = true;
        self.result = MessageBoxResult::None;
    }

    /// 隐藏消息框
    pub fn hide(&mut self) {
        self.visible = false;
    }

    /// 绘制消息框（完全不需要外部参数，内部使用全局资源）
    pub fn draw(&mut self, ctx: &egui::Context) {
        if !self.visible {
            return;
        }

        // 获取 Prguse[360] 实际尺寸（使用全局库）
        let (dialog_w, dialog_h) = {
            // ✅ 新 API
            if let Some(info) = LibraryName::Prguse.get_egui_texture(ctx, 360) {
                if let Some(ref handle) = info.egui_texture {
                    (handle.size_vec2().x, handle.size_vec2().y)
                } else {
                    (460.0, 200.0)
                }
            } else {
                (460.0, 200.0)
            }
        };

        // ESC键关闭消息框
        if ctx.input(|i| i.key_pressed(egui::Key::Escape)) {
            self.visible = false;
            self.result = MessageBoxResult::Cancel;
            return;
        }

        egui::Area::new(egui::Id::new(&self.id))
            .default_pos(egui::pos2(
                (screen_width() / screen_dpi_scale() - dialog_w) / 2.0,
                (screen_height() / screen_dpi_scale() - dialog_h) / 2.0
            ))  // 初始居中位置，但可以拖动
            .interactable(true)
            .movable(true)  // 允许拖动消息框
            .order(egui::Order::Foreground)  // 确保消息框在最上层
            .show(ctx, |ui| {
                // 分配对话框空间 (仅用于定位,不拦截点击事件)
                let rect = ui.allocate_rect(
                    egui::Rect::from_min_size(ui.cursor().min, egui::vec2(dialog_w, dialog_h)),
                    egui::Sense::hover()
                ).rect;

                // 绘制背景纹理 Prguse[360]（使用全局纹理缓存）
                // ✅ 新 API
                if let Some(info) = LibraryName::Prguse.get_egui_texture(ctx, 360) {
                    if let Some(ref handle) = info.egui_texture {
                        let uv = egui::Rect::from_min_max(egui::pos2(0.0, 0.0), egui::pos2(1.0, 1.0));
                        let tint = egui::Color32::WHITE;
                        ui.painter().image(
                            handle.id(),
                            rect,
                            uv,
                            tint,
                        );
                    }
                }

                // 绘制标题和文本
                let title_pos = egui::pos2(rect.min.x + 20.0, rect.min.y + 10.0);
                ui.painter().text(
                    title_pos,
                    egui::Align2::LEFT_TOP,
                    &self.title,
                    egui::FontId::proportional(18.0),
                    egui::Color32::WHITE,
                );

                let text_pos = egui::pos2(rect.min.x + 20.0, rect.min.y + 50.0);
                ui.painter().text(
                    text_pos,
                    egui::Align2::LEFT_TOP,
                    &self.text,
                    egui::FontId::proportional(14.0),
                    egui::Color32::WHITE,
                );

                // 绘制按钮（根据类型）
                match self.buttons {
                    MessageBoxButtons::Ok => {
                        // OK 按钮 (Title 200/201/202)
                        if self.draw_image_button_internal(ui, ctx, 200, 201, 202, egui::pos2(rect.min.x + 360.0, rect.min.y + 157.0)) {
                            self.result = MessageBoxResult::Ok;
                            self.visible = false;
                        }
                    }
                    MessageBoxButtons::OkCancel => {
                        // OK 按钮
                        if self.draw_image_button_internal(ui, ctx, 200, 201, 202, egui::pos2(rect.min.x + 260.0, rect.min.y + 157.0)) {
                            self.result = MessageBoxResult::Ok;
                            self.visible = false;
                        }
                        // Cancel 按钮 (Title 203/204/205)
                        if self.draw_image_button_internal(ui, ctx, 203, 204, 205, egui::pos2(rect.min.x + 360.0, rect.min.y + 157.0)) {
                            self.result = MessageBoxResult::Cancel;
                            self.visible = false;
                        }
                    }
                    MessageBoxButtons::YesNo => {
                        // Yes 按钮 (Title 206/207/208)
                        if self.draw_image_button_internal(ui, ctx, 206, 207, 208, egui::pos2(rect.min.x + 260.0, rect.min.y + 157.0)) {
                            self.result = MessageBoxResult::Yes;
                            self.visible = false;
                        }
                        // No 按钮 (Title 210/211/212)
                        if self.draw_image_button_internal(ui, ctx, 210, 211, 212, egui::pos2(rect.min.x + 360.0, rect.min.y + 157.0)) {
                            self.result = MessageBoxResult::No;
                            self.visible = false;
                        }
                    }
                }
            });
    }
    
    /// 绘制图像按钮(实例方法,使用 MessageBox 的 ID 避免冲突)
    fn draw_image_button_internal(
        &self,
        ui: &mut egui::Ui,
        ctx: &egui::Context,
        normal_idx: usize,
        hover_idx: usize,
        pressed_idx: usize,
        abs_pos: egui::Pos2,
    ) -> bool {
        // 获取按钮正常状态纹理（使用全局纹理缓存）
        // ✅ 新 API
        if let Some(info) = LibraryName::Title.get_egui_texture(ctx, normal_idx) {
            if let Some(ref handle) = info.egui_texture {
                // 获取纹理尺寸
                let texture_size = handle.size_vec2();
                let button_rect = egui::Rect::from_min_size(abs_pos, texture_size);

                // 检测鼠标交互 - 使用 MessageBox ID + 按钮索引确保唯一性
                let button_id = format!("{}_{}", self.id, normal_idx);
                let response = ui.interact(button_rect, egui::Id::new(button_id), egui::Sense::click());

                // 根据状态选择纹理索引
                let texture_idx = if response.is_pointer_button_down_on() {
                    pressed_idx  // 按下状态
                } else if response.hovered() {
                    hover_idx  // 悬停状态
                } else {
                    normal_idx  // 正常状态
                };

                // 绘制按钮纹理（使用全局纹理缓存）
                // ✅ 新 API
                if let Some(btn_info) = LibraryName::Title.get_egui_texture(ctx, texture_idx) {
                    if let Some(ref btn_handle) = btn_info.egui_texture {
                        let uv = egui::Rect::from_min_max(egui::pos2(0.0, 0.0), egui::pos2(1.0, 1.0));
                        let tint = egui::Color32::WHITE;
                        ui.painter().image(
                            btn_handle.id(),
                            button_rect,
                            uv,
                            tint,
                        );
                    }
                }

                return response.clicked();
            }
        }
        false
    }
}
