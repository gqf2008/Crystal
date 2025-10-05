// TradeDialog - 交易对话框
// 对应C#的TradeDialog类

use crate::scenes::dialogs::Dialog;

/// Trade dialog - 交易对话框（主机方）
#[derive(Debug)]
pub struct TradeDialog {
    pub visible: bool,
    pub x: i32,
    pub y: i32,
    pub width: i32,
    pub height: i32,

    // 交易物品网格 (5x2)
    pub grid: Vec<Option<u32>>, // 物品ID数组，None表示空槽

    // 标签
    pub name_label: String,
    pub gold_label: String,

    // 按钮状态
    pub confirm_button_pressed: bool,
    pub close_button_pressed: bool,

    // 交易状态
    pub trade_locked: bool,
    pub trade_gold_amount: u32,
}

impl Default for TradeDialog {
    fn default() -> Self {
        Self {
            visible: false,
            x: 0,
            y: 0,
            width: 204,
            height: 152,
            grid: vec![None; 10], // 5x2网格
            name_label: String::new(),
            gold_label: String::new(),
            confirm_button_pressed: false,
            close_button_pressed: false,
            trade_locked: false,
            trade_gold_amount: 0,
        }
    }
}

impl Dialog for TradeDialog {
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
        // 更新交易对话框逻辑
    }

    fn draw(&self) {
        // 绘制交易对话框
    }

    fn name(&self) -> &str {
        "TradeDialog"
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