// Skill Bar Dialog - 技能栏对话框
// 对应C#的SkillBarDialog类

use crate::scenes::dialogs::Dialog;

/// 技能栏对话框
pub struct SkillBarDialog {
    visible: bool,
}

impl SkillBarDialog {
    /// 创建新的技能栏对话框
    pub fn new() -> Self {
        Self {
            visible: true,
        }
    }
}

impl Dialog for SkillBarDialog {
    fn show(&mut self) {
        self.visible = true;
    }

    fn hide(&mut self) {
        self.visible = false;
    }

    fn is_visible(&self) -> bool {
        self.visible
    }

    fn toggle(&mut self) {
        self.visible = !self.visible;
    }

    fn update(&mut self, _delta_time: f32) {
        // Update logic
    }

    fn draw(&self) {
        if !self.visible {
            return;
        }
        // Draw logic
    }

    fn name(&self) -> &str {
        "SkillBarDialog"
    }

    fn contains_point(&self, x: i32, y: i32) -> bool {
        x >= 0 && x < 400 && y >= 0 && y < 50
    }

    fn position(&self) -> (i32, i32) {
        (0, 0)
    }

    fn size(&self) -> (i32, i32) {
        (400, 50)
    }
}

impl Default for SkillBarDialog {
    fn default() -> Self {
        Self::new()
    }
}