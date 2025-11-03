//! 消息框
//! 对应C#: Client/MirControls/MirMessageBox.cs

use crate::ecs::scenes::ui::Button;
use crate::graphics::{draw_sprite_at, LibraryName};
use ggez::{
    graphics::{Canvas, Color, PxScale, Text},
    Context,
};

/// 消息框
/// C#原版: Index=360 (Prguse), 按钮在Title库
pub struct MessageBox {
    pub x: f32,
    pub y: f32,
    pub visible: bool,
    pub message: String,
    pub ok_button: Button,
}

impl MessageBox {
    // 相对于MessageBox纹理(Prguse 360)的偏移量
    const OFFSET_OK_BUTTON_X: f32 = 360.0;
    const OFFSET_OK_BUTTON_Y: f32 = 157.0;
    const OFFSET_TEXT_X: f32 = 35.0;
    const OFFSET_TEXT_Y: f32 = 35.0;

    pub fn new(message: String, screen_w: f32, screen_h: f32) -> Self {
        // TODO: 从纹理库获取实际尺寸，暂时使用估算值
        let box_w = 460.0;
        let box_h = 220.0;

        let x = (screen_w - box_w) / 2.0;
        let y = (screen_h - box_h) / 2.0;

        Self {
            x,
            y,
            visible: true,
            message,
            ok_button: Button::new(
                x + Self::OFFSET_OK_BUTTON_X,
                y + Self::OFFSET_OK_BUTTON_Y,
                LibraryName::Title,
                200,
            ),
        }
    }

    /// 更新位置以适应窗口大小（在设计坐标系中居中）
    pub fn update_positions(&mut self, screen_w: f32, screen_h: f32) {
        let box_w = 460.0; // TODO: 从纹理获取
        let box_h = 220.0;

        self.x = (screen_w - box_w) / 2.0;
        self.y = (screen_h - box_h) / 2.0;
        self.ok_button.x = self.x + Self::OFFSET_OK_BUTTON_X;
        self.ok_button.y = self.y + Self::OFFSET_OK_BUTTON_Y;
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
    pub fn draw(&self, ctx: &mut ggez::graphics::GraphicsContext, canvas: &mut Canvas) -> anyhow::Result<()> {
        if !self.visible {
            return Ok(());
        }

        // 绘制背景 (Prguse Index=360)
        draw_sprite_at(ctx, canvas, &LibraryName::Prguse, 360, self.x, self.y)?;

        // 绘制消息文本（使用相对纹理的偏移）
        let mut text = Text::new(&self.message);
        text.set_font("AlibabaPuHuiTi")
            .set_scale(PxScale::from(14.0));
        canvas.draw(
            &text,
            ggez::graphics::DrawParam::default()
                .dest([self.x + Self::OFFSET_TEXT_X, self.y + Self::OFFSET_TEXT_Y])
                .color(Color::WHITE),
        );

        // 绘制OK按钮
        self.ok_button.draw(ctx, canvas)?;

        Ok(())
    }
}

