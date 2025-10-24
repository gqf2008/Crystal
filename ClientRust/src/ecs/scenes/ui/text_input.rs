//! 文本输入框组件

use ggez::{Context, graphics::{Canvas, Color, DrawMode, Rect, Mesh, Text, PxScale}};

#[derive(Debug, Clone)]
pub struct TextInput {
    pub x: f32, pub y: f32, pub width: f32, pub height: f32,
    pub text: String, pub focused: bool, pub enabled: bool, pub visible: bool,
    pub password_mode: bool, pub max_length: usize,
    cursor_visible: bool, cursor_timer: f32,
}

impl TextInput {
    pub fn new(x: f32, y: f32, width: f32, max_length: usize) -> Self {
        Self {
            x, y, width, height: 20.0, text: String::new(),
            focused: false, enabled: true, visible: true, password_mode: false,
            max_length, cursor_visible: true, cursor_timer: 0.0,
        }
    }
    
    pub fn password(mut self) -> Self { self.password_mode = true; self }
    pub fn contains(&self, px: f32, py: f32) -> bool {
        if !self.enabled || !self.visible { return false; }
        px >= self.x && px < self.x + self.width && py >= self.y && py < self.y + self.height
    }
    pub fn add_char(&mut self, c: char) { if self.text.len() < self.max_length { self.text.push(c); } }
    
    /// 添加文本 (用于 IME 中文输入)
    pub fn add_text(&mut self, text: &str) {
        for c in text.chars() {
            if self.text.len() < self.max_length {
                self.text.push(c);
            } else {
                break;
            }
        }
    }
    
    pub fn backspace(&mut self) { self.text.pop(); }
    pub fn clear(&mut self) { self.text.clear(); }
    
    pub fn update(&mut self, dt: f32) {
        if self.focused {
            self.cursor_timer += dt;
            if self.cursor_timer >= 0.5 {
                self.cursor_visible = !self.cursor_visible;
                self.cursor_timer = 0.0;
            }
        } else {
            self.cursor_visible = false;
        }
    }
    
    pub fn draw(&self, ctx: &mut Context, canvas: &mut Canvas) -> anyhow::Result<()> {
        if !self.visible { return Ok(()); }
        let rect = Rect::new(self.x, self.y, self.width, self.height);
        
        // 半透明黑色背景，焦点时稍微亮一些
        let color = if self.focused { 
            Color::from_rgba(40, 40, 40, 200)  // 焦点：深灰半透明
        } else { 
            Color::from_rgba(20, 20, 20, 180)  // 非焦点：更深的灰色半透明
        };
        
        let bg = Mesh::new_rectangle(ctx, DrawMode::fill(), rect, color)?;
        canvas.draw(&bg, ggez::graphics::DrawParam::default());
        
        // 边框：焦点时金色，非焦点时深灰
        let border_color = if self.focused {
            Color::from_rgb(200, 180, 100)  // 金色边框
        } else {
            Color::from_rgb(80, 80, 80)     // 深灰边框
        };
        let border = Mesh::new_rectangle(ctx, DrawMode::stroke(1.0), rect, border_color)?;
        canvas.draw(&border, ggez::graphics::DrawParam::default());
        
        // 文本：白色 + 中文字体
        let display_text = if self.password_mode { "*".repeat(self.text.len()) } else { self.text.clone() };
        let mut text = Text::new(display_text.clone());
        text.set_font("AlibabaPuHuiTi");  // ✅ 使用中文字体
        text.set_scale(PxScale::from(16.0));
        canvas.draw(&text, ggez::graphics::DrawParam::default()
            .dest([self.x + 5.0, self.y + 2.0])
            .color(Color::WHITE));
        
        // 光标：白色 - 使用实际文本宽度计算位置
        if self.focused && self.cursor_visible {
            // 测量实际文本宽度
            let text_width = if !display_text.is_empty() {
                text.measure(ctx).map(|dim| dim.x).unwrap_or(0.0)
            } else {
                0.0
            };
            let cursor_x = self.x + 5.0 + text_width;
            let cursor = Mesh::new_rectangle(ctx, DrawMode::fill(), 
                Rect::new(cursor_x, self.y + 2.0, 2.0, 16.0), Color::WHITE)?;
            canvas.draw(&cursor, ggez::graphics::DrawParam::default());
        }
        Ok(())
    }
}
