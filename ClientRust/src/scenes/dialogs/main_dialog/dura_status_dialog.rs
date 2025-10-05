// Dura Status Dialog - 耐久状态对话框
// 对应C#的DuraStatusDialog类

use crate::scenes::dialogs::Dialog;

/// 耐久状态对话框
pub struct DuraStatusDialog {
    visible: bool,
}

impl DuraStatusDialog {
    /// 创建新的耐久状态对话框
    pub fn new() -> Self {
        Self {
            visible: true,
        }
    }
}

impl Dialog for DuraStatusDialog {
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
        "DuraStatusDialog"
    }

    fn contains_point(&self, x: i32, y: i32) -> bool {
        x >= 0 && x < 200 && y >= 0 && y < 100
    }

    fn position(&self) -> (i32, i32) {
        (0, 0)
    }

    fn size(&self) -> (i32, i32) {
        (200, 100)
    }
}

impl Default for DuraStatusDialog {
    fn default() -> Self {
        Self::new()
    }
}