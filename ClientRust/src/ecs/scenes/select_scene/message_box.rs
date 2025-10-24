//! SelectScene 消息框
//! 基于 LoginScene 的消息框实现，使用正确的纹理和按钮系统

use ggez::{Context, graphics::{Canvas, Color, Text, PxScale}};
use crate::graphics::{LibraryName, draw_sprite_at};
use crate::ecs::scenes::ui::Button;

/// 消息框（与 LoginScene 一致）
/// C#原版: Index=360 (Prguse), 按钮在Title库
pub struct MessageBox {
    pub x: f32,
    pub y: f32,
    pub visible: bool,
    pub message: String,
    pub ok_button: Button,
    pub yes_button: Option<Button>,
    pub no_button: Option<Button>,
    pub buttons_type: MessageBoxButtons,
    pub result: MessageBoxResult,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MessageBoxButtons {
    Ok,
    YesNo,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MessageBoxResult {
    None,
    Ok,
    Yes,
    No,
}

impl MessageBox {
    // 相对于MessageBox纹理(Prguse 360)的偏移量
    // C# 原版: OKButton位置(360,157), YesButton位置(260,157), NoButton位置(360,157)
    const OFFSET_OK_BUTTON_X: f32 = 360.0;
    const OFFSET_OK_BUTTON_Y: f32 = 157.0;
    const OFFSET_YES_BUTTON_X: f32 = 260.0;  // Yes按钮位置 (C#原版)
    const OFFSET_YES_BUTTON_Y: f32 = 157.0;
    const OFFSET_NO_BUTTON_X: f32 = 360.0;   // No按钮位置 (C#原版)
    const OFFSET_NO_BUTTON_Y: f32 = 157.0;
    const OFFSET_TEXT_X: f32 = 35.0;
    const OFFSET_TEXT_Y: f32 = 35.0;
    
    pub fn new(message: String, buttons: MessageBoxButtons, screen_w: f32, screen_h: f32) -> Self {
        // TODO: 从纹理库获取实际尺寸，暂时使用估算值
        let box_w = 460.0;
        let box_h = 220.0;
        
        let x = (screen_w - box_w) / 2.0;
        let y = (screen_h - box_h) / 2.0;
        
        let (ok_button, yes_button, no_button) = match buttons {
            MessageBoxButtons::Ok => {
                (
                    Button::new(x + Self::OFFSET_OK_BUTTON_X, y + Self::OFFSET_OK_BUTTON_Y, LibraryName::Title, 200),
                    None,
                    None
                )
            }
            MessageBoxButtons::YesNo => {
                (
                    Button::new(0.0, 0.0, LibraryName::Title, 200), // 占位符，不使用
                    // C# 原版: Yes按钮索引206(Normal), 207(Hover), 208(Pressed)
                    Some(Button::new(x + Self::OFFSET_YES_BUTTON_X, y + Self::OFFSET_YES_BUTTON_Y, LibraryName::Title, 206)),
                    // C# 原版: No按钮索引210(Normal), 211(Hover), 212(Pressed)
                    Some(Button::new(x + Self::OFFSET_NO_BUTTON_X, y + Self::OFFSET_NO_BUTTON_Y, LibraryName::Title, 210))
                )
            }
        };
        
        Self {
            x,
            y,
            visible: true,
            message,
            ok_button,
            yes_button,
            no_button,
            buttons_type: buttons,
            result: MessageBoxResult::None,
        }
    }
    
    pub fn show(&mut self) {
        self.visible = true;
        self.result = MessageBoxResult::None;
    }
    
    pub fn hide(&mut self) {
        self.visible = false;
    }
    
    /// 更新位置以适应窗口大小（在设计坐标系中居中）
    pub fn update_positions(&mut self, screen_w: f32, screen_h: f32) {
        let box_w = 460.0;
        let box_h = 220.0;
        
        self.x = (screen_w - box_w) / 2.0;
        self.y = (screen_h - box_h) / 2.0;
        
        match self.buttons_type {
            MessageBoxButtons::Ok => {
                self.ok_button.x = self.x + Self::OFFSET_OK_BUTTON_X;
                self.ok_button.y = self.y + Self::OFFSET_OK_BUTTON_Y;
            }
            MessageBoxButtons::YesNo => {
                if let Some(ref mut btn) = self.yes_button {
                    btn.x = self.x + Self::OFFSET_YES_BUTTON_X;
                    btn.y = self.y + Self::OFFSET_YES_BUTTON_Y;
                }
                if let Some(ref mut btn) = self.no_button {
                    btn.x = self.x + Self::OFFSET_NO_BUTTON_X;
                    btn.y = self.y + Self::OFFSET_NO_BUTTON_Y;
                }
            }
        }
    }
    
    /// 更新按钮悬停状态
    pub fn on_mouse_move(&mut self, x: f32, y: f32) {
        match self.buttons_type {
            MessageBoxButtons::Ok => {
                self.ok_button.update_hover(x, y);
            }
            MessageBoxButtons::YesNo => {
                if let Some(ref mut btn) = self.yes_button {
                    btn.update_hover(x, y);
                }
                if let Some(ref mut btn) = self.no_button {
                    btn.update_hover(x, y);
                }
            }
        }
    }
    
    /// 处理鼠标点击，返回是否点击了按钮
    pub fn on_mouse_down(&mut self, x: f32, y: f32) -> bool {
        if !self.visible {
            return false;
        }
        
        match self.buttons_type {
            MessageBoxButtons::Ok => {
                if self.ok_button.contains(x, y) {
                    self.result = MessageBoxResult::Ok;
                    self.visible = false;
                    return true;
                }
            }
            MessageBoxButtons::YesNo => {
                if let Some(ref btn) = self.yes_button {
                    if btn.contains(x, y) {
                        self.result = MessageBoxResult::Yes;
                        self.visible = false;
                        return true;
                    }
                }
                if let Some(ref btn) = self.no_button {
                    if btn.contains(x, y) {
                        self.result = MessageBoxResult::No;
                        self.visible = false;
                        return true;
                    }
                }
            }
        }
        
        // 点击对话框外部也消费事件（不传递到下层）
        true
    }
    
    /// 检查是否有结果
    pub fn has_result(&self) -> bool {
        self.result != MessageBoxResult::None
    }
    
    /// 绘制消息框
    pub fn draw(&self, ctx: &mut Context, canvas: &mut Canvas) -> anyhow::Result<()> {
        if !self.visible {
            return Ok(());
        }
        
        // 绘制背景 (Prguse Index=360)
        draw_sprite_at(ctx, canvas, &LibraryName::Prguse, 360, self.x, self.y)?;
        
        // 绘制消息文本（支持换行和中文字体）
        // 手动按 \n 分割并逐行绘制
        let line_height = 20.0; // 行高
        let mut y_offset = 0.0;
        
        for line in self.message.lines() {
            let mut text = Text::new(line);
            text.set_scale(PxScale::from(14.0));
            text.set_font("AlibabaPuHuiTi"); // 使用中文字体
            
            // 绘制当前行
            canvas.draw(
                &text,
                ggez::graphics::DrawParam::default()
                    .dest([self.x + Self::OFFSET_TEXT_X, self.y + Self::OFFSET_TEXT_Y + y_offset])
                    .color(Color::WHITE),
            );
            
            y_offset += line_height;
        }
        
        // 绘制按钮
        match self.buttons_type {
            MessageBoxButtons::Ok => {
                self.ok_button.draw(ctx, canvas)?;
            }
            MessageBoxButtons::YesNo => {
                if let Some(ref btn) = self.yes_button {
                    btn.draw(ctx, canvas)?;
                }
                if let Some(ref btn) = self.no_button {
                    btn.draw(ctx, canvas)?;
                }
            }
        }
        
        Ok(())
    }
}
