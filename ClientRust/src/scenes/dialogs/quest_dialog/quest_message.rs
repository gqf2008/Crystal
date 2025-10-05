// QuestMessage - 任务消息控件
// 对应C#的QuestMessage类

/// Quest message - 任务消息控件
#[derive(Debug)]
pub struct QuestMessage {
    pub visible: bool,
    pub x: i32,
    pub y: i32,
    pub width: i32,
    pub height: i32,

    // 消息内容
    pub message_text: String,
    pub scroll_position: usize,
    pub max_lines: usize,

    // 滚动按钮状态
    pub up_button_pressed: bool,
    pub down_button_pressed: bool,
}

impl Default for QuestMessage {
    fn default() -> Self {
        Self {
            visible: true,
            x: 0,
            y: 0,
            width: 280,
            height: 150,
            message_text: String::new(),
            scroll_position: 0,
            max_lines: 10,
            up_button_pressed: false,
            down_button_pressed: false,
        }
    }
}