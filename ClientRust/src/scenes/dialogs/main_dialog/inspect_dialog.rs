// Inspect Dialog - 检查对话框
// 对应C#的InspectDialog类

use crate::scenes::dialogs::Dialog;

/// 检查对话框
pub struct InspectDialog {
    visible: bool,
}

impl InspectDialog {
    /// 创建新的检查对话框
    pub fn new() -> Self {
        Self {
            visible: false,
        }
    }
}

impl Dialog for InspectDialog {
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
        "InspectDialog"
    }

    fn contains_point(&self, x: i32, y: i32) -> bool {
        x >= 0 && x < 300 && y >= 0 && y < 400
    }

    fn position(&self) -> (i32, i32) {
        (0, 0)
    }

    fn size(&self) -> (i32, i32) {
        (300, 400)
    }
}

impl Default for InspectDialog {
    fn default() -> Self {
        Self::new()
    }
}