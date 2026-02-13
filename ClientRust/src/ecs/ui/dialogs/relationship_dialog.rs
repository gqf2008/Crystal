// ============================================================================
// 结婚对话框 — RelationshipDialog (对应 C# RelationshipDialog.cs)
// ============================================================================

use ggez::{Context, GameResult};
use ggez::graphics::Canvas;

/// 结婚对话框
pub struct RelationshipDialog {
    pub visible: bool,
    pub position: (f32, f32),
    pub size: (f32, f32),
    /// 伴侣名称
    pub partner_name: String,
    /// 是否已婚
    pub is_married: bool,
    /// 结婚日期
    pub wedding_date: String,
}

impl RelationshipDialog {
    pub fn new() -> Self {
        Self {
            visible: false,
            position: (200.0, 150.0),
            size: (280.0, 250.0),
            partner_name: String::new(),
            is_married: false,
            wedding_date: String::new(),
        }
    }

    pub fn close(&mut self) { self.visible = false; }

    pub fn draw(&self, _ctx: &mut Context, _canvas: &mut Canvas) -> GameResult {
        if !self.visible { return Ok(()); }
        // TODO: 绘制结婚界面
        Ok(())
    }
}

impl Default for RelationshipDialog {
    fn default() -> Self {
        Self::new()
    }
}
