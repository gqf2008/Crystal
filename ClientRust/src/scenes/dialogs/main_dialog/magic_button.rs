// Magic Button - 魔法按钮
// 对应C#的MagicButton类

use crate::scenes::dialogs::Dialog;

/// 魔法按钮
pub struct MagicButton {
    visible: bool,
}

impl MagicButton {
    /// 创建新的魔法按钮
    pub fn new() -> Self {
        Self {
            visible: true,
        }
    }
}

impl Dialog for MagicButton {
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
        "MagicButton"
    }

    fn contains_point(&self, x: i32, y: i32) -> bool {
        x >= 0 && x < 40 && y >= 0 && y < 40
    }

    fn position(&self) -> (i32, i32) {
        (0, 0)
    }

    fn size(&self) -> (i32, i32) {
        (40, 40)
    }
}

impl Default for MagicButton {
    fn default() -> Self {
        Self::new()
    }
}