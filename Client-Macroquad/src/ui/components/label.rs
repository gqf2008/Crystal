// ============================================================================
// TexturedLabel - 增强型文本标签组件
// ============================================================================

use egui_macroquad::egui;

/// 增强型文本标签，支持颜色、对齐和（未来的）描边效果
#[derive(Debug, Clone)]
pub struct TexturedLabel {
    pub text: String,
    pub color: egui::Color32,
    pub align: egui::Align2,
    pub font_size: f32,
    // TODO: 实现描边效果
    pub outline: bool,
    pub outline_color: egui::Color32,
}

impl TexturedLabel {
    pub fn new(text: impl Into<String>) -> Self {
        Self {
            text: text.into(),
            color: egui::Color32::WHITE,
            align: egui::Align2::LEFT_TOP,
            font_size: 14.0, // 默认字体大小
            outline: false,
            outline_color: egui::Color32::BLACK,
        }
    }

    pub fn with_color(mut self, color: egui::Color32) -> Self {
        self.color = color;
        self
    }

    pub fn with_align(mut self, align: egui::Align2) -> Self {
        self.align = align;
        self
    }

    pub fn with_font_size(mut self, size: f32) -> Self {
        self.font_size = size;
        self
    }

    pub fn with_outline(mut self, outline: bool) -> Self {
        self.outline = outline;
        self
    }

    /// 在指定区域绘制标签
    pub fn draw_at(&self, ui: &mut egui::Ui, rect: egui::Rect) {
        let font_id = egui::FontId::proportional(self.font_size);
        
        // 计算文本位置
        let galley = ui.painter().layout_no_wrap(
            self.text.clone(),
            font_id.clone(),
            self.color,
        );

        let text_size = galley.size();
        let pos = self.align.pos_in_rect(&rect.shrink(2.0)); // 稍微留点边距
        
        // 简单的对齐调整
        let draw_pos = match self.align {
            egui::Align2::LEFT_TOP => pos,
            egui::Align2::CENTER_CENTER => pos - text_size / 2.0,
            egui::Align2::CENTER_TOP => pos - egui::vec2(text_size.x / 2.0, 0.0),
            _ => pos, // 其他对齐方式暂略，按需完善
        };

        // 模拟描边（简单的阴影效果）
        if self.outline {
            ui.painter().text(
                draw_pos + egui::vec2(1.0, 1.0),
                egui::Align2::LEFT_TOP,
                &self.text,
                font_id.clone(),
                self.outline_color,
            );
        }

        ui.painter().galley(draw_pos, galley, self.color);
    }

    /// 在当前布局中绘制标签
    pub fn draw(&self, ui: &mut egui::Ui) {
        let text = egui::RichText::new(&self.text)
            .color(self.color)
            .size(self.font_size);
        
        ui.label(text);
    }
}
