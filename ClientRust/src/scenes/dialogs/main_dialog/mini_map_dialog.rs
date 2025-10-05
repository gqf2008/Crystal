// Mini Map Dialog - 小地图对话框
// 对应C#的MiniMapDialog类

use crate::scenes::dialogs::Dialog;

/// 小地图对话框
pub struct MiniMapDialog {
    visible: bool,
}

impl MiniMapDialog {
    /// 创建新的小地图对话框
    pub fn new() -> Self {
        Self {
            visible: true,
        }
    }
}

impl Dialog for MiniMapDialog {
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
        "MiniMapDialog"
    }

    fn contains_point(&self, x: i32, y: i32) -> bool {
        x >= 0 && x < 150 && y >= 0 && y < 150
    }

    fn position(&self) -> (i32, i32) {
        (0, 0)
    }

    fn size(&self) -> (i32, i32) {
        (150, 150)
    }
}

impl Default for MiniMapDialog {
    fn default() -> Self {
        Self::new()
    }
}