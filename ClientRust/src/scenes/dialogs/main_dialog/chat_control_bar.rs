// Chat Control Bar - 聊天控制栏
// 对应C#的ChatControlBar类

use crate::scenes::dialogs::Dialog;

/// 聊天控制栏
pub struct ChatControlBar {
    visible: bool,
}

impl ChatControlBar {
    /// 创建新的聊天控制栏
    pub fn new() -> Self {
        Self {
            visible: true,
        }
    }
}

impl Dialog for ChatControlBar {
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
        "ChatControlBar"
    }

    fn contains_point(&self, x: i32, y: i32) -> bool {
        x >= 0 && x < 200 && y >= 0 && y < 30
    }

    fn position(&self) -> (i32, i32) {
        (0, 0)
    }

    fn size(&self) -> (i32, i32) {
        (200, 30)
    }
}

impl Default for ChatControlBar {
    fn default() -> Self {
        Self::new()
    }
}