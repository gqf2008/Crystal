// QuestDetailDialog - 任务详情对话框
// 对应C#的QuestDetailDialog类

use crate::scenes::dialogs::Dialog;

/// Quest detail dialog - 任务详情对话框
#[derive(Debug)]
pub struct QuestDetailDialog {
    pub visible: bool,
    pub x: i32,
    pub y: i32,
    pub width: i32,
    pub height: i32,

    // 任务详情数据
    pub quest_title: String,
    pub quest_description: String,
    pub quest_objectives: Vec<String>,
    pub quest_rewards: Vec<String>,
}

impl Default for QuestDetailDialog {
    fn default() -> Self {
        Self {
            visible: false,
            x: 0,
            y: 0,
            width: 400,
            height: 300,
            quest_title: String::new(),
            quest_description: String::new(),
            quest_objectives: Vec::new(),
            quest_rewards: Vec::new(),
        }
    }
}

impl Dialog for QuestDetailDialog {
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
        // 更新任务详情对话框逻辑
    }

    fn draw(&self) {
        // 绘制任务详情对话框
    }

    fn name(&self) -> &str {
        "QuestDetailDialog"
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