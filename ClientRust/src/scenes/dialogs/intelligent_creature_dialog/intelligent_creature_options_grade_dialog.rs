// IntelligentCreatureOptionsGradeDialog - 智能生物选项等级对话框
// 对应C#的IntelligentCreatureOptionsGradeDialog类

use crate::scenes::dialogs::Dialog;

/// Intelligent creature options grade dialog - 智能生物选项等级对话框
#[derive(Debug)]
pub struct IntelligentCreatureOptionsGradeDialog {
    pub visible: bool,
    pub x: i32,
    pub y: i32,
    pub width: i32,
    pub height: i32,

    // 等级信息
    pub current_grade: u32,
    pub max_grade: u32,
    pub upgrade_cost: u32,
    pub upgrade_success_rate: f32,

    // 按钮状态
    pub close_button_pressed: bool,
    pub upgrade_button_pressed: bool,
    pub cancel_button_pressed: bool,
}

impl Default for IntelligentCreatureOptionsGradeDialog {
    fn default() -> Self {
        Self {
            visible: false,
            x: 0,
            y: 0,
            width: 250,
            height: 200,
            current_grade: 1,
            max_grade: 10,
            upgrade_cost: 0,
            upgrade_success_rate: 0.0,
            close_button_pressed: false,
            upgrade_button_pressed: false,
            cancel_button_pressed: false,
        }
    }
}

impl Dialog for IntelligentCreatureOptionsGradeDialog {
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
        // 更新智能生物选项等级对话框逻辑
    }

    fn draw(&self) {
        // 绘制智能生物选项等级对话框
    }

    fn name(&self) -> &str {
        "IntelligentCreatureOptionsGradeDialog"
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