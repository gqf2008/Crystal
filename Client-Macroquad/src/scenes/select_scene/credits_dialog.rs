use macroquad::prelude::*;

use crate::resources::LibraryName;
use crate::ui::text_renderer::{draw_text_cn, measure_text_cn};
use crate::ui::widgets::{begin_modal, draw_dialog_panel};

#[derive(Debug, Clone)]
pub struct CreditLine {
    pub text: String,
    pub font_size: f32,
    pub color: Color,
    pub is_title: bool,
}

#[derive(Debug, Clone)]
pub struct CreditsDialog {
    visible: bool,
    content: Vec<CreditLine>,
}

impl CreditsDialog {
    pub fn new() -> Self {
        // 与 ClientRust `CreditsDialog::new()` 内容保持一致
        let content = vec![
            CreditLine {
                text: "Legend of Mir 2".to_string(),
                font_size: 20.0,
                color: Color::from_rgba(255, 215, 0, 255),
                is_title: true,
            },
            CreditLine {
                text: "Rust Client".to_string(),
                font_size: 14.0,
                color: Color::from_rgba(180, 180, 180, 255),
                is_title: true,
            },
            CreditLine {
                text: "".to_string(),
                font_size: 8.0,
                color: WHITE,
                is_title: false,
            },
            CreditLine {
                text: "Version 0.1.0-alpha".to_string(),
                font_size: 13.0,
                color: WHITE,
                is_title: false,
            },
            CreditLine {
                text: "Build: 2025-10-08".to_string(),
                font_size: 13.0,
                color: WHITE,
                is_title: false,
            },
            CreditLine {
                text: "".to_string(),
                font_size: 8.0,
                color: WHITE,
                is_title: false,
            },
            CreditLine {
                text: "Technology".to_string(),
                font_size: 14.0,
                color: Color::from_rgba(100, 200, 255, 255),
                is_title: true,
            },
            CreditLine {
                text: "Rust + ggez + Tokio".to_string(),
                font_size: 12.0,
                color: WHITE,
                is_title: false,
            },
            CreditLine {
                text: "".to_string(),
                font_size: 8.0,
                color: WHITE,
                is_title: false,
            },
            CreditLine {
                text: "Development".to_string(),
                font_size: 14.0,
                color: Color::from_rgba(100, 200, 255, 255),
                is_title: true,
            },
            CreditLine {
                text: "Original: Crystal Team".to_string(),
                font_size: 12.0,
                color: WHITE,
                is_title: false,
            },
            CreditLine {
                text: "Rust Port: Community".to_string(),
                font_size: 12.0,
                color: WHITE,
                is_title: false,
            },
            CreditLine {
                text: "".to_string(),
                font_size: 10.0,
                color: WHITE,
                is_title: false,
            },
            CreditLine {
                text: "Press ESC or Click to Close".to_string(),
                font_size: 11.0,
                color: Color::from_rgba(150, 150, 150, 255),
                is_title: true,
            },
        ];

        Self {
            visible: false,
            content,
        }
    }

    pub fn is_visible(&self) -> bool {
        self.visible
    }

    pub fn _open(&mut self) {
        self.visible = true;
    }

    pub fn close(&mut self) {
        self.visible = false;
    }

    pub fn update(&mut self, dt: f32) {
        let _ = dt;
    }

    /// 处理输入；返回 true 表示已消费（应阻止底层 UI）。
    pub fn handle_input(&mut self) -> bool {
        if !self.visible {
            return false;
        }

        if is_key_pressed(KeyCode::Escape) {
            self.close();
            return true;
        }

        if is_mouse_button_pressed(MouseButton::Left) {
            self.close();
            return true;
        }

        true
    }

    pub fn draw(&mut self) {
        if !self.visible {
            return;
        }

        // 使用与 ClientRust credits_dialog 类似的尺寸/纹理。
        let dialog_w = 464.0;
        let dialog_h = 260.0;
        let dialog = begin_modal(dialog_w, dialog_h, 200);

        // 背景：优先使用 Prguse[360] 纹理；缺失时回退到通用对话框面板。
        if let Some(info) = LibraryName::Prguse.get_texture(360) {
            if let Some(ref texture) = info.image {
                draw_texture(
                    texture,
                    // ClientRust CreditsDialog 使用 draw_sprite_at（不应用 offset）
                    dialog.x, dialog.y, WHITE,
                );
            } else {
                draw_dialog_panel(
                    dialog.x,
                    dialog.y,
                    dialog.w,
                    dialog.h,
                    Color::from_rgba(40, 40, 50, 240),
                    Color::from_rgba(100, 100, 120, 255),
                );
            }
        } else {
            draw_dialog_panel(
                dialog.x,
                dialog.y,
                dialog.w,
                dialog.h,
                Color::from_rgba(40, 40, 50, 240),
                Color::from_rgba(100, 100, 120, 255),
            );
        }

        // 内容区域：与 ClientRust 一致，从顶部偏移 30
        let top = dialog.y + 30.0;
        let center_x = dialog.x + dialog.w / 2.0;
        let left_x = dialog.x + 60.0;

        let mut y = top;
        for line in &self.content {
            if line.text.is_empty() {
                y += line.font_size * 0.4;
                continue;
            }

            if line.is_title {
                let w = measure_text_cn(&line.text, line.font_size).width;
                draw_text_cn(
                    &line.text,
                    center_x - w / 2.0,
                    y,
                    line.font_size,
                    line.color,
                );
            } else {
                draw_text_cn(&line.text, left_x, y, line.font_size, line.color);
            }

            let line_spacing = if line.is_title { 10.0 } else { 5.0 };
            y += line.font_size + line_spacing;
        }
    }
}
