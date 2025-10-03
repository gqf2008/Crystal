// InventoryDialog - Inventory/backpack UI
// Mirrors Client/MirScenes/Dialogs/InventoryDialog.cs

use super::Dialog;
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
    pub visible: bool,
    pub x: i32,
    pub y: i32,
    pub width: i32,
    pub height: i32,
    pub current_tab: InventoryTab,
    
    // Inventory slots (46 slots)
    pub inventory: Vec<Option<UserItem>>,
    
    // Equipment slots (14 slots)
    pub equipment: Vec<Option<UserItem>>,
    
    // Quest items (40 slots)
    pub quest_items: Vec<Option<UserItem>>,
    
    // Weight tracking
    pub current_weight: i32,
    pub max_weight: i32,
    
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
            inventory: vec![None; 46],
            equipment: vec![None; 14],
            quest_items: vec![None; 40],
            current_weight: 0,
            max_weight: 100,
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
        assert!(!dialog.visible); // Starts hidden
        assert_eq!(dialog.inventory.len(), 46);
        assert_eq!(dialog.equipment.len(), 14);
        assert_eq!(dialog.quest_items.len(), 40);
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
    fn test_find_empty_slot() {
        let dialog = InventoryDialog::new();
        
        // All slots empty initially
        assert_eq!(dialog.find_empty_slot(), Some(0));
        assert!(dialog.is_full() == false);
    }

    #[test]
    fn test_toggle() {
        let mut dialog = InventoryDialog::new();
        
        assert!(!dialog.visible);
        
        dialog.toggle();
        assert!(dialog.visible);
        
        dialog.toggle();
        assert!(!dialog.visible);
    }
}
