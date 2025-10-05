// Main Dialog - 主对话框
// 对应C#的MainDialog类

use crate::scenes::dialogs::Dialog;

/// 主对话框
pub struct MainDialog {
    visible: bool,
}

impl MainDialog {
    /// 创建新的主对话框
    pub fn new() -> Self {
        Self {
            visible: true, // 主对话框通常默认可见
        }
    }
}

impl Dialog for MainDialog {
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
        "MainDialog"
    }

    fn contains_point(&self, x: i32, y: i32) -> bool {
        x >= 0 && x < 800 && y >= 0 && y < 600
    }

    fn position(&self) -> (i32, i32) {
        (0, 0)
    }

    fn size(&self) -> (i32, i32) {
        (800, 600)
    }
}

impl Default for MainDialog {
    fn default() -> Self {
        Self::new()
    }
}