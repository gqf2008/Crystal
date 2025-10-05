// QuestListDialog - 任务列表对话框
// 对应C#的QuestListDialog类

use crate::scenes::dialogs::Dialog;

/// Quest list dialog - 任务列表对话框
#[derive(Debug)]
pub struct QuestListDialog {
    pub visible: bool,
    pub x: i32,
    pub y: i32,
    pub width: i32,
    pub height: i32,

    // 任务数据
    pub quests: Vec<String>, // 暂时使用String，之后可以定义QuestProgress
    pub selected_index: usize,
    pub selected_quest: Option<String>, // 暂时使用String
    pub start_index: usize,
    pub current_npc_id: u32,

    // UI状态
    pub accept_button_pressed: bool,
    pub finish_button_pressed: bool,
    pub up_button_pressed: bool,
    pub down_button_pressed: bool,
}

impl Default for QuestListDialog {
    fn default() -> Self {
        Self {
            visible: false,
            x: 0,
            y: 0,
            width: 320,
            height: 480,
            quests: Vec::new(),
            selected_index: 0,
            selected_quest: None,
            start_index: 0,
            current_npc_id: 0,
            accept_button_pressed: false,
            finish_button_pressed: false,
            up_button_pressed: false,
            down_button_pressed: false,
        }
    }
}

impl Dialog for QuestListDialog {
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
        // 更新任务列表对话框逻辑
    }

    fn draw(&self) {
        // 绘制任务列表对话框
    }

    fn name(&self) -> &str {
        "QuestListDialog"
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