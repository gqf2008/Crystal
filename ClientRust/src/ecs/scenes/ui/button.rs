//! 按钮组件

use ggez::{Context, graphics::{Canvas, Color, Text, TextFragment, PxScale, DrawParam}};
use ggez::glam::Vec2;
use crate::graphics::{LibraryName, draw_sprite_at, draw_sprite_scaled};

#[derive(Debug, Clone)]
pub struct Button {
    pub x: f32,
    pub y: f32,
    pub width: f32,
    pub height: f32,
    pub library: LibraryName,
    pub sprite_index: i32,
    pub hover_sprite_index: i32,
    pub pressed_sprite_index: i32,
    pub hovered: bool,
    pub pressed: bool,
    pub enabled: bool,
    pub visible: bool,
    pub text: String,
    pub center_text: bool,
}

impl Button {
    pub fn new(x: f32, y: f32, library: LibraryName, sprite_index: i32) -> Self {
        Self {
            x, y, width: 80.0, height: 30.0, library, sprite_index,
            hover_sprite_index: sprite_index + 1,
            pressed_sprite_index: sprite_index + 2,
            hovered: false, pressed: false, enabled: true, visible: true,
            text: String::new(),
            center_text: false,
        }
    }
    
    /// 创建带3态索引的按钮 (normal, hover, pressed)
    pub fn new_with_states(x: f32, y: f32, library: LibraryName, normal: i32, hover: i32, pressed: i32) -> Self {
        Self {
            x, y, width: 80.0, height: 30.0, library,
            sprite_index: normal,
            hover_sprite_index: hover,
            pressed_sprite_index: pressed,
            hovered: false, pressed: false, enabled: true, visible: true,
            text: String::new(),
            center_text: false,
        }
    }
    
    pub fn contains(&self, px: f32, py: f32) -> bool {
        if !self.enabled || !self.visible { return false; }
        px >= self.x && px < self.x + self.width && py >= self.y && py < self.y + self.height
    }
    
    pub fn update_hover(&mut self, mouse_x: f32, mouse_y: f32) {
        self.hovered = self.contains(mouse_x, mouse_y);
    }
    
    pub fn draw(&self, ctx: &mut Context, canvas: &mut Canvas) -> anyhow::Result<()> {
        if !self.visible { return Ok(()); }
        let index = if self.pressed && self.enabled {
            self.pressed_sprite_index
        } else if self.hovered && self.enabled {
            self.hover_sprite_index
        } else {
            self.sprite_index
        };
        draw_sprite_at(ctx, canvas, &self.library, index, self.x, self.y)?;
        
        // 渲染文本
        if !self.text.is_empty() {
            let mut text = Text::new(TextFragment {
                text: self.text.clone(),
                color: Some(Color::WHITE),
                scale: Some(PxScale::from(16.0)),
                ..Default::default()
            });
            
            let text_dims = text.measure(ctx)?;
            let text_x = if self.center_text {
                self.x + (self.width - text_dims.x) / 2.0
            } else {
                self.x + 4.0
            };
            let text_y = self.y + (self.height - text_dims.y) / 2.0;
            
            canvas.draw(&text, DrawParam::default().dest(Vec2::new(text_x, text_y)));
        }
        
        Ok(())
    }
}
