// Hero Info Panel - 英雄信息面板
// 对应C#的HeroInfoPanel类

use crate::scenes::dialogs::Dialog;

/// 英雄信息面板
pub struct HeroInfoPanel {
    visible: bool,
}

impl HeroInfoPanel {
    /// 创建新的英雄信息面板
    pub fn new() -> Self {
        Self {
            visible: false,
        }
    }
}

impl Dialog for HeroInfoPanel {
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
        "HeroInfoPanel"
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

impl Default for HeroInfoPanel {
    fn default() -> Self {
        Self::new()
    }
}