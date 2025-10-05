// AuctionRow - 拍卖行控件
// Mirrors Client/MirScenes/Dialogs/TrustMerchantDialog.AuctionRow

use crate::scenes::dialogs::Dialog;
use mir2_shared::UserItem;

/// 拍卖行 - 显示单个拍卖物品的控件
pub struct AuctionRow {
    visible: bool,
    pub position: (i32, i32),
    pub size: (i32, i32),
    pub item: Option<UserItem>,
    pub seller_name: String,
    pub price: u64,
    pub time_remaining: u32, // 剩余时间（分钟）
    pub is_selected: bool,
}

impl AuctionRow {
    pub fn new() -> Self {
        Self {
            visible: true,
            position: (0, 0),
            size: (380, 40),
            item: None,
            seller_name: String::new(),
            price: 0,
            time_remaining: 0,
            is_selected: false,
        }
    }

    pub fn set_item(&mut self, item: UserItem, seller: String, price: u64, time: u32) {
        self.item = Some(item);
        self.seller_name = seller;
        self.price = price;
        self.time_remaining = time;
    }

    pub fn clear_item(&mut self) {
        self.item = None;
        self.seller_name.clear();
        self.price = 0;
        self.time_remaining = 0;
    }

    pub fn set_position(&mut self, x: i32, y: i32) {
        self.position = (x, y);
    }

    pub fn select(&mut self) {
        self.is_selected = true;
    }

    pub fn deselect(&mut self) {
        self.is_selected = false;
    }

    pub fn get_time_remaining_text(&self) -> String {
        if self.time_remaining == 0 {
            "Expired".to_string()
        } else if self.time_remaining < 60 {
            format!("{}m", self.time_remaining)
        } else {
            format!("{}h {}m", self.time_remaining / 60, self.time_remaining % 60)
        }
    }
}

impl Dialog for AuctionRow {
    fn show(&mut self) {
        self.visible = true;
    }

    fn hide(&mut self) {
        self.visible = false;
    }

    fn update(&mut self, delta_time: f32) {
        // Update time remaining
        if self.time_remaining > 0 {
            let delta_minutes = (delta_time / 60.0) as u32;
            self.time_remaining = self.time_remaining.saturating_sub(delta_minutes);
        }
    }

    fn draw(&self) {
        if !self.visible {
            return;
        }
        // Draw row background, item icon, seller name, price, time
    }

    fn is_visible(&self) -> bool {
        self.visible
    }

    fn name(&self) -> &str { "AuctionRow" }
    fn contains_point(&self, x: i32, y: i32) -> bool {
        x >= self.position.0 && x < self.position.0 + self.size.0 &&
        y >= self.position.1 && y < self.position.1 + self.size.1
    }
    fn position(&self) -> (i32, i32) { self.position }
    fn size(&self) -> (i32, i32) { self.size }
}