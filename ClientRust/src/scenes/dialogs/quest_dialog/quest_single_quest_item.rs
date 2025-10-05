// QuestSingleQuestItem - 单个任务项控件
// 对应C#的QuestSingleQuestItem类

/// Quest single quest item - 单个任务项控件
#[derive(Debug)]
pub struct QuestSingleQuestItem {
    pub visible: bool,
    pub x: i32,
    pub y: i32,
    pub width: i32,
    pub height: i32,

    // 任务项数据
    pub quest_name: String,
    pub quest_description: String,
    pub is_completed: bool,
    pub is_selected: bool,
    pub track_quest: bool,
    pub quest: Option<String>, // 暂时使用String
}

impl Default for QuestSingleQuestItem {
    fn default() -> Self {
        Self {
            visible: true,
            x: 0,
            y: 0,
            width: 250,
            height: 40,
            quest_name: String::new(),
            quest_description: String::new(),
            is_completed: false,
            is_selected: false,
            track_quest: false,
            quest: None,
        }
    }
}