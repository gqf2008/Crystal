// ============================================================================
// AmountBoxHybrid - 数量输入框（对齐 C# MirAmountBox）
// ============================================================================
//
// C# 参考：Client/MirControls/MirAmountBox.cs
// - 背景：Prguse[238]
// - 关闭按钮：Prguse2[360-362]
// - OK：Title[200-202]
// - Cancel：Title[203-205]
// - 标题：x=19 y=8
// - 物品图标区域：x=15 y=34 w=38 h=34
// - 输入框区域：x=58 y=43 w=132 h=19
// - OK/Cancel：x=23/110 y=76

use macroquad::prelude::*;

use crate::resources::LibraryName;
use crate::scenes::dialogs::game::native_ui_utils::{ButtonState, ButtonTextures};
use crate::ui::text_renderer::draw_text_cn;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AmountBoxResult {
    None,
    Ok(u32),
    Cancel,
}

pub struct AmountBoxHybrid {
    visible: bool,
    title: String,
    image_index: u16,
    min_amount: u32,
    max_amount: u32,
    amount_text: String,

    bg_texture: Option<Texture2D>,
    bg_size: Vec2,

    close_btn: ButtonTextures,
    ok_btn: ButtonTextures,
    cancel_btn: ButtonTextures,
}

impl Default for AmountBoxHybrid {
    fn default() -> Self {
        Self::new()
    }
}

impl AmountBoxHybrid {
    const BG_INDEX: usize = 238;

    const TITLE_X: f32 = 19.0;
    const TITLE_Y: f32 = 8.0;

    const CLOSE_X: f32 = 180.0;
    const CLOSE_Y: f32 = 3.0;

    const ITEM_X: f32 = 15.0;
    const ITEM_Y: f32 = 34.0;
    const ITEM_W: f32 = 38.0;
    const ITEM_H: f32 = 34.0;

    const INPUT_X: f32 = 58.0;
    const INPUT_Y: f32 = 43.0;
    const INPUT_W: f32 = 132.0;
    const INPUT_H: f32 = 19.0;

    const OK_X: f32 = 23.0;
    const OK_Y: f32 = 76.0;
    const CANCEL_X: f32 = 110.0;
    const CANCEL_Y: f32 = 76.0;

    pub fn new() -> Self {
        Self {
            visible: false,
            title: String::new(),
            image_index: 0,
            min_amount: 0,
            max_amount: 0,
            amount_text: String::new(),

            bg_texture: None,
            bg_size: vec2(0.0, 0.0),

            close_btn: ButtonTextures::load_from_library(LibraryName::Prguse2, 360),
            ok_btn: ButtonTextures::load_from_library(LibraryName::Title, 200),
            cancel_btn: ButtonTextures::load_from_library(LibraryName::Title, 203),
        }
    }

    pub fn is_visible(&self) -> bool {
        self.visible
    }

    pub fn hide(&mut self) {
        self.visible = false;
        self.amount_text.clear();
    }

    pub fn show(
        &mut self,
        title: impl Into<String>,
        image_index: u16,
        max: u32,
        min: u32,
        default_amount: u32,
    ) {
        self.title = title.into();
        self.image_index = image_index;
        self.max_amount = max;
        self.min_amount = min;

        let default_amount = default_amount.clamp(min, max);
        self.amount_text = default_amount.to_string();
        self.visible = true;
        self.ensure_textures_loaded();
    }

    fn ensure_textures_loaded(&mut self) {
        if self.bg_texture.is_some() {
            return;
        }
        if let Some(info) = LibraryName::Prguse.get_texture(Self::BG_INDEX) {
            self.bg_texture = info.image;
            self.bg_size = vec2(info.width as f32, info.height as f32);
        } else {
            self.bg_size = vec2(200.0, 110.0);
        }
    }

    fn rect(&self) -> Rect {
        let sw = screen_width() / screen_dpi_scale();
        let sh = screen_height() / screen_dpi_scale();
        let w = self.bg_size.x.max(200.0);
        let h = self.bg_size.y.max(110.0);
        Rect::new((sw - w) / 2.0, (sh - h) / 2.0, w, h)
    }

    pub fn is_mouse_over(&self, mouse_pos: Vec2) -> bool {
        self.visible && self.rect().contains(mouse_pos)
    }

    fn parse_amount(&self) -> Option<u32> {
        self.amount_text.trim().parse::<u32>().ok()
    }

    fn validate(&self) -> (bool, u32) {
        let Some(mut amt) = self.parse_amount() else {
            return (false, self.min_amount);
        };
        if amt < self.min_amount {
            return (false, self.min_amount);
        }
        if amt > self.max_amount {
            amt = self.max_amount;
        }
        (true, amt)
    }

    fn handle_keyboard(&mut self) {
        for key in [
            KeyCode::Key0,
            KeyCode::Key1,
            KeyCode::Key2,
            KeyCode::Key3,
            KeyCode::Key4,
            KeyCode::Key5,
            KeyCode::Key6,
            KeyCode::Key7,
            KeyCode::Key8,
            KeyCode::Key9,
            KeyCode::Kp0,
            KeyCode::Kp1,
            KeyCode::Kp2,
            KeyCode::Kp3,
            KeyCode::Kp4,
            KeyCode::Kp5,
            KeyCode::Kp6,
            KeyCode::Kp7,
            KeyCode::Kp8,
            KeyCode::Kp9,
        ] {
            if is_key_pressed(key) {
                let digit = match key {
                    KeyCode::Key0 | KeyCode::Kp0 => '0',
                    KeyCode::Key1 | KeyCode::Kp1 => '1',
                    KeyCode::Key2 | KeyCode::Kp2 => '2',
                    KeyCode::Key3 | KeyCode::Kp3 => '3',
                    KeyCode::Key4 | KeyCode::Kp4 => '4',
                    KeyCode::Key5 | KeyCode::Kp5 => '5',
                    KeyCode::Key6 | KeyCode::Kp6 => '6',
                    KeyCode::Key7 | KeyCode::Kp7 => '7',
                    KeyCode::Key8 | KeyCode::Kp8 => '8',
                    KeyCode::Key9 | KeyCode::Kp9 => '9',
                    _ => continue,
                };

                if self.amount_text == "0" {
                    self.amount_text.clear();
                }
                if self.amount_text.len() < 9 {
                    self.amount_text.push(digit);
                }
            }
        }

        if is_key_pressed(KeyCode::Backspace) {
            self.amount_text.pop();
        }

        if is_key_pressed(KeyCode::Escape) {
            self.hide();
        }
    }

    pub fn update_and_draw(&mut self) -> AmountBoxResult {
        if !self.visible {
            return AmountBoxResult::None;
        }

        self.ensure_textures_loaded();
        self.handle_keyboard();

        let (mx, my) = mouse_position();
        let mouse_pos = vec2(mx, my);
        let rect = self.rect();

        // 背景
        if let Some(bg) = self.bg_texture.as_ref() {
            draw_texture(bg, rect.x, rect.y, WHITE);
        } else {
            draw_rectangle(
                rect.x,
                rect.y,
                rect.w,
                rect.h,
                Color::new(0.0, 0.0, 0.0, 0.75),
            );
        }

        // 标题
        draw_text_cn(
            &self.title,
            rect.x + Self::TITLE_X,
            rect.y + Self::TITLE_Y + 14.0,
            14.0,
            WHITE,
        );

        // 关闭
        let close_rect = Rect::new(
            rect.x + Self::CLOSE_X,
            rect.y + Self::CLOSE_Y,
            self.close_btn.size.x,
            self.close_btn.size.y,
        );
        let close_state = ButtonState::from_mouse(close_rect, mouse_pos);
        self.close_btn
            .draw(vec2(close_rect.x, close_rect.y), close_state);
        if ButtonState::is_clicked(close_rect, mouse_pos) {
            self.hide();
            return AmountBoxResult::Cancel;
        }

        // 图标
        if let Some(img) = LibraryName::Items
            .get_texture(self.image_index as usize)
            .and_then(|i| i.image)
        {
            let icon_w = img.width();
            let icon_h = img.height();
            let ox = rect.x + Self::ITEM_X + (Self::ITEM_W - icon_w) / 2.0;
            let oy = rect.y + Self::ITEM_Y + (Self::ITEM_H - icon_h) / 2.0;
            draw_texture(&img, ox, oy, WHITE);
        }

        // 输入框
        let (ok_visible, clamped_amount) = self.validate();
        let mut border = Color::new(0.0, 1.0, 0.0, 1.0);
        if !ok_visible {
            border = Color::new(1.0, 0.0, 0.0, 1.0);
        } else if clamped_amount == self.max_amount {
            border = Color::new(1.0, 0.65, 0.0, 1.0);
        }

        let input_rect = Rect::new(
            rect.x + Self::INPUT_X,
            rect.y + Self::INPUT_Y,
            Self::INPUT_W,
            Self::INPUT_H,
        );
        draw_rectangle(
            input_rect.x,
            input_rect.y,
            input_rect.w,
            input_rect.h,
            Color::new(0.0, 0.0, 0.0, 0.35),
        );
        draw_rectangle_lines(
            input_rect.x,
            input_rect.y,
            input_rect.w,
            input_rect.h,
            1.0,
            border,
        );
        draw_text_cn(
            &self.amount_text,
            input_rect.x + 6.0,
            input_rect.y + 14.0,
            14.0,
            WHITE,
        );

        // Cancel
        let cancel_rect = Rect::new(
            rect.x + Self::CANCEL_X,
            rect.y + Self::CANCEL_Y,
            self.cancel_btn.size.x,
            self.cancel_btn.size.y,
        );
        let cancel_state = ButtonState::from_mouse(cancel_rect, mouse_pos);
        self.cancel_btn
            .draw(vec2(cancel_rect.x, cancel_rect.y), cancel_state);
        if ButtonState::is_clicked(cancel_rect, mouse_pos) || is_key_pressed(KeyCode::Escape) {
            self.hide();
            return AmountBoxResult::Cancel;
        }

        // OK
        let ok_rect = Rect::new(
            rect.x + Self::OK_X,
            rect.y + Self::OK_Y,
            self.ok_btn.size.x,
            self.ok_btn.size.y,
        );
        let ok_state = ButtonState::from_mouse(ok_rect, mouse_pos);
        self.ok_btn.draw(vec2(ok_rect.x, ok_rect.y), ok_state);
        if ok_visible
            && (ButtonState::is_clicked(ok_rect, mouse_pos) || is_key_pressed(KeyCode::Enter))
        {
            self.hide();
            return AmountBoxResult::Ok(clamped_amount);
        }

        AmountBoxResult::None
    }
}
