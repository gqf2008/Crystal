// QuestRow - 任务行控件
// 对应C#的QuestRow类

/// Quest row - 任务行控件
#[derive(Debug)]
pub struct QuestRow {
    pub visible: bool,
    pub x: i32,
    pub y: i32,
    pub width: i32,
    pub height: i32,

    // 任务信息
    pub quest_name: String,
    pub quest_level: u16,
    pub quest_type: String,
    pub is_selected: bool,
    pub is_completed: bool,
}

impl Default for QuestRow {
    fn default() -> Self {
        Self {
            visible: true,
            x: 0,
            y: 0,
            width: 280,
            height: 30,
            quest_name: String::new(),
            quest_level: 0,
            quest_type: String::new(),
            is_selected: false,
            is_completed: false,
        }
    }
}