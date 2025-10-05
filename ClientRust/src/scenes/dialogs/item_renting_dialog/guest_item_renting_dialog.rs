// GuestItemRentingDialog - 访客物品租赁中对话框
// 对应C#的GuestItemRentingDialog类

use crate::scenes::dialogs::Dialog;

/// Guest item renting dialog - 访客物品租赁中对话框
#[derive(Debug)]
pub struct GuestItemRentingDialog {
    pub visible: bool,
    pub x: i32,
    pub y: i32,
    pub width: i32,
    pub height: i32,

    // 租赁物品信息
    pub item_id: Option<u32>,
    pub item_name: String,
    pub item_count: u32,
    pub rental_price: u32,
    pub rental_period: u32,
    pub time_remaining: u32, // 剩余时间（秒）

    // 出租者信息
    pub owner_name: String,
    pub rental_start_time: u64,

    // UI状态
    pub use_button_pressed: bool,
    pub return_button_pressed: bool,
    pub extend_button_pressed: bool,
}

impl Default for GuestItemRentingDialog {
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
            rental_price: 0,
            rental_period: 0,
            time_remaining: 0,
            owner_name: String::new(),
            rental_start_time: 0,
            use_button_pressed: false,
            return_button_pressed: false,
            extend_button_pressed: false,
        }
    }
}

impl Dialog for GuestItemRentingDialog {
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
        // 更新访客物品租赁中对话框逻辑
        if self.time_remaining > 0 {
            // 减少剩余时间
        }
    }

    fn draw(&self) {
        // 绘制访客物品租赁中对话框
    }

    fn name(&self) -> &str {
        "GuestItemRentingDialog"
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