// Hero Auto Pot Preview - 英雄自动药水预览
// 对应C#的HeroAutoPotPreview类

use crate::scenes::dialogs::Dialog;

/// 英雄自动药水预览
pub struct HeroAutoPotPreview {
    visible: bool,
}

impl HeroAutoPotPreview {
    /// 创建新的英雄自动药水预览
    pub fn new() -> Self {
        Self {
            visible: false,
        }
    }
}

impl Dialog for HeroAutoPotPreview {
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
        "HeroAutoPotPreview"
    }

    fn contains_point(&self, x: i32, y: i32) -> bool {
        x >= 0 && x < 400 && y >= 0 && y < 300
    }

    fn position(&self) -> (i32, i32) {
        (0, 0)
    }

    fn size(&self) -> (i32, i32) {
        (400, 300)
    }
}

impl Default for HeroAutoPotPreview {
    fn default() -> Self {
        Self::new()
    }
}