// QuestCell - 任务单元格控件
// 对应C#的QuestCell类

/// Quest cell - 任务单元格控件
#[derive(Debug)]
pub struct QuestCell {
    pub visible: bool,
    pub x: i32,
    pub y: i32,
    pub width: i32,
    pub height: i32,

    // 单元格数据
    pub item_id: Option<u32>,
    pub item_count: u32,
    pub is_selected: bool,
    pub cell_index: usize,
}

impl Default for QuestCell {
    fn default() -> Self {
        Self {
            visible: true,
            x: 0,
            y: 0,
            width: 36,
            height: 32,
            item_id: None,
            item_count: 0,
            is_selected: false,
            cell_index: 0,
        }
    }
}