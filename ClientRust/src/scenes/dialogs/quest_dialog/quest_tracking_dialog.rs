// QuestTrackingDialog - 任务跟踪对话框
// 对应C#的QuestTrackingDialog类

use crate::scenes::dialogs::Dialog;

/// Quest tracking dialog - 任务跟踪对话框
#[derive(Debug)]
pub struct QuestTrackingDialog {
    pub visible: bool,
    pub x: i32,
    pub y: i32,
    pub width: i32,
    pub height: i32,

    // 跟踪的任务
    pub tracked_quests: Vec<String>, // 暂时使用String
    pub max_tracked_quests: usize,
}

impl Default for QuestTrackingDialog {
    fn default() -> Self {
        Self {
            visible: false,
            x: 0,
            y: 0,
            width: 300,
            height: 200,
            tracked_quests: Vec::new(),
            max_tracked_quests: 5,
        }
    }
}

impl Dialog for QuestTrackingDialog {
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
        // 更新任务跟踪对话框逻辑
    }

    fn draw(&self) {
        // 绘制任务跟踪对话框
    }

    fn name(&self) -> &str {
        "QuestTrackingDialog"
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