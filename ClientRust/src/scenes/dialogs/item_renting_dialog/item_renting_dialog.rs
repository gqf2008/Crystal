// ItemRentingDialog - 物品租赁中对话框
// 对应C#的ItemRentingDialog类

use crate::scenes::dialogs::Dialog;

/// Item renting dialog - 物品租赁中对话框
#[derive(Debug)]
pub struct ItemRentingDialog {
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

    // 租赁者信息
    pub renter_name: String,
    pub rental_start_time: u64,

    // UI状态
    pub extend_button_pressed: bool,
    pub cancel_button_pressed: bool,
    pub reclaim_button_pressed: bool,
}

impl Default for ItemRentingDialog {
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
            renter_name: String::new(),
            rental_start_time: 0,
            extend_button_pressed: false,
            cancel_button_pressed: false,
            reclaim_button_pressed: false,
        }
    }
}

impl Dialog for ItemRentingDialog {
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
        // 更新物品租赁中对话框逻辑
        if self.time_remaining > 0 {
            // 减少剩余时间
        }
    }

    fn draw(&self) {
        // 绘制物品租赁中对话框
    }

    fn name(&self) -> &str {
        "ItemRentingDialog"
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