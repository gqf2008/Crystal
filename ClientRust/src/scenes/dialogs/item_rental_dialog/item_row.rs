// ItemRow - 物品行控件
// 对应C#的ItemRow类

/// Item row - 物品行控件
#[derive(Debug)]
pub struct ItemRow {
    pub visible: bool,
    pub x: i32,
    pub y: i32,
    pub width: i32,
    pub height: i32,

    // 物品信息
    pub item_id: Option<u32>,
    pub item_name: String,
    pub item_count: u32,
    pub item_level: u16,
    pub item_type: String,

    // 租赁信息
    pub rental_price: u32,     // 租赁价格
    pub rental_period: u32,    // 租赁时长
    pub is_available: bool,    // 是否可租赁

    // 行状态
    pub is_selected: bool,
    pub row_index: usize,
}

impl Default for ItemRow {
    fn default() -> Self {
        Self {
            visible: true,
            x: 0,
            y: 0,
            width: 350,
            height: 30,
            item_id: None,
            item_name: String::new(),
            item_count: 0,
            item_level: 0,
            item_type: String::new(),
            rental_price: 0,
            rental_period: 1,
            is_available: true,
            is_selected: false,
            row_index: 0,
        }
    }
}