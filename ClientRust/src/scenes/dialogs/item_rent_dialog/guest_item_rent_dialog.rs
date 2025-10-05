// GuestItemRentDialog - 访客物品出租对话框
// 对应C#的GuestItemRentDialog类

use crate::scenes::dialogs::Dialog;

/// Guest item rent dialog - 访客物品出租对话框
#[derive(Debug)]
pub struct GuestItemRentDialog {
    pub visible: bool,
    pub x: i32,
    pub y: i32,
    pub width: i32,
    pub height: i32,

    // 出租物品信息
    pub item_id: Option<u32>,
    pub item_name: String,
    pub item_count: u32,
    pub seller_name: String,

    // 出租设置
    pub rental_price: u32,     // 出租价格
    pub rental_period: u32,    // 出租时长（小时）

    // UI状态
    pub rent_button_pressed: bool,
    pub cancel_button_pressed: bool,
    pub negotiate_button_pressed: bool,
}

impl Default for GuestItemRentDialog {
    fn default() -> Self {
        Self {
            visible: false,
            x: 0,
            y: 0,
            width: 300,
            height: 200,
            item_id: None,
            item_name: String::new(),
            item_count: 0,
            seller_name: String::new(),
            rental_price: 0,
            rental_period: 0,
            rent_button_pressed: false,
            cancel_button_pressed: false,
            negotiate_button_pressed: false,
        }
    }
}

impl Dialog for GuestItemRentDialog {
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
        // 更新访客物品出租对话框逻辑
    }

    fn draw(&self) {
        // 绘制访客物品出租对话框
    }

    fn name(&self) -> &str {
        "GuestItemRentDialog"
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