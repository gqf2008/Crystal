// Fishing Status Dialog - 钓鱼状态对话框
// 显示钓鱼进度和状态信息

use crate::scenes::dialogs::Dialog;

/// 钓鱼状态对话框 - 显示钓鱼进度
#[derive(Debug, Clone)]
pub struct FishingStatusDialog {
    visible: bool,
    pub chance_percent: i32,      // 成功率 (0-100)
    pub progress_percent: i32,     // 进度 (0-100)
    pub auto_cast: bool,           // 自动抛竿
    pub can_auto_cast: bool,       // 是否可自动抛竿
    pub esc_exit: bool,            // ESC退出
}

impl FishingStatusDialog {
    pub fn new() -> Self {
        Self {
            visible: false,
            chance_percent: 0,
            progress_percent: 0,
            auto_cast: false,
            can_auto_cast: false,
            esc_exit: false,
        }
    }

    pub fn show(&mut self) {
        self.visible = true;
    }

    pub fn hide(&mut self) {
        self.visible = false;
    }

    pub fn update_chance(&mut self, percent: i32) {
        self.chance_percent = percent.clamp(0, 100);
    }

    pub fn update_progress(&mut self, percent: i32) {
        self.progress_percent = percent.clamp(0, 100);
    }

    pub fn set_auto_cast(&mut self, enabled: bool) {
        if self.can_auto_cast {
            self.auto_cast = enabled;
        }
    }

    pub fn enable_auto_cast(&mut self, can_auto_cast: bool) {
        self.can_auto_cast = can_auto_cast;
        if !can_auto_cast {
            self.auto_cast = false;
        }
    }

    pub fn set_esc_exit(&mut self, esc_exit: bool) {
        self.esc_exit = esc_exit;
    }

    pub fn is_success(&self) -> bool {
        self.progress_percent >= 100
    }

    pub fn reset_progress(&mut self) {
        self.progress_percent = 0;
        self.chance_percent = 0;
    }

    pub fn get_success_chance(&self) -> f32 {
        self.chance_percent as f32 / 100.0
    }

    pub fn get_progress_ratio(&self) -> f32 {
        self.progress_percent as f32 / 100.0
    }
}

impl Dialog for FishingStatusDialog {
    fn show(&mut self) {
        self.visible = true;
    }

    fn hide(&mut self) {
        self.visible = false;
    }

    fn update(&mut self, _delta_time: f32) {
        // Update fishing progress logic would go here
    }

    fn draw(&self) {
        if !self.visible {
            return;
        }
        // Draw logic would go here
        // - Draw progress bar
        // - Draw chance percentage
        // - Draw auto-cast indicator
    }

    fn is_visible(&self) -> bool {
        self.visible
    }

    fn name(&self) -> &str { "FishingStatusDialog" }
    fn contains_point(&self, x: i32, y: i32) -> bool { x >= 0 && x < 200 && y >= 0 && y < 100 }
    fn position(&self) -> (i32, i32) { (0, 0) }
    fn size(&self) -> (i32, i32) { (200, 100) }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_fishing_status() {
        let mut status = FishingStatusDialog::new();
        assert!(!status.is_visible());

        status.update_chance(75);
        assert_eq!(status.chance_percent, 75);
        assert_eq!(status.get_success_chance(), 0.75);

        status.update_progress(50);
        assert_eq!(status.progress_percent, 50);
        assert_eq!(status.get_progress_ratio(), 0.5);
        assert!(!status.is_success());

        status.update_progress(100);
        assert!(status.is_success());
    }

    #[test]
    fn test_auto_cast() {
        let mut status = FishingStatusDialog::new();

        // Initially cannot auto cast
        status.set_auto_cast(true);
        assert!(!status.auto_cast);

        // Enable auto cast capability
        status.enable_auto_cast(true);
        status.set_auto_cast(true);
        assert!(status.auto_cast);

        // Disable auto cast capability
        status.enable_auto_cast(false);
        assert!(!status.auto_cast);
    }
}