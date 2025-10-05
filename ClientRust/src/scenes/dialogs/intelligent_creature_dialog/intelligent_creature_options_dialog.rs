// IntelligentCreatureOptionsDialog - 智能生物选项对话框
// 对应C#的IntelligentCreatureOptionsDialog类

use crate::scenes::dialogs::Dialog;

/// Intelligent creature options dialog - 智能生物选项对话框
#[derive(Debug)]
pub struct IntelligentCreatureOptionsDialog {
    pub visible: bool,
    pub x: i32,
    pub y: i32,
    pub width: i32,
    pub height: i32,

    // 选项设置
    pub auto_pickup_enabled: bool,
    pub auto_revive_enabled: bool,
    pub auto_defend_enabled: bool,
    pub auto_attack_enabled: bool,
    pub follow_distance: u32,
    pub attack_distance: u32,

    // 按钮状态
    pub close_button_pressed: bool,
    pub save_button_pressed: bool,
    pub grade_button_pressed: bool,
}

impl Default for IntelligentCreatureOptionsDialog {
    fn default() -> Self {
        Self {
            visible: false,
            x: 0,
            y: 0,
            width: 300,
            height: 250,
            auto_pickup_enabled: false,
            auto_revive_enabled: false,
            auto_defend_enabled: false,
            auto_attack_enabled: false,
            follow_distance: 3,
            attack_distance: 5,
            close_button_pressed: false,
            save_button_pressed: false,
            grade_button_pressed: false,
        }
    }
}

impl Dialog for IntelligentCreatureOptionsDialog {
    fn show(&mut self) {
        self.visible = true;
    }

    fn hide(&mut self) {
        self.visible = false;
    }

    fn is_visible(&self) -> bool {
        self.visible
    }

    fn update(&mut self, _delta_time: f32) {
        // 更新智能生物选项对话框逻辑
    }

    fn draw(&self) {
        // 绘制智能生物选项对话框
    }

    fn name(&self) -> &str {
        "IntelligentCreatureOptionsDialog"
    }

    fn contains_point(&self, x: i32, y: i32) -> bool {
        x >= self.x && x < self.x + self.width && y >= self.y && y < self.y + self.height
    }

    fn position(&self) -> (i32, i32) {
        (self.x, self.y)
    }

    fn size(&self) -> (i32, i32) {
        (self.width, self.height)
    }
}