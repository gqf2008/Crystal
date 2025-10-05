// Mail Item Row - 邮件项目行
// 对应C#的MailItemRow类

use crate::scenes::dialogs::Dialog;

/// 邮件项目行
pub struct MailItemRow {
    visible: bool,
}

impl MailItemRow {
    /// 创建新的邮件项目行
    pub fn new() -> Self {
        Self {
            visible: false,
        }
    }
}

impl Dialog for MailItemRow {
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
        "MailItemRow"
    }

    fn contains_point(&self, x: i32, y: i32) -> bool {
        x >= 0 && x < 312 && y >= 0 && y < 30
    }

    fn position(&self) -> (i32, i32) {
        (0, 0)
    }

    fn size(&self) -> (i32, i32) {
        (312, 30)
    }
}

impl Default for MailItemRow {
    fn default() -> Self {
        Self::new()
    }
}