// Credits对话框
// 显示游戏版本、开发团队信息等

use ggez::graphics::{Color, PxScale, Text, DrawParam};
use ggez::mint::Vector2;

#[derive(Debug, Clone)]
pub struct CreditsDialog {
    pub visible: bool,
    pub content: Vec<CreditLine>,
}

#[derive(Debug, Clone)]
pub struct CreditLine {
    pub text: String,
    pub font_size: f32,
    pub color: Color,
    pub is_title: bool,
}

impl CreditsDialog {
    pub fn new() -> Self {
        let mut content = Vec::new();
        
        // 游戏标题
        content.push(CreditLine {
            text: "Legend of Mir 2".to_string(),
            font_size: 20.0,
            color: Color::from_rgb(255, 215, 0), // 金色
            is_title: true,
        });
        
        content.push(CreditLine {
            text: "Rust Client".to_string(),
            font_size: 14.0,
            color: Color::from_rgb(180, 180, 180),
            is_title: true,
        });
        
        content.push(CreditLine {
            text: "".to_string(),
            font_size: 8.0,
            color: Color::WHITE,
            is_title: false,
        });
        
        // 版本信息
        content.push(CreditLine {
            text: "Version 0.1.0-alpha".to_string(),
            font_size: 13.0,
            color: Color::WHITE,
            is_title: false,
        });
        
        content.push(CreditLine {
            text: "Build: 2025-10-08".to_string(),
            font_size: 13.0,
            color: Color::WHITE,
            is_title: false,
        });
        
        content.push(CreditLine {
            text: "".to_string(),
            font_size: 8.0,
            color: Color::WHITE,
            is_title: false,
        });
        
        // 技术栈
        content.push(CreditLine {
            text: "Technology".to_string(),
            font_size: 14.0,
            color: Color::from_rgb(100, 200, 255),
            is_title: true,
        });
        
        content.push(CreditLine {
            text: "Rust + ggez + Tokio".to_string(),
            font_size: 12.0,
            color: Color::WHITE,
            is_title: false,
        });
        
        content.push(CreditLine {
            text: "".to_string(),
            font_size: 8.0,
            color: Color::WHITE,
            is_title: false,
        });
        
        // 开发团队
        content.push(CreditLine {
            text: "Development".to_string(),
            font_size: 14.0,
            color: Color::from_rgb(100, 200, 255),
            is_title: true,
        });
        
        content.push(CreditLine {
            text: "Original: Crystal Team".to_string(),
            font_size: 12.0,
            color: Color::WHITE,
            is_title: false,
        });
        
        content.push(CreditLine {
            text: "Rust Port: Community".to_string(),
            font_size: 12.0,
            color: Color::WHITE,
            is_title: false,
        });
        
        content.push(CreditLine {
            text: "".to_string(),
            font_size: 10.0,
            color: Color::WHITE,
            is_title: false,
        });
        
        content.push(CreditLine {
            text: "Press ESC or Click to Close".to_string(),
            font_size: 11.0,
            color: Color::from_rgb(150, 150, 150),
            is_title: true,
        });
        
        Self {
            visible: false,
            content,
        }
    }
    
    pub fn show(&mut self) {
        self.visible = true;
        tracing::info!("📜 Credits对话框打开");
    }
    
    pub fn hide(&mut self) {
        self.visible = false;
        tracing::info!("❌ Credits对话框关闭");
    }
    
    pub fn is_visible(&self) -> bool {
        self.visible
    }
    
    /// 绘制Credits对话框 (使用新的绘制系统)
    pub fn draw(
        &self,
        ctx: &mut ggez::Context,
        canvas: &mut ggez::graphics::Canvas,
    ) -> ggez::GameResult {
        use crate::graphics::{LibraryName, draw_sprite_at};
        use ggez::graphics::{Mesh, Rect, DrawMode};
        
        if !self.visible {
            return Ok(());
        }
        
        // 设计分辨率 1024x768
        let design_width = 1024.0;
        let design_height = 768.0;
        
        // 1. 绘制半透明黑色背景遮罩
        let bg_rect = Mesh::new_rectangle(
            ctx,
            DrawMode::fill(),
            Rect::new(0.0, 0.0, design_width, design_height),
            Color::from_rgba(0, 0, 0, 200),
        )?;
        canvas.draw(&bg_rect, DrawParam::default());
        
        // 2. 绘制对话框背景 (Prguse_360, 464×260)
        // 居中显示，使用单个背景框即可
        let box_width = 464.0;
        let box_height = 260.0;
        let content_x = (design_width - box_width) / 2.0;  // 约280
        let content_y = (design_height - box_height) / 2.0; // 约254
        
        // 绘制单个背景
        let _ = draw_sprite_at(ctx, canvas, &LibraryName::Prguse, 360, content_x, content_y);
        
        // 3. 绘制内容文本
        let mut current_y = content_y + 25.0;  // 顶部边距
        let content_width = box_width;
        let left_margin = 50.0;  // 左边距
        let base_x = content_x + left_margin;
        
        for line in &self.content {
            if line.text.is_empty() {
                current_y += line.font_size * 0.5;  // 空行占半个字体高度
                continue;
            }
            
            let mut text = Text::new(&line.text);
            text.set_font("AlibabaPuHuiTi");
            text.set_scale(PxScale::from(line.font_size));
            
            let x = if line.is_title {
                // 标题居中 - 先测量文本宽度
                let text_size = text.measure(ctx).unwrap_or(Vector2 { x: 200.0, y: 20.0 });
                let text_width = text_size.x;
                content_x + (content_width - text_width) / 2.0
            } else {
                base_x
            };
            
            canvas.draw(&text, DrawParam::default()
                .dest([x, current_y])
                .color(line.color));
            
            // 根据字体大小调整行距
            let line_spacing = if line.is_title { 12.0 } else { 6.0 };
            current_y += line.font_size + line_spacing;
        }
        
        Ok(())
    }
    
    /// 处理点击事件
    pub fn handle_click(&mut self, _x: f32, _y: f32, _window_width: f32, _window_height: f32) {
        // 点击任意位置关闭
        self.hide();
    }
}
