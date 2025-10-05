// TrustMerchantDialog - 信任商人对话框
// Mirrors Client/MirScenes/Dialogs/TrustMerchantDialog.cs

use crate::scenes::dialogs::Dialog;
use mir2_shared::UserItem;

/// 信任商人对话框 - 管理寄售物品
pub struct TrustMerchantDialog {
    visible: bool,
    pub position: (i32, i32),
    pub size: (i32, i32),
    pub items: Vec<Option<UserItem>>,
    pub selected_item: Option<usize>,
    pub gold_amount: u64,
    pub sale_duration: u32, // 销售持续时间（小时）
}

impl TrustMerchantDialog {
    pub fn new() -> Self {
        Self {
            visible: false,
            position: (300, 200),
            size: (400, 500),
            items: vec![None; 10], // 10个寄售槽位
            selected_item: None,
            gold_amount: 0,
            sale_duration: 24, // 默认24小时
        }
    }

    pub fn add_item(&mut self, item: UserItem) -> bool {
        if let Some(slot) = self.items.iter().position(|slot| slot.is_none()) {
            self.items[slot] = Some(item);
            true
        } else {
            false
        }
    }

    pub fn remove_item(&mut self, slot: usize) -> Option<UserItem> {
        if slot < self.items.len() {
            self.items[slot].take()
        } else {
            None
        }
    }

    pub fn select_item(&mut self, slot: usize) {
        if slot < self.items.len() && self.items[slot].is_some() {
            self.selected_item = Some(slot);
        }
    }

    pub fn clear_selection(&mut self) {
        self.selected_item = None;
    }

    pub fn set_sale_price(&mut self, gold: u64) {
        self.gold_amount = gold;
    }

    pub fn set_duration(&mut self, hours: u32) {
        self.sale_duration = hours.clamp(1, 168); // 1小时到7天
    }
}

impl Dialog for TrustMerchantDialog {
    fn show(&mut self) {
        self.visible = true;
    }

    fn hide(&mut self) {
        self.visible = false;
        self.clear_selection();
    }

    fn update(&mut self, _delta_time: f32) {
        // Update item display and sale logic
    }

    fn draw(&self) {
        if !self.visible {
            return;
        }
        // Draw dialog background, item slots, price input, etc.
    }

    fn is_visible(&self) -> bool {
        self.visible
    }

    fn name(&self) -> &str { "TrustMerchantDialog" }
    fn contains_point(&self, x: i32, y: i32) -> bool {
        x >= self.position.0 && x < self.position.0 + self.size.0 &&
        y >= self.position.1 && y < self.position.1 + self.size.1
    }
    fn position(&self) -> (i32, i32) { self.position }
    fn size(&self) -> (i32, i32) { self.size }
}