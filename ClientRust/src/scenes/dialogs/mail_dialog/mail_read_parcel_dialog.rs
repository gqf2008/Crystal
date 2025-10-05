// Mail Read Parcel Dialog - 邮件阅读包裹对话框
// 对应C#的MailReadParcelDialog类

use crate::scenes::dialogs::Dialog;

/// 邮件阅读包裹对话框
pub struct MailReadParcelDialog {
    visible: bool,
}

impl MailReadParcelDialog {
    /// 创建新的邮件阅读包裹对话框
    pub fn new() -> Self {
        Self {
            visible: false,
        }
    }
}

impl Dialog for MailReadParcelDialog {
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
        "MailReadParcelDialog"
    }

    fn contains_point(&self, x: i32, y: i32) -> bool {
        x >= 0 && x < 400 && y >= 0 && y < 350
    }

    fn position(&self) -> (i32, i32) {
        (0, 0)
    }

    fn size(&self) -> (i32, i32) {
        (400, 350)
    }
}

impl Default for MailReadParcelDialog {
    fn default() -> Self {
        Self::new()
    }
}