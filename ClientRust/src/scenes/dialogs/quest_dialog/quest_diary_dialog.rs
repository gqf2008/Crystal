// QuestDiaryDialog - 任务日志对话框
// 对应C#的QuestDiaryDialog类

use crate::scenes::dialogs::Dialog;

/// Quest diary dialog - 任务日志对话框
#[derive(Debug)]
pub struct QuestDiaryDialog {
    pub visible: bool,
    pub x: i32,
    pub y: i32,
    pub width: i32,
    pub height: i32,

    // 任务日志数据
    pub completed_quests: Vec<String>, // 暂时使用String
    pub active_quests: Vec<String>, // 暂时使用String
    pub selected_tab: usize, // 0=活跃任务, 1=完成任务
}

impl Default for QuestDiaryDialog {
    fn default() -> Self {
        Self {
            visible: false,
            x: 0,
            y: 0,
            width: 400,
            height: 500,
            completed_quests: Vec::new(),
            active_quests: Vec::new(),
            selected_tab: 0,
        }
    }
}

impl Dialog for QuestDiaryDialog {
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
        // 更新任务日志对话框逻辑
    }

    fn draw(&self) {
        // 绘制任务日志对话框
    }

    fn name(&self) -> &str {
        "QuestDiaryDialog"
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