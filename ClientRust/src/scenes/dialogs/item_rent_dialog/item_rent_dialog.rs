// ItemRentDialog - 物品出租对话框
// 对应C#的ItemRentDialog类

use crate::scenes::dialogs::Dialog;

/// Item rent dialog - 物品出租对话框
#[derive(Debug)]
pub struct ItemRentDialog {
    pub visible: bool,
    pub x: i32,
    pub y: i32,
    pub width: i32,
    pub height: i32,

    // 出租物品信息
    pub item_id: Option<u32>,
    pub item_name: String,
    pub item_count: u32,

    // 出租设置
    pub rental_price: u32,     // 出租价格
    pub rental_period: u32,    // 出租时长（小时）
    pub max_rental_period: u32,

    // UI状态
    pub rent_button_pressed: bool,
    pub cancel_button_pressed: bool,
    pub price_up_pressed: bool,
    pub price_down_pressed: bool,
    pub period_up_pressed: bool,
    pub period_down_pressed: bool,
}

impl Default for ItemRentDialog {
    fn default() -> Self {
        Self {
            visible: false,
            x: 0,
            y: 0,
            width: 300,
            height: 250,
            item_id: None,
            item_name: String::new(),
            item_count: 0,
            rental_price: 100,
            rental_period: 1,
            max_rental_period: 168, // 7天
            rent_button_pressed: false,
            cancel_button_pressed: false,
            price_up_pressed: false,
            price_down_pressed: false,
            period_up_pressed: false,
            period_down_pressed: false,
        }
    }
}

impl Dialog for ItemRentDialog {
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
        // 更新物品出租对话框逻辑
    }

    fn draw(&self) {
        // 绘制物品出租对话框
    }

    fn name(&self) -> &str {
        "ItemRentDialog"
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