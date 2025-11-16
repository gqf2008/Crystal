// ============================================================================
// 制作名单对话框组件
// ============================================================================
// 
// 【功能说明】
// 显示游戏制作团队名单，支持：
// 1. 滚动文本显示
// 2. 关闭按钮
// 3. ESC 键关闭
//
// ============================================================================

use super::Dialog;
use egui_macroquad::egui;
use crate::resources::LibraryName;

/// 制作名单对话框
pub struct CreditsDialog {
    /// 滚动偏移量
    scroll_offset: f32,
    /// 滚动速度（像素/秒）
    scroll_speed: f32,
    /// 是否自动滚动
    auto_scroll: bool,
}

impl CreditsDialog {
    pub fn new() -> Self {
        Self {
            scroll_offset: 0.0,
            scroll_speed: 30.0,
            auto_scroll: true,
        }
    }
    
    /// 更新滚动
    pub fn update(&mut self, dt: f32) {
        if self.auto_scroll {
            self.scroll_offset += self.scroll_speed * dt;
        }
    }
    
    /// 重置滚动
    pub fn reset(&mut self) {
        self.scroll_offset = 0.0;
        self.auto_scroll = true;
    }
    
    /// 获取制作名单文本
    fn get_credits_text() -> &'static str {
        r#"
《传奇2 - Legend of Mir 2》

━━━━━━━━━━━━━━━━━━━━━━
        游戏制作团队
━━━━━━━━━━━━━━━━━━━━━━

原版开发
    Wemade Entertainment

Rust 重制版
    Crystal Project Team

━━━━━━━━━━━━━━━━━━━━━━
        技术栈
━━━━━━━━━━━━━━━━━━━━━━

游戏引擎
    Macroquad

UI 框架
    egui

编程语言
    Rust

━━━━━━━━━━━━━━━━━━━━━━
        特别感谢
━━━━━━━━━━━━━━━━━━━━━━

感谢所有玩家的支持！

感谢开源社区的贡献！

━━━━━━━━━━━━━━━━━━━━━━

© 2024 Crystal Project
All Rights Reserved

━━━━━━━━━━━━━━━━━━━━━━
"#
    }
    
    /// 绘制关闭按钮
    fn draw_close_button(
        &self,
        ui: &mut egui::Ui,
        ctx: &egui::Context,
        pos: egui::Pos2,
    ) -> bool {
        // 使用 Title[280-282] (Cancel 按钮)
        if let Some(info) = LibraryName::Title.get_egui_texture(ctx, 280) {
            if let Some(texture) = info.egui_texture {
                let size = egui::vec2(texture.size()[0] as f32, texture.size()[1] as f32);
                let rect = egui::Rect::from_min_size(pos, size);
                let response = ui.interact(rect, egui::Id::new("credits_close_btn"), egui::Sense::click());
                
                let texture_idx = if response.is_pointer_button_down_on() {
                    282  // Pressed
                } else if response.hovered() {
                    281  // Hover
                } else {
                    280  // Normal
                };
                
                if let Some(btn_info) = LibraryName::Title.get_egui_texture(ctx, texture_idx) {
                    if let Some(btn_texture) = btn_info.egui_texture {
                        ui.painter().image(
                            btn_texture.id(),
                            rect,
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

impl Dialog for CreditsDialog {
    fn show(&mut self, ctx: &egui::Context, open: &mut bool) {
        if !*open {
            return;
        }
        
        // ESC 键关闭
        if ctx.input(|i| i.key_pressed(egui::Key::Escape)) {
            *open = false;
            self.reset();
            return;
        }
        
        // 获取背景纹理 Prguse[360] 或使用固定尺寸
        let (dialog_w, _) = LibraryName::Prguse.get_size(360).unwrap_or((460, 200));
        let dialog_w = dialog_w as f32;
        let dialog_h = 500.0_f32;  // 使用更高的对话框
        
        egui::Area::new(egui::Id::new("credits_dialog"))
            .default_pos(egui::pos2(
                (macroquad::prelude::screen_width() / macroquad::prelude::screen_dpi_scale() - dialog_w) / 2.0,
                (macroquad::prelude::screen_height() / macroquad::prelude::screen_dpi_scale() - dialog_h) / 2.0
            ))
            .movable(true)
            .interactable(true)
            .order(egui::Order::Foreground)
            .show(ctx, |ui| {
                // 分配对话框空间
                let rect = ui.allocate_rect(
                    egui::Rect::from_min_size(ui.cursor().min, egui::vec2(dialog_w, dialog_h)),
                    egui::Sense::hover()
                ).rect;
                
                // 绘制背景纹理 Prguse[360] - 直接拉伸填充整个对话框
                if let Some(info) = LibraryName::Prguse.get_egui_texture(ctx, 360) {
                    if let Some(bg_texture) = info.egui_texture {
                        ui.painter().image(
                            bg_texture.id(),
                            rect,  // 直接使用对话框的矩形区域
                            egui::Rect::from_min_max(egui::pos2(0.0, 0.0), egui::pos2(1.0, 1.0)),
                            egui::Color32::WHITE,
                        );
                    }
                } else {
                    // 如果纹理加载失败，使用黑色半透明背景作为后备
                    ui.painter().rect_filled(
                        rect,
                        5.0,
                        egui::Color32::from_rgba_premultiplied(0, 0, 0, 230)
                    );
                }
                
                // 绘制边框
                ui.painter().rect_stroke(
                    rect,
                    5.0,
                    egui::Stroke::new(2.0, egui::Color32::from_rgb(100, 100, 100)),
                    egui::epaint::StrokeKind::Middle,
                );
                
                // 标题
                ui.painter().text(
                    egui::pos2(rect.center().x, rect.min.y + 20.0),
                    egui::Align2::CENTER_TOP,
                    "制作名单",
                    egui::FontId::proportional(24.0),
                    egui::Color32::WHITE,
                );
                
                // 滚动文本区域
                let text_area = egui::Rect::from_min_size(
                    egui::pos2(rect.min.x + 20.0, rect.min.y + 60.0),
                    egui::vec2(dialog_w - 40.0, dialog_h - 140.0)
                );
                
                // 设置裁剪区域
                ui.set_clip_rect(text_area);
                
                // 绘制滚动文本
                let credits_text = Self::get_credits_text();
                let lines: Vec<&str> = credits_text.lines().collect();
                let line_height = 25.0;
                
                for (i, line) in lines.iter().enumerate() {
                    let y = text_area.min.y + (i as f32 * line_height) - self.scroll_offset;
                    
                    // 只绘制可见区域的文本
                    if y > text_area.min.y - line_height && y < text_area.max.y + line_height {
                        let color = if line.starts_with("━") || line.trim().is_empty() {
                            egui::Color32::from_rgb(150, 150, 150)
                        } else if line.trim().starts_with("《") || line.contains("Team") {
                            egui::Color32::from_rgb(255, 215, 0)  // 金色
                        } else {
                            egui::Color32::WHITE
                        };
                        
                        ui.painter().text(
                            egui::pos2(text_area.center().x, y),
                            egui::Align2::CENTER_TOP,
                            line,
                            egui::FontId::proportional(16.0),
                            color,
                        );
                    }
                }
                
                // 重置裁剪区域
                ui.set_clip_rect(rect);
                
                // 关闭按钮
                let close_btn_pos = egui::pos2(
                    rect.center().x - 40.0,
                    rect.max.y - 60.0
                );
                
                if self.draw_close_button(ui, ctx, close_btn_pos) {
                    *open = false;
                    self.reset();
                }
                
                // 提示文字
                ui.painter().text(
                    egui::pos2(rect.center().x, rect.max.y - 25.0),
                    egui::Align2::CENTER_CENTER,
                    "ESC 关闭",
                    egui::FontId::proportional(12.0),
                    egui::Color32::from_rgb(150, 150, 150),
                );
            });
    }
}
