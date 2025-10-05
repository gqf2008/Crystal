// ItemRentalDialog - 物品租赁对话框
// 对应C#的ItemRentalDialog类

use crate::scenes::dialogs::Dialog;

/// Item rental dialog - 物品租赁对话框
#[derive(Debug)]
pub struct ItemRentalDialog {
    pub visible: bool,
    pub x: i32,
    pub y: i32,
    pub width: i32,
    pub height: i32,

    // 租赁物品列表
    pub rental_items: Vec<crate::scenes::dialogs::ItemRow>,
    pub selected_item_index: Option<usize>,

    // 租赁信息
    pub rental_period: u32, // 租赁时长（小时）
    pub rental_cost: u32,   // 租赁费用
    pub total_cost: u32,    // 总费用

    // UI状态
    pub rent_button_pressed: bool,
    pub cancel_button_pressed: bool,
    pub period_up_pressed: bool,
    pub period_down_pressed: bool,
}

impl Default for ItemRentalDialog {
    fn default() -> Self {
        Self {
            visible: false,
            x: 0,
            y: 0,
            width: 400,
            height: 300,
            rental_items: Vec::new(),
            selected_item_index: None,
            rental_period: 1,
            rental_cost: 0,
            total_cost: 0,
            rent_button_pressed: false,
            cancel_button_pressed: false,
            period_up_pressed: false,
            period_down_pressed: false,
        }
    }
}

impl Dialog for ItemRentalDialog {
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
        // 更新物品租赁对话框逻辑
    }

    fn draw(&self) {
        // 绘制物品租赁对话框
    }

    fn name(&self) -> &str {
        "ItemRentalDialog"
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