/// InputBox - Text input dialog for user confirmation
/// Mirrors Client/MirControls/MirInputBox.cs

use ggez::{Context, GameResult};
use ggez::graphics::{Canvas, Color, DrawParam, Text};
use crate::graphics::libraries::{get_library, LibraryName};
use super::TextInput;

/// 
/// Mirrors C# MirInputBox:
/// ```csharp
/// public class MirInputBox : MirImageControl
/// {
///     public MirButton OKButton, CancelButton;
///     public MirTextBox InputTextBox;
///     public MirLabel Label;
/// }
/// ```
pub struct InputBox {
    /// 对话框位置
    pub x: f32,
    pub y: f32,
    
    /// 对话框大小
    pub width: f32,
    pub height: f32,
    
    /// 提示文本
    pub prompt: String,
    
    /// 输入框
    pub input: TextInput,
    
    /// 是否可见
    pub visible: bool,
    
    /// 是否已确认
    pub confirmed: bool,
    
    /// 是否已取消
    pub cancelled: bool,
    
    /// 按钮区域
    ok_button_rect: (f32, f32, f32, f32),
    cancel_button_rect: (f32, f32, f32, f32),
    
    /// 按钮悬停状态
    ok_hover: bool,
    cancel_hover: bool,
}

impl InputBox {
    /// 创建新的输入框
    /// 
    /// Mirrors C# constructor:
    /// ```csharp
    /// public MirInputBox(string message)
    /// ```
    pub fn new(prompt: String) -> Self {
        // 居中显示
        let width = 400.0;
        let height = 250.0;
        let x = (1024.0 - width) / 2.0;
        let y = (768.0 - height) / 2.0;
        
        // 输入框位置
        let input_x = x + 50.0;
        let input_y = y + 100.0;
        let input_width = width - 100.0;
        
        // 按钮位置
        let button_y = y + height - 60.0;
        let ok_x = x + (width / 2.0) - 90.0;
        let cancel_x = x + (width / 2.0) + 10.0;
        
        Self {
            x,
            y,
            width,
            height,
            prompt,
            input: TextInput::new(input_x, input_y, input_width, 20),
            visible: false,
            confirmed: false,
            cancelled: false,
            ok_button_rect: (ok_x, button_y, 80.0, 40.0),
            cancel_button_rect: (cancel_x, button_y, 80.0, 40.0),
            ok_hover: false,
            cancel_hover: false,
        }
    }
    
    /// 显示输入框
    pub fn show(&mut self) {
        self.visible = true;
        self.confirmed = false;
        self.cancelled = false;
        self.input.text.clear();
        self.input.focused = true;
    }
    
    /// 隐藏输入框
    pub fn hide(&mut self) {
        self.visible = false;
        self.input.focused = false;
    }
    
    /// 获取输入的文本
    pub fn get_input(&self) -> &str {
        &self.input.text
    }
    
    /// 绘制输入框
    pub fn draw(&mut self, ctx: &mut Context, canvas: &mut Canvas) -> GameResult {
        if !self.visible {
            return Ok(());
        }
        
        // 绘制半透明背景遮罩
        let screen_rect = ggez::graphics::Mesh::new_rectangle(
            ctx,
            ggez::graphics::DrawMode::fill(),
            ggez::graphics::Rect::new(0.0, 0.0, 1024.0, 768.0),
            Color::from_rgba(0, 0, 0, 128),
        )?;
        canvas.draw(&screen_rect, DrawParam::default());
        
        // 绘制对话框背景
        if let Some(lib_arc) = get_library(LibraryName::Prguse) {
            if let Ok(mut lib) = lib_arc.try_lock() {
                let _ = lib.draw_with_color(ctx, canvas, 394, self.x, self.y, Color::WHITE, false);
            }
        }
        
        // 绘制提示文本
        let text_x = self.x + 20.0;
        let text_y = self.y + 40.0;
        
        let prompt_text = Text::new(&self.prompt);
        canvas.draw(
            &prompt_text,
            DrawParam::default()
                .dest([text_x, text_y])
                .color(Color::WHITE)
                .scale([1.0, 1.0]),
        );
        
        // 绘制输入框
        let _ = self.input.draw(ctx, canvas);  // 忽略错误，使用 GameResult
        
        // 绘制 OK 按钮
        let (ok_x, ok_y, _, _) = self.ok_button_rect;
        let ok_index = if self.ok_hover { 361 } else { 360 };
        if let Some(lib_arc) = get_library(LibraryName::Title) {
            if let Ok(mut lib) = lib_arc.try_lock() {
                let _ = lib.draw_with_color(ctx, canvas, ok_index, ok_x, ok_y, Color::WHITE, false);
            }
        }
        
        // 绘制 Cancel 按钮
        let (cancel_x, cancel_y, _, _) = self.cancel_button_rect;
        let cancel_index = if self.cancel_hover { 363 } else { 362 };
        if let Some(lib_arc) = get_library(LibraryName::Title) {
            if let Ok(mut lib) = lib_arc.try_lock() {
                let _ = lib.draw_with_color(ctx, canvas, cancel_index, cancel_x, cancel_y, Color::WHITE, false);
            }
        }
        
        Ok(())
    }
    
    /// 鼠标移动事件
    pub fn on_mouse_move(&mut self, x: f32, y: f32) {
        if !self.visible {
            return;
        }
        
        self.ok_hover = self.is_point_in_rect(x, y, self.ok_button_rect);
        self.cancel_hover = self.is_point_in_rect(x, y, self.cancel_button_rect);
    }
    
    /// 鼠标点击事件
    pub fn on_mouse_down(&mut self, x: f32, y: f32) -> bool {
        if !self.visible {
            return false;
        }
        
        // 检测输入框点击 - 简单设置焦点
        let (input_x, input_y, input_width, input_height) = (self.input.x, self.input.y, self.input.width, self.input.height);
        if x >= input_x && x <= input_x + input_width && y >= input_y && y <= input_y + input_height {
            self.input.focused = true;
        }
        
        // 检测 OK 按钮
        if self.is_point_in_rect(x, y, self.ok_button_rect) {
            self.confirmed = true;
            self.visible = false;
            return true;
        }
        
        // 检测 Cancel 按钮
        if self.is_point_in_rect(x, y, self.cancel_button_rect) {
            self.cancelled = true;
            self.visible = false;
            return true;
        }
        
        true
    }
    
    /// 文本输入事件
    pub fn on_text_input(&mut self, text: &str) {
        if self.visible && self.input.focused {
            self.input.add_text(text);
        }
    }
    
    /// 按键事件
    pub fn on_key_down(&mut self, key: ggez::winit::keyboard::KeyCode) {
        if !self.visible {
            return;
        }
        
        use ggez::winit::keyboard::KeyCode;
        
        match key {
            KeyCode::Enter | KeyCode::NumpadEnter => {
                // 回车确认
                self.confirmed = true;
                self.visible = false;
            }
            KeyCode::Escape => {
                // ESC 取消
                self.cancelled = true;
                self.visible = false;
            }
            KeyCode::Backspace => {
                // 退格删除
                if !self.input.text.is_empty() {
                    self.input.text.pop();
                }
            }
            _ => {}
        }
    }
    
    /// 检测点是否在矩形内
    fn is_point_in_rect(&self, x: f32, y: f32, rect: (f32, f32, f32, f32)) -> bool {
        let (rx, ry, rw, rh) = rect;
        x >= rx && x <= rx + rw && y >= ry && y <= ry + rh
    }
}
