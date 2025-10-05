// Fishing Dialog - 钓鱼系统对话框
// 管理钓鱼装备和显示钓鱼进度

use mir2_shared::UserItem;
use crate::scenes::dialogs::Dialog;

/// 钓鱼装备槽
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FishingSlot {
    Hook = 0,   // 鱼钩
    Float = 1,  // 浮标
    Bait = 2,   // 鱼饵
    Finder = 3, // 探鱼器
    Reel = 4,   // 渔线轮
}

/// 钓鱼对话框 - 装备管理
#[derive(Debug, Clone)]
pub struct FishingDialog {
    visible: bool,
    pub slots: [Option<UserItem>; 5],
    pub rod_name: String,
}

impl FishingDialog {
    pub fn new() -> Self {
        Self {
            visible: false,
            slots: Default::default(),
            rod_name: String::new(),
        }
    }

    pub fn show(&mut self, rod: &UserItem) {
        self.rod_name = rod.info.as_ref()
            .map(|info| info.name.clone())
            .unwrap_or_default();
        if rod.slots.len() >= 5 {
            for i in 0..5 {
                self.slots[i] = rod.slots[i].clone();
            }
        }
        self.visible = true;
    }

    pub fn hide(&mut self) {
        self.visible = false;
    }

    pub fn get_slot(&self, slot: FishingSlot) -> Option<&UserItem> {
        self.slots[slot as usize].as_ref()
    }

    pub fn set_slot(&mut self, slot: FishingSlot, item: Option<UserItem>) {
        self.slots[slot as usize] = item;
    }

    pub fn is_slot_empty(&self, slot: FishingSlot) -> bool {
        self.slots[slot as usize].is_none()
    }

    pub fn clear_slot(&mut self, slot: FishingSlot) {
        self.slots[slot as usize] = None;
    }

    pub fn has_all_equipment(&self) -> bool {
        self.slots.iter().all(|slot| slot.is_some())
    }

    pub fn get_equipment_count(&self) -> usize {
        self.slots.iter().filter(|slot| slot.is_some()).count()
    }
}

impl Dialog for FishingDialog {
    fn show(&mut self) {
        self.visible = true;
    }

    fn hide(&mut self) {
        self.visible = false;
    }

    fn update(&mut self, _delta_time: f32) {
        // Update logic would go here
    }

    fn draw(&self) {
        if !self.visible {
            return;
        }
        // Draw logic would go here
    }

    fn is_visible(&self) -> bool {
        self.visible
    }

    fn name(&self) -> &str { "FishingDialog" }
    fn contains_point(&self, x: i32, y: i32) -> bool { x >= 0 && x < 300 && y >= 0 && y < 200 }
    fn position(&self) -> (i32, i32) { (0, 0) }
    fn size(&self) -> (i32, i32) { (300, 200) }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_fishing_dialog() {
        let dialog = FishingDialog::new();
        assert!(!dialog.is_visible());
        assert_eq!(dialog.get_equipment_count(), 0);
    }
}