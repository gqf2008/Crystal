//! 消息框
//! 对应C#: Client/MirControls/MirMessageBox.cs

use ggez::{Context, graphics::{Canvas, Color, Text, PxScale}};
use crate::graphics::{LibraryName, draw_sprite_at};
use crate::ecs::scenes::ui::Button;

/// 消息框
/// C#原版: Index=360 (Prguse), 按钮在Title库
pub struct MessageBox {
    pub x: f32,
    pub y: f32,
    pub visible: bool,
    pub message: String,
    pub ok_button: Button,
    // 背景尺寸
    width: f32,
    height: f32,
}

impl MessageBox {
    /// 创建新消息框
    /// C#原版: Location = (ScreenWidth - Width) / 2, (ScreenHeight - Height) / 2
    pub fn new(message: String) -> Self {
        // 背景纹理大小约460x220,居中显示
        let screen_w = 1280.0;
        let screen_h = 720.0;
        let box_w = 460.0;
        let box_h = 220.0;
        let x = (screen_w - box_w) / 2.0;
        let y = (screen_h - box_h) / 2.0;
        
        Self {
            x,
            y,
            visible: true,
            message,
            width: box_w,
            height: box_h,
            // C#原版: OK按钮在(360, 157)相对位置,使用Title库200/201/202
            ok_button: Button::new(x + 360.0, y + 157.0, LibraryName::Title, 200),
        }
    }
    
    /// 更新位置以适应窗口大小
    pub fn update_positions(&mut self, screen_w: f32, screen_h: f32) {
        self.x = (screen_w - self.width) / 2.0;
        self.y = (screen_h - self.height) / 2.0;
        self.ok_button.x = self.x + 360.0;
        self.ok_button.y = self.y + 157.0;
    }
    
    /// 更新按钮悬停状态
    pub fn on_mouse_move(&mut self, x: f32, y: f32) {
        self.ok_button.update_hover(x, y);
    }
    
    /// 处理鼠标点击
    pub fn on_mouse_down(&mut self, x: f32, y: f32) -> bool {
        self.ok_button.contains(x, y)
    }
    
    /// 绘制消息框
    pub fn draw(&self, ctx: &mut Context, canvas: &mut Canvas) -> anyhow::Result<()> {
        if !self.visible {
            return Ok(());
        }
        
        // 绘制背景 (Prguse Index=360)
        draw_sprite_at(ctx, canvas, &LibraryName::Prguse, 360, self.x, self.y)?;
        
        // 绘制消息文本 (C#原版: Location=(35, 35), Size=(390, 110))
        let mut text = Text::new(&self.message);
        text.set_scale(PxScale::from(14.0));
        canvas.draw(
            &text,
            ggez::graphics::DrawParam::default()
                .dest([self.x + 35.0, self.y + 35.0])
                .color(Color::WHITE),
        );
        
        // 绘制OK按钮
        self.ok_button.draw(ctx, canvas)?;
        
        Ok(())
    }
}
