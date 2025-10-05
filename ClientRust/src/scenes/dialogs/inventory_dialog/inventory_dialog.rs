// InventoryDialog - Inventory/backpack UI
// Mirrors Client/MirScenes/Dialogs/InventoryDialog.cs

use crate::scenes::dialogs::Dialog;
use mir2_shared::UserItem;

/// Inventory tab type
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum InventoryTab {
    Inventory,  // 普通背包
    Equipment,  // 装备栏
    Quest,      // 任务物品
}

/// Inventory dialog
#[derive(Debug)]
pub struct InventoryDialog {
    visible: bool,
    pub x: i32,
    pub y: i32,
    pub width: i32,
    pub height: i32,
    pub current_tab: InventoryTab,

    // Inventory slots (基础46 + 扩展34 = 80 slots)
    pub inventory: Vec<Option<UserItem>>,

    // Equipment slots (14 slots)
    pub equipment: Vec<Option<UserItem>>,

    // Quest items (40 slots)
    pub quest_items: Vec<Option<UserItem>>,

    // Weight tracking
    pub current_weight: i32,
    pub max_weight: i32,

    // Gold amount
    pub gold: u64,

    // Extension system
    pub extended_slots: usize, // 扩展槽位数量 (0-34)
    pub lock_bars: [bool; 10], // 锁定栏显示状态
    pub add_button_visible: bool, // 添加按钮是否可见

    // Selected item
    pub selected_slot: Option<usize>,
    pub selected_tab: Option<InventoryTab>,
}

impl InventoryDialog {
    pub fn new() -> Self {
        Self {
            visible: false, // Start hidden
            x: 600,
            y: 100,
            width: 400,
            height: 500,
            current_tab: InventoryTab::Inventory,
            inventory: vec![None; 80], // 基础46 + 扩展34
            equipment: vec![None; 14],
            quest_items: vec![None; 40],
            current_weight: 0,
            max_weight: 100,
            gold: 0,
            extended_slots: 34, // 默认全扩展
            lock_bars: [false; 10], // 默认全解锁
            add_button_visible: false, // 默认隐藏
            selected_slot: None,
            selected_tab: None,
        }
    }

    /// Toggle inventory visibility
    pub fn toggle(&mut self) {
        self.visible = !self.visible;
    }

    /// Switch tab
    pub fn set_tab(&mut self, tab: InventoryTab) {
        self.current_tab = tab;
    }

    /// Get current slot list
    pub fn get_current_slots(&self) -> &Vec<Option<UserItem>> {
        match self.current_tab {
            InventoryTab::Inventory => &self.inventory,
            InventoryTab::Equipment => &self.equipment,
            InventoryTab::Quest => &self.quest_items,
        }
    }

    /// Get current slot list (mutable)
    pub fn get_current_slots_mut(&mut self) -> &mut Vec<Option<UserItem>> {
        match self.current_tab {
            InventoryTab::Inventory => &mut self.inventory,
            InventoryTab::Equipment => &mut self.equipment,
            InventoryTab::Quest => &mut self.quest_items,
        }
    }

    /// Select slot
    pub fn select_slot(&mut self, slot: usize) {
        let slots = self.get_current_slots();
        if slot < slots.len() {
            self.selected_slot = Some(slot);
            self.selected_tab = Some(self.current_tab);
        }
    }

    /// Clear selection
    pub fn clear_selection(&mut self) {
        self.selected_slot = None;
        self.selected_tab = None;
    }

    /// Use item in slot
    pub fn use_item(&mut self, slot: usize) {
        let slots = self.get_current_slots();
        if slot >= slots.len() {
            return;
        }

        if let Some(ref item) = slots[slot] {
            let name = item.info.as_ref().map(|i| i.name.as_str()).unwrap_or("Unknown");
            println!("Using item: {}", name);
            // TODO: Send use item packet to server
        }
    }

    /// Drop item from slot
    pub fn drop_item(&mut self, slot: usize, amount: u32) {
        let slots = self.get_current_slots();
        if slot >= slots.len() {
            return;
        }

        if let Some(ref item) = slots[slot] {
            let name = item.info.as_ref().map(|i| i.name.as_str()).unwrap_or("Unknown");
            println!("Dropping item: {} x{}", name, amount);
            // TODO: Send drop item packet to server
        }
    }

    /// Move item from one slot to another
    pub fn move_item(&mut self, from_slot: usize, to_slot: usize) {
        if from_slot == to_slot {
            return;
        }

        let slots = self.get_current_slots_mut();
        if from_slot >= slots.len() || to_slot >= slots.len() {
            return;
        }

        // Swap items
        slots.swap(from_slot, to_slot);
        println!("Moved item from slot {} to {}", from_slot, to_slot);

        // TODO: Send move item packet to server
    }

    /// Find empty slot
    pub fn find_empty_slot(&self) -> Option<usize> {
        self.get_current_slots()
            .iter()
            .position(|slot| slot.is_none())
    }

    /// Check if inventory is full
    pub fn is_full(&self) -> bool {
        self.find_empty_slot().is_none()
    }

    /// Get weight percentage
    pub fn get_weight_percent(&self) -> f32 {
        if self.max_weight == 0 {
            return 0.0;
        }
        (self.current_weight as f32 / self.max_weight as f32) * 100.0
    }

    /// Update weight
    pub fn update_weight(&mut self) {
        // Calculate total weight from inventory
        self.current_weight = self.inventory
            .iter()
            .filter_map(|item| item.as_ref())
            .map(|item| item.weight(item.info.as_ref()))
            .sum();
    }

    /// Update gold display
    pub fn update_gold(&mut self, gold: u64) {
        self.gold = gold;
    }

    /// Add inventory extension slots
    pub fn add_inventory_slots(&mut self) {
        if self.extended_slots < 34 {
            self.extended_slots = (self.extended_slots + 4).min(34);
            // Update lock bars based on extension level
            let open_level = self.extended_slots / 4;
            for i in 0..self.lock_bars.len() {
                self.lock_bars[i] = i >= open_level;
            }
            self.add_button_visible = open_level < 8; // 8 levels max (32 slots)
        }
    }

    /// Get extension cost for next level
    pub fn get_extension_cost(&self) -> u64 {
        let open_level = self.extended_slots / 4;
        1000000 + (open_level as u64) * 1000000
    }

    /// Check if slot is visible in current tab
    pub fn is_slot_visible(&self, slot: usize) -> bool {
        match self.current_tab {
            InventoryTab::Inventory => {
                if slot < 46 {
                    true // 基础46格总是可见
                } else if slot < 46 + self.extended_slots {
                    !self.lock_bars[(slot - 46) / 4] // 扩展格根据锁定状态
                } else {
                    false
                }
            }
            InventoryTab::Equipment => slot < self.equipment.len(),
            InventoryTab::Quest => slot < self.quest_items.len(),
        }
    }

    /// Get visible slots count for current tab
    pub fn get_visible_slots_count(&self) -> usize {
        match self.current_tab {
            InventoryTab::Inventory => 46 + self.extended_slots,
            InventoryTab::Equipment => self.equipment.len(),
            InventoryTab::Quest => self.quest_items.len(),
        }
    }

    /// Get empty slots count
    pub fn get_empty_slots_count(&self) -> usize {
        self.get_current_slots()
            .iter()
            .take(self.get_visible_slots_count())
            .filter(|slot| slot.is_none())
            .count()
    }
}

impl Default for InventoryDialog {
    fn default() -> Self {
        Self::new()
    }
}

impl Dialog for InventoryDialog {
    fn show(&mut self) {
        self.visible = true;
    }

    fn hide(&mut self) {
        self.visible = false;
        self.clear_selection();
    }

    fn update(&mut self, _delta_time: f32) {
        // TODO: Update item tooltips
        // TODO: Update weight display
    }

    fn draw(&self) {
        if !self.visible {
            return;
        }

        // TODO: Draw inventory background
        // TODO: Draw tab buttons
        // TODO: Draw item slots
        // TODO: Draw weight bar
        // TODO: Draw selected item highlight
    }

    fn is_visible(&self) -> bool {
        self.visible
    }

    fn name(&self) -> &str {
        "InventoryDialog"
    }

    fn contains_point(&self, x: i32, y: i32) -> bool {
        x >= self.x && x < self.x + self.width &&
        y >= self.y && y < self.y + self.height
    }

    fn position(&self) -> (i32, i32) {
        (self.x, self.y)
    }

    fn size(&self) -> (i32, i32) {
        (self.width, self.height)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_inventory_dialog_creation() {
        let dialog = InventoryDialog::new();
        assert!(!dialog.is_visible()); // Starts hidden
        assert_eq!(dialog.inventory.len(), 80); // 46 + 34 extended
        assert_eq!(dialog.equipment.len(), 14);
        assert_eq!(dialog.quest_items.len(), 40);
        assert_eq!(dialog.extended_slots, 34);
        assert_eq!(dialog.gold, 0);
    }

    #[test]
    fn test_tab_switching() {
        let mut dialog = InventoryDialog::new();

        assert_eq!(dialog.current_tab, InventoryTab::Inventory);

        dialog.set_tab(InventoryTab::Equipment);
        assert_eq!(dialog.current_tab, InventoryTab::Equipment);

        dialog.set_tab(InventoryTab::Quest);
        assert_eq!(dialog.current_tab, InventoryTab::Quest);
    }

    #[test]
    fn test_slot_selection() {
        let mut dialog = InventoryDialog::new();

        assert!(dialog.selected_slot.is_none());

        dialog.select_slot(5);
        assert_eq!(dialog.selected_slot, Some(5));
        assert_eq!(dialog.selected_tab, Some(InventoryTab::Inventory));

        dialog.clear_selection();
        assert!(dialog.selected_slot.is_none());
    }

    #[test]
    fn test_slot_visibility() {
        let mut dialog = InventoryDialog::new();

        // Inventory tab
        dialog.set_tab(InventoryTab::Inventory);
        assert!(dialog.is_slot_visible(0)); // 基础槽位可见
        assert!(dialog.is_slot_visible(45));
        assert!(dialog.is_slot_visible(46)); // 扩展槽位可见 (默认全扩展)
        assert!(dialog.is_slot_visible(79));
        assert!(!dialog.is_slot_visible(80)); // 超出范围

        // Equipment tab
        dialog.set_tab(InventoryTab::Equipment);
        assert!(dialog.is_slot_visible(0));
        assert!(dialog.is_slot_visible(13));
        assert!(!dialog.is_slot_visible(14));

        // Quest tab
        dialog.set_tab(InventoryTab::Quest);
        assert!(dialog.is_slot_visible(0));
        assert!(dialog.is_slot_visible(39));
        assert!(!dialog.is_slot_visible(40));
    }

    #[test]
    fn test_inventory_extension() {
        let mut dialog = InventoryDialog::new();

        // 初始状态: 全扩展
        assert_eq!(dialog.extended_slots, 34);
        assert_eq!(dialog.get_visible_slots_count(), 80);

        // 重置为无扩展状态测试
        dialog.extended_slots = 0;
        dialog.lock_bars = [true; 10];
        dialog.add_button_visible = true;

        // 添加扩展槽位
        dialog.add_inventory_slots();
        assert_eq!(dialog.extended_slots, 4);
        assert_eq!(dialog.get_visible_slots_count(), 50); // 46 + 4

        // 检查锁定栏状态
        assert!(dialog.lock_bars[0]); // 第一级仍锁定
        assert!(!dialog.lock_bars[1]); // 其他解锁?
        assert!(dialog.add_button_visible);
    }

    #[test]
    fn test_extension_cost() {
        let mut dialog = InventoryDialog::new();

        // 无扩展
        dialog.extended_slots = 0;
        assert_eq!(dialog.get_extension_cost(), 1000000);

        // 1级扩展 (4槽)
        dialog.extended_slots = 4;
        assert_eq!(dialog.get_extension_cost(), 2000000);

        // 8级扩展 (32槽)
        dialog.extended_slots = 32;
        assert_eq!(dialog.get_extension_cost(), 9000000);
    }

    #[test]
    fn test_gold_update() {
        let mut dialog = InventoryDialog::new();

        assert_eq!(dialog.gold, 0);

        dialog.update_gold(1234567);
        assert_eq!(dialog.gold, 1234567);
    }

    #[test]
    fn test_empty_slots_count() {
        let mut dialog = InventoryDialog::new();

        // 初始全空
        assert_eq!(dialog.get_empty_slots_count(), 80);

        // 添加一个物品
        dialog.inventory[0] = Some(UserItem::default());
        assert_eq!(dialog.get_empty_slots_count(), 79);
    }
}