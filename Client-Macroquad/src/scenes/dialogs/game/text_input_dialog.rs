// ============================================================================
// TextInputDialogHybrid - 通用文本输入对话框
// ============================================================================
//
// 【用途】
// - 组队邀请：输入玩家名称
// - 添加好友：输入玩家名称
// - 拜师：输入师傅名称
//
// 【资源】
// - 背景：Prguse[660]
// - OK 按钮：Title[200-202]
// - Cancel 按钮：Title[203-205]
//
// 【输入支持】
// - Unicode/中文输入（get_char_pressed）
// - Ctrl+V 粘贴
// - Backspace 连续删除
// - Enter 确认，Escape 取消
//
// ============================================================================

use macroquad::prelude::*;

use super::native_ui_utils::{ButtonState, ButtonTextures};
use crate::resources::LibraryName;
use crate::ui::text_renderer::draw_text_cn;
use crate::utils::ime::set_ime_enabled;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum TextInputResult {
    None,
    Ok(String),
    Cancel,
}

pub struct TextInputDialogHybrid {
    visible: bool,
    input_active: bool,
    input_text: String,
    max_length: usize,

    // 配置
    title: String,
    placeholder: String,

    // 布局
    position: Vec2,
    size: Vec2,

    // 纹理
    bg_texture: Option<Texture2D>,
    ok_btn: ButtonTextures,
    cancel_btn: ButtonTextures,

    // Backspace 连续删除
    backspace_timer: f64,
    backspace_repeat: bool,
}

impl Default for TextInputDialogHybrid {
    fn default() -> Self {
        Self::new()
    }
}

impl TextInputDialogHybrid {
    const OK_X: f32 = 30.0;
    const OK_Y: f32 = 85.0;
    const CANCEL_X: f32 = 120.0;
    const CANCEL_Y: f32 = 85.0;
    const INPUT_X: f32 = 12.0;
    const INPUT_Y: f32 = 38.0;
    const INPUT_W: f32 = 170.0;
    const INPUT_H: f32 = 22.0;

    pub fn new() -> Self {
        Self {
            visible: false,
            input_active: true,
            input_text: String::new(),
            max_length: 32,
            title: String::new(),
            placeholder: String::new(),
            position: Vec2::ZERO,
            size: vec2(200.0, 115.0),
            bg_texture: None,
            ok_btn: ButtonTextures::new(),
            cancel_btn: ButtonTextures::new(),
            backspace_timer: 0.0,
            backspace_repeat: false,
        }
    }

    /// 显示输入对话框
    pub fn show(&mut self, title: &str, placeholder: &str, max_length: usize) {
        self.title = title.to_string();
        self.placeholder = placeholder.to_string();
        self.max_length = max_length;
        self.input_text.clear();
        self.input_active = true;
        self.visible = true;
        self.load_textures();

        // 居中显示
        let screen_w = screen_width();
        let screen_h = screen_height();
        self.position = vec2(
            (screen_w - self.size.x) / 2.0,
            (screen_h - self.size.y) / 2.0,
        );

        set_ime_enabled(true);
    }

    pub fn hide(&mut self) {
        self.visible = false;
        self.input_active = false;
        set_ime_enabled(false);
    }

    pub fn is_visible(&self) -> bool {
        self.visible
    }

    fn load_textures(&mut self) {
        // 背景 Prguse[660]
        if let Some(tex) = LibraryName::Prguse.get_texture(660) {
            self.bg_texture = tex.image.clone();
            self.size = vec2(tex.width as f32, tex.height as f32);
        }

        // OK 按钮 Title[200-202]
        self.ok_btn = ButtonTextures::load_from_indices(LibraryName::Title, [200, 201, 202]);
        // Cancel 按钮 Title[203-205]
        self.cancel_btn = ButtonTextures::load_from_indices(LibraryName::Title, [203, 204, 205]);
    }

    pub fn update_and_draw(&mut self) -> TextInputResult {
        if !self.visible {
            return TextInputResult::None;
        }

        let kb_result = self.handle_keyboard();
        if !matches!(kb_result, TextInputResult::None) {
            return kb_result;
        }

        let (mx, my) = mouse_position();
        let mouse_pos = vec2(mx, my);
        let px = self.position.x;
        let py = self.position.y;

        // 背景
        if let Some(ref bg) = self.bg_texture {
            draw_texture(bg, px, py, WHITE);
        } else {
            draw_rectangle(
                px,
                py,
                self.size.x,
                self.size.y,
                Color::new(0.05, 0.05, 0.1, 0.9),
            );
        }

        // 标题
        draw_text_cn(&self.title, px + 12.0, py + 18.0, 14.0, WHITE);

        // 输入框背景
        let input_rect = Rect::new(
            px + Self::INPUT_X,
            py + Self::INPUT_Y,
            Self::INPUT_W,
            Self::INPUT_H,
        );
        draw_rectangle(
            input_rect.x,
            input_rect.y,
            input_rect.w,
            input_rect.h,
            Color::from_rgba(30, 30, 40, 255),
        );
        draw_rectangle_lines(
            input_rect.x,
            input_rect.y,
            input_rect.w,
            input_rect.h,
            1.0,
            Color::from_rgba(80, 80, 100, 255),
        );

        // 输入文本
        let display_text = if self.input_text.is_empty() {
            &self.placeholder
        } else {
            &self.input_text
        };
        let text_color = if self.input_text.is_empty() {
            Color::from_rgba(120, 120, 120, 255)
        } else {
            WHITE
        };
        draw_text_cn(
            display_text,
            input_rect.x + 4.0,
            input_rect.y + 16.0,
            13.0,
            text_color,
        );

        // OK 按钮
        let ok_rect = Rect::new(px + Self::OK_X, py + Self::OK_Y, 32.0, 22.0);
        let ok_state = ButtonState::from_mouse(ok_rect, mouse_pos);
        self.ok_btn.draw(vec2(ok_rect.x, ok_rect.y), ok_state);

        // Cancel 按钮
        let cancel_rect = Rect::new(px + Self::CANCEL_X, py + Self::CANCEL_Y, 32.0, 22.0);
        let cancel_state = ButtonState::from_mouse(cancel_rect, mouse_pos);
        self.cancel_btn
            .draw(vec2(cancel_rect.x, cancel_rect.y), cancel_state);

        // 按钮点击
        if ButtonState::is_clicked(ok_rect, mouse_pos) {
            return self.submit_ok();
        }
        if ButtonState::is_clicked(cancel_rect, mouse_pos) {
            return TextInputResult::Cancel;
        }

        TextInputResult::None
    }

    fn handle_keyboard(&mut self) -> TextInputResult {
        if !self.input_active {
            return TextInputResult::None;
        }

        // Unicode/中文输入
        let mut pending_chars: Vec<char> = Vec::new();
        while let Some(ch) = get_char_pressed() {
            if !ch.is_control() {
                pending_chars.push(ch);
            }
        }
        for ch in pending_chars {
            if self.input_text.chars().count() < self.max_length {
                self.input_text.push(ch);
            }
        }

        // Ctrl+V 粘贴
        if (is_key_down(KeyCode::LeftControl) || is_key_down(KeyCode::RightControl))
            && is_key_pressed(KeyCode::V)
        {
            if let Some(clipboard) = miniquad::window::clipboard_get() {
                for ch in clipboard.chars() {
                    if !ch.is_control() && self.input_text.chars().count() < self.max_length {
                        self.input_text.push(ch);
                    }
                }
            }
        }

        // Enter 确认
        if is_key_pressed(KeyCode::Enter) && !self.input_text.is_empty() {
            return self.submit_ok();
        }

        // Escape 取消
        if is_key_pressed(KeyCode::Escape) {
            self.hide();
            return TextInputResult::Cancel;
        }

        // Backspace 连续删除
        if is_key_down(KeyCode::Backspace) {
            let now = get_time();
            if is_key_pressed(KeyCode::Backspace) {
                if !self.input_text.is_empty() {
                    self.input_text.pop();
                }
                self.backspace_timer = now;
                self.backspace_repeat = false;
            } else {
                let delay = if self.backspace_repeat { 0.03 } else { 0.4 };
                if now - self.backspace_timer > delay {
                    if !self.input_text.is_empty() {
                        self.input_text.pop();
                    }
                    self.backspace_timer = now;
                    self.backspace_repeat = true;
                }
            }
        } else {
            self.backspace_repeat = false;
        }

        TextInputResult::None
    }

    fn submit_ok(&mut self) -> TextInputResult {
        let text = self.input_text.clone();
        self.hide();
        if text.is_empty() {
            TextInputResult::Cancel
        } else {
            TextInputResult::Ok(text)
        }
    }
}
