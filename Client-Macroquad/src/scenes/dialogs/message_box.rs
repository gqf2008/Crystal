// 公共消息框组件
use super::Dialog;
use crate::resources::libraries::LibraryName;
use crate::resources::get_or_create_egui_texture;
use egui_macroquad::egui;
use macroquad::prelude::*;

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
    /// 唯一ID，用于区分不同的MessageBox实例
    id: u64,
}

// 静态计数器用于生成唯一ID
static MESSAGE_BOX_ID_COUNTER: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(0);

impl MessageBox {
    /// 创建新消息框
    pub fn new(title: &str, text: &str, buttons: MessageBoxButtons) -> Self {
        let id = MESSAGE_BOX_ID_COUNTER.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
        Self {
            title: title.to_string(),
            text: text.to_string(),
            buttons,
            result: MessageBoxResult::None,
            id,
        }
    }

    /// 重置结果
    pub fn reset_result(&mut self) {
        self.result = MessageBoxResult::None;
    }

    /// 绘制图像按钮（Title 库）- 使用全局纹理缓存
    fn draw_image_button(
        ui: &mut egui::Ui,
        ctx: &egui::Context,
        normal_idx: usize,
        hover_idx: usize,
        pressed_idx: usize,
        abs_pos: egui::Pos2,
    ) -> bool {
        // 获取按钮正常状态纹理（使用全局纹理缓存）

        if let Some(mut img) = LibraryName::Title.get_image(normal_idx) {
            if let Some(handle) = img.as_egui_texture(ctx) {
                // 获取纹理尺寸
                let texture_size = handle.size_vec2();
                let button_rect = egui::Rect::from_min_size(abs_pos, texture_size);

                // 检测鼠标交互
                let response = ui.interact(
                    button_rect,
                    egui::Id::new(format!("btn_{}", normal_idx)),
                    egui::Sense::click(),
                );

                // 根据状态选择纹理索引
                let texture_idx = if response.is_pointer_button_down_on() {
                    pressed_idx // 按下状态
                } else if response.hovered() {
                    hover_idx // 悬停状态
                } else {
                    normal_idx // 正常状态
                };

                // 绘制按钮纹理（使用全局纹理缓存）
                if let Some(mut btn_image) = LibraryName::Title.get_image(texture_idx) {
                    if let Some(btn_handle) = btn_image.as_egui_texture(ctx) {
                        let uv =
                            egui::Rect::from_min_max(egui::pos2(0.0, 0.0), egui::pos2(1.0, 1.0));
                        let tint = egui::Color32::WHITE;
                        ui.painter().image(btn_handle.id(), button_rect, uv, tint);
                    }
                }
                return response.clicked();
            }
        }

        false
    }
}

// ============================================================================
// Dialog trait 实现
// ============================================================================

impl Dialog for MessageBox {
    fn show(&mut self, ctx: &egui::Context, open: &mut bool) {
        if !*open {
            return;
        }

        // 获取 Prguse[360] 实际尺寸（使用全局库）
        let (dialog_w, dialog_h) = {
            if let Some(handle) = get_or_create_egui_texture(ctx, LibraryName::Prguse, 360) {
                (handle.size_vec2().x, handle.size_vec2().y)
            } else {
                (460.0, 200.0)
            }
        };

        // ESC键关闭消息框
        if ctx.input(|i| i.key_pressed(egui::Key::Escape)) {
            *open = false;
            self.result = MessageBoxResult::Cancel;
            return;
        }

        egui::Area::new(egui::Id::new(format!("message_box_{}", self.id)))
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

                // 绘制背景纹理 Prguse[360]（使用全局纹理缓存）
                if let Some(handle) = get_or_create_egui_texture(ctx, LibraryName::Prguse, 360) {
                    let uv = egui::Rect::from_min_max(egui::pos2(0.0, 0.0), egui::pos2(1.0, 1.0));
                    let tint = egui::Color32::WHITE;
                    ui.painter().image(handle.id(), rect, uv, tint);
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
                        if Self::draw_image_button(
                            ui,
                            ctx,
                            200,
                            201,
                            202,
                            egui::pos2(rect.min.x + 360.0, rect.min.y + 157.0),
                        ) {
                            self.result = MessageBoxResult::Ok;
                            *open = false;
                        }
                    }
                    MessageBoxButtons::OkCancel => {
                        // OK 按钮
                        if Self::draw_image_button(
                            ui,
                            ctx,
                            200,
                            201,
                            202,
                            egui::pos2(rect.min.x + 260.0, rect.min.y + 157.0),
                        ) {
                            self.result = MessageBoxResult::Ok;
                            *open = false;
                        }
                        // Cancel 按钮 (Title 203/204/205)
                        if Self::draw_image_button(
                            ui,
                            ctx,
                            203,
                            204,
                            205,
                            egui::pos2(rect.min.x + 360.0, rect.min.y + 157.0),
                        ) {
                            self.result = MessageBoxResult::Cancel;
                            *open = false;
                        }
                    }
                    MessageBoxButtons::YesNo => {
                        // Yes 按钮 (Title 206/207/208)
                        if Self::draw_image_button(
                            ui,
                            ctx,
                            206,
                            207,
                            208,
                            egui::pos2(rect.min.x + 260.0, rect.min.y + 157.0),
                        ) {
                            self.result = MessageBoxResult::Yes;
                            *open = false;
                        }
                        // No 按钮 (Title 210/211/212)
                        if Self::draw_image_button(
                            ui,
                            ctx,
                            210,
                            211,
                            212,
                            egui::pos2(rect.min.x + 360.0, rect.min.y + 157.0),
                        ) {
                            self.result = MessageBoxResult::No;
                            *open = false;
                        }
                    }
                }
            });
    }
}
