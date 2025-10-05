// GuestTradeDialog - 访客交易对话框
// 对应C#的GuestTradeDialog类

use crate::scenes::dialogs::Dialog;

/// Guest trade dialog - 访客交易对话框
#[derive(Debug)]
pub struct GuestTradeDialog {
    pub visible: bool,
    pub x: i32,
    pub y: i32,
    pub width: i32,
    pub height: i32,

    // 访客交易物品网格 (5x2)
    pub guest_grid: Vec<Option<u32>>, // 物品ID数组，None表示空槽

    // 标签
    pub guest_name_label: String,
    pub guest_gold_label: String,

    // 按钮状态
    pub confirm_button_pressed: bool,

    // 访客交易数据
    pub guest_name: String,
    pub guest_gold: u32,
    pub guest_items: Vec<Option<u32>>, // 访客物品数组
}

impl Default for GuestTradeDialog {
    fn default() -> Self {
        Self {
            visible: false,
            x: 0,
            y: 0,
            width: 204,
            height: 152,
            guest_grid: vec![None; 10], // 5x2网格
            guest_name_label: String::new(),
            guest_gold_label: String::new(),
            confirm_button_pressed: false,
            guest_name: String::new(),
            guest_gold: 0,
            guest_items: vec![None; 10], // 5x2网格
        }
    }
}

impl Dialog for GuestTradeDialog {
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
        // 更新访客交易对话框逻辑
    }

    fn draw(&self) {
        // 绘制访客交易对话框
    }

    fn name(&self) -> &str {
        "GuestTradeDialog"
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