/// MessageBox - Generic message dialog with OK/YesNo buttons
/// Mirrors Client/MirControls/MirMessageBox.cs

use ggez::{Context, GameResult};
use ggez::graphics::{Canvas, Color, DrawParam, Text};
use crate::graphics::libraries::{get_library, LibraryName};

/// 按钮类型
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MessageBoxButtons {
    Ok,
    YesNo,
}

/// 消息框状态
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MessageBoxResult {
    None,
    Ok,
    Yes,
    No,
}

/// 
/// Mirrors C# MirMessageBox:
/// ```csharp
/// public class MirMessageBox : MirImageControl
/// {
///     public MirButton OKButton, NoButton, YesButton;
///     public MirLabel Label;
///     public string Text;
/// }
/// ```
pub struct MessageBox {
    /// 对话框位置
    pub x: f32,
    pub y: f32,
    
    /// 对话框大小
    pub width: f32,
    pub height: f32,
    
    /// 显示的文本
    pub text: String,
    
    /// 按钮类型
    pub buttons: MessageBoxButtons,
    
    /// 是否可见
    pub visible: bool,
    
    /// 用户选择的结果
    pub result: MessageBoxResult,
    
    /// 按钮区域 (用于点击检测)
    ok_button_rect: Option<(f32, f32, f32, f32)>,  // (x, y, width, height)
    yes_button_rect: Option<(f32, f32, f32, f32)>,
    no_button_rect: Option<(f32, f32, f32, f32)>,
    
    /// 按钮悬停状态
    ok_hover: bool,
    yes_hover: bool,
    no_hover: bool,
}

impl MessageBox {
    /// 创建新的消息框
    /// 
    /// Mirrors C# constructor:
    /// ```csharp
    /// public MirMessageBox(string message, MirMessageBoxButtons b = MirMessageBoxButtons.OK)
    /// ```
    pub fn new(text: String, buttons: MessageBoxButtons) -> Self {
        // 居中显示 (基于 1024x768 设计坐标)
        let width = 400.0;
        let height = 200.0;
        let x = (1024.0 - width) / 2.0;
        let y = (768.0 - height) / 2.0;
        
        Self {
            x,
            y,
            width,
            height,
            text,
            buttons,
            visible: false,
            result: MessageBoxResult::None,
            ok_button_rect: None,
            yes_button_rect: None,
            no_button_rect: None,
            ok_hover: false,
            yes_hover: false,
            no_hover: false,
        }
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
    
    /// 是否已经有结果（用户点击了按钮）
    pub fn has_result(&self) -> bool {
        self.result != MessageBoxResult::None
    }
    
    /// 绘制消息框
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
        
        // 绘制对话框背景 (使用 Prguse 394)
        if let Some(lib_arc) = get_library(LibraryName::Prguse) {
            if let Ok(mut lib) = lib_arc.try_lock() {
                let _ = lib.draw_with_color(ctx, canvas, 394, self.x, self.y, Color::WHITE, false);
            }
        }
        
        // 绘制文本（多行支持）
        let text_x = self.x + 20.0;
        let text_y = self.y + 40.0;
        let text_width = self.width - 40.0;
        
        let text_fragment = Text::new(&self.text);
        canvas.draw(
            &text_fragment,
            DrawParam::default()
                .dest([text_x, text_y])
                .color(Color::WHITE)
                .scale([1.0, 1.0]),
        );
        
        // 绘制按钮
        match self.buttons {
            MessageBoxButtons::Ok => {
                self.draw_ok_button(ctx, canvas)?;
            }
            MessageBoxButtons::YesNo => {
                self.draw_yes_no_buttons(ctx, canvas)?;
            }
        }
        
        Ok(())
    }
    
    /// 绘制 OK 按钮
    fn draw_ok_button(&mut self, ctx: &mut Context, canvas: &mut Canvas) -> GameResult {
        let button_x = self.x + (self.width - 80.0) / 2.0;
        let button_y = self.y + self.height - 60.0;
        
        self.ok_button_rect = Some((button_x, button_y, 80.0, 40.0));
        
        // 选择按钮图像索引 (正常/悬停)
        let index = if self.ok_hover { 361 } else { 360 };
        
        if let Some(lib_arc) = get_library(LibraryName::Title) {
            if let Ok(mut lib) = lib_arc.try_lock() {
                let _ = lib.draw_with_color(ctx, canvas, index, button_x, button_y, Color::WHITE, false);
            }
        }
        
        Ok(())
    }
    
    /// 绘制 Yes/No 按钮
    fn draw_yes_no_buttons(&mut self, ctx: &mut Context, canvas: &mut Canvas) -> GameResult {
        let button_y = self.y + self.height - 60.0;
        let yes_x = self.x + (self.width / 2.0) - 90.0;
        let no_x = self.x + (self.width / 2.0) + 10.0;
        
        self.yes_button_rect = Some((yes_x, button_y, 80.0, 40.0));
        self.no_button_rect = Some((no_x, button_y, 80.0, 40.0));
        
        // Yes 按钮
        let yes_index = if self.yes_hover { 361 } else { 360 };
        if let Some(lib_arc) = get_library(LibraryName::Title) {
            if let Ok(mut lib) = lib_arc.try_lock() {
                let _ = lib.draw_with_color(ctx, canvas, yes_index, yes_x, button_y, Color::WHITE, false);
            }
        }
        
        // No 按钮
        let no_index = if self.no_hover { 363 } else { 362 };
        if let Some(lib_arc) = get_library(LibraryName::Title) {
            if let Ok(mut lib) = lib_arc.try_lock() {
                let _ = lib.draw_with_color(ctx, canvas, no_index, no_x, button_y, Color::WHITE, false);
            }
        }
        
        Ok(())
    }
    
    /// 鼠标移动事件
    pub fn on_mouse_move(&mut self, x: f32, y: f32) {
        if !self.visible {
            return;
        }
        
        // 检测悬停
        self.ok_hover = self.is_point_in_rect(x, y, self.ok_button_rect);
        self.yes_hover = self.is_point_in_rect(x, y, self.yes_button_rect);
        self.no_hover = self.is_point_in_rect(x, y, self.no_button_rect);
    }
    
    /// 鼠标点击事件
    pub fn on_mouse_down(&mut self, x: f32, y: f32) -> bool {
        if !self.visible {
            return false;
        }
        
        // 检测点击
        if self.is_point_in_rect(x, y, self.ok_button_rect) {
            self.result = MessageBoxResult::Ok;
            self.visible = false;
            return true;
        }
        
        if self.is_point_in_rect(x, y, self.yes_button_rect) {
            self.result = MessageBoxResult::Yes;
            self.visible = false;
            return true;
        }
        
        if self.is_point_in_rect(x, y, self.no_button_rect) {
            self.result = MessageBoxResult::No;
            self.visible = false;
            return true;
        }
        
        // 点击对话框外部也算处理了事件（不传递到下层）
        true
    }
    
    /// 检测点是否在矩形内
    fn is_point_in_rect(&self, x: f32, y: f32, rect: Option<(f32, f32, f32, f32)>) -> bool {
        if let Some((rx, ry, rw, rh)) = rect {
            x >= rx && x <= rx + rw && y >= ry && y <= ry + rh
        } else {
            false
        }
    }
}
