// ============================================================================
// 师徒对话框 — MentorDialog (对应 C# MentorDialog.cs)
// ============================================================================

use ggez::{Context, GameResult};
use ggez::graphics::Canvas;

/// 师徒对话框
pub struct MentorDialog {
    pub visible: bool,
    pub position: (f32, f32),
    pub size: (f32, f32),
    /// 师傅名称
    pub mentor_name: String,
    /// 师傅等级
    pub mentor_level: u16,
    /// 是否已有师傅
    pub has_mentor: bool,
    /// 徒弟列表
    pub students: Vec<String>,
}

impl MentorDialog {
    pub fn new() -> Self {
        Self {
            visible: false,
            position: (200.0, 150.0),
            size: (300.0, 280.0),
            mentor_name: String::new(),
            mentor_level: 0,
            has_mentor: false,
            students: Vec::new(),
        }
    }

    pub fn close(&mut self) { self.visible = false; }

    pub fn draw(&self, _ctx: &mut Context, _canvas: &mut Canvas) -> GameResult {
        if !self.visible { return Ok(()); }
        // TODO: 绘制师徒界面
        Ok(())
    }
}

impl Default for MentorDialog {
    fn default() -> Self {
        Self::new()
    }
}
