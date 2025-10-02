// Storage Dialog - 仓库对话框
// 显示玩家的仓库物品 (80个普通槽位 + 80个扩展槽位)

use super::Dialog;
use crate::network::network::protocol::UserItem;

/// 仓库类型
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum StorageType {
    Storage1,  // 仓库1 (默认80个槽位)
    Storage2,  // 仓库2 (需要租赁，额外80个槽位)
}

/// 仓库对话框
pub struct StorageDialog {
    visible: bool,
    current_storage: StorageType,

    // 仓库槽位
    pub storage1: Vec<Option<UserItem>>, // 80个槽位
    pub storage2: Vec<Option<UserItem>>, // 80个扩展槽位

    // 扩展仓库状态
    pub has_expanded_storage: bool, // 是否已租赁扩展仓库
    pub rental_expiry: Option<i64>, // 租赁到期时间(Unix时间戳)

    // 选中的槽位
    pub selected_slot: Option<usize>,
    pub selected_item: Option<UserItem>,

    // 保护模式
    pub protect_mode: bool, // 是否开启保护模式
}

impl StorageDialog {
    /// 创建新的仓库对话框
    pub fn new() -> Self {
        Self {
            visible: false,
            current_storage: StorageType::Storage1,
            storage1: vec![None; 80],
            storage2: vec![None; 80],
            has_expanded_storage: false,
            rental_expiry: None,
            selected_slot: None,
            selected_item: None,
            protect_mode: false,
        }
    }

    /// 切换仓库
    pub fn switch_storage(&mut self, storage_type: StorageType) {
        self.current_storage = storage_type;
        self.selected_slot = None;
        self.selected_item = None;
    }

    /// 获取当前仓库
    pub fn get_current_storage(&self) -> StorageType {
        self.current_storage
    }

    /// 获取当前仓库的槽位数组
    fn get_current_slots(&self) -> &[Option<UserItem>] {
        match self.current_storage {
            StorageType::Storage1 => &self.storage1,
            StorageType::Storage2 => &self.storage2,
        }
    }

    /// 获取当前仓库的槽位数组(可变)
    fn get_current_slots_mut(&mut self) -> &mut [Option<UserItem>] {
        match self.current_storage {
            StorageType::Storage1 => &mut self.storage1,
            StorageType::Storage2 => &mut self.storage2,
        }
    }

    /// 设置物品到槽位
    pub fn set_item(&mut self, slot: usize, item: Option<UserItem>) {
        let slots = self.get_current_slots_mut();
        if slot < slots.len() {
            slots[slot] = item;
        }
    }

    /// 获取槽位中的物品
    pub fn get_item(&self, slot: usize) -> Option<&UserItem> {
        self.get_current_slots().get(slot)?.as_ref()
    }

    /// 选中槽位
    pub fn select_slot(&mut self, slot: usize) {
        if slot < self.get_current_slots().len() {
            self.selected_slot = Some(slot);
            self.selected_item = self.get_current_slots()[slot].clone();
        }
    }

    /// 取消选中
    pub fn deselect(&mut self) {
        self.selected_slot = None;
        self.selected_item = None;
    }

    /// 查找空槽位
    pub fn find_empty_slot(&self) -> Option<usize> {
        self.get_current_slots()
            .iter()
            .position(|slot| slot.is_none())
    }

    /// 检查仓库是否已满
    pub fn is_full(&self) -> bool {
        self.find_empty_slot().is_none()
    }

    /// 移动物品
    pub fn move_item(&mut self, from: usize, to: usize) -> bool {
        let slots = self.get_current_slots_mut();
        if from < slots.len() && to < slots.len() {
            // 交换物品
            let temp = slots[from].take();
            slots[from] = slots[to].take();
            slots[to] = temp;
            return true;
        }
        false
    }

    /// 存入物品 (从背包到仓库)
    pub fn store_item(&mut self, item: UserItem) -> bool {
        if let Some(empty_slot) = self.find_empty_slot() {
            self.set_item(empty_slot, Some(item));
            true
        } else {
            false
        }
    }

    /// 取出物品 (从仓库到背包)
    pub fn retrieve_item(&mut self, slot: usize) -> Option<UserItem> {
        let slots = self.get_current_slots_mut();
        if slot < slots.len() {
            slots[slot].take()
        } else {
            None
        }
    }

    /// 清空仓库
    pub fn clear_storage(&mut self, storage_type: StorageType) {
        match storage_type {
            StorageType::Storage1 => {
                self.storage1.iter_mut().for_each(|slot| *slot = None);
            }
            StorageType::Storage2 => {
                self.storage2.iter_mut().for_each(|slot| *slot = None);
            }
        }
    }

    /// 清空所有仓库
    pub fn clear_all(&mut self) {
        self.clear_storage(StorageType::Storage1);
        self.clear_storage(StorageType::Storage2);
    }

    /// 启用扩展仓库
    pub fn enable_expanded_storage(&mut self, expiry_time: Option<i64>) {
        self.has_expanded_storage = true;
        self.rental_expiry = expiry_time;
    }

    /// 禁用扩展仓库
    pub fn disable_expanded_storage(&mut self) {
        self.has_expanded_storage = false;
        self.rental_expiry = None;
        self.clear_storage(StorageType::Storage2);
        if self.current_storage == StorageType::Storage2 {
            self.switch_storage(StorageType::Storage1);
        }
    }

    /// 检查扩展仓库是否已过期
    pub fn is_expanded_storage_expired(&self, current_time: i64) -> bool {
        if !self.has_expanded_storage {
            return false;
        }
        if let Some(expiry) = self.rental_expiry {
            current_time > expiry
        } else {
            false
        }
    }

    /// 获取扩展仓库剩余时间(秒)
    pub fn get_rental_time_remaining(&self, current_time: i64) -> Option<i64> {
        if let Some(expiry) = self.rental_expiry {
            Some((expiry - current_time).max(0))
        } else {
            None
        }
    }

    /// 切换保护模式
    pub fn toggle_protect_mode(&mut self) {
        self.protect_mode = !self.protect_mode;
    }

    /// 统计仓库中的物品数量
    pub fn count_items(&self) -> usize {
        let count1 = self.storage1.iter().filter(|slot| slot.is_some()).count();
        let count2 = if self.has_expanded_storage {
            self.storage2.iter().filter(|slot| slot.is_some()).count()
        } else {
            0
        };
        count1 + count2
    }

    /// 统计总槽位数量
    pub fn total_slots(&self) -> usize {
        if self.has_expanded_storage {
            160 // 80 + 80
        } else {
            80
        }
    }

    /// 统计空槽位数量
    pub fn count_empty_slots(&self) -> usize {
        self.total_slots() - self.count_items()
    }
}

impl Default for StorageDialog {
    fn default() -> Self {
        Self::new()
    }
}

impl Dialog for StorageDialog {
    fn show(&mut self) {
        self.visible = true;
        // 默认显示Storage1
        self.switch_storage(StorageType::Storage1);
    }

    fn hide(&mut self) {
        self.visible = false;
        self.deselect();
    }

    fn update(&mut self, _delta_time: f32) {
        // 更新逻辑 (如检查租赁到期等)
    }

    fn draw(&self) {
        if !self.visible {
            return;
        }
        // TODO: 实际渲染逻辑
        // 绘制仓库对话框背景、物品格子、标签页按钮等
    }

    fn is_visible(&self) -> bool {
        self.visible
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_storage_dialog_creation() {
        let dialog = StorageDialog::new();
        assert!(!dialog.is_visible());
        assert_eq!(dialog.storage1.len(), 80);
        assert_eq!(dialog.storage2.len(), 80);
        assert!(!dialog.has_expanded_storage);
    }

    #[test]
    fn test_storage_switch() {
        let mut dialog = StorageDialog::new();
        assert_eq!(dialog.get_current_storage(), StorageType::Storage1);
        
        dialog.switch_storage(StorageType::Storage2);
        assert_eq!(dialog.get_current_storage(), StorageType::Storage2);
    }

    #[test]
    fn test_storage_set_get_item() {
        let mut dialog = StorageDialog::new();
        
        let item = UserItem {
            unique_id: 1001,
            item_index: 42,
            current_dura: 1000,
            max_dura: 1000,
            count: 1,
            ..Default::default()
        };
        
        dialog.set_item(0, Some(item.clone()));
        
        let stored = dialog.get_item(0);
        assert!(stored.is_some());
        assert_eq!(stored.unwrap().unique_id, 1001);
    }

    #[test]
    fn test_storage_find_empty_slot() {
        let mut dialog = StorageDialog::new();
        
        let item = UserItem::default();
        for i in 0..5 {
            dialog.set_item(i, Some(item.clone()));
        }
        
        let empty = dialog.find_empty_slot();
        assert_eq!(empty, Some(5));
    }

    #[test]
    fn test_storage_is_full() {
        let mut dialog = StorageDialog::new();
        assert!(!dialog.is_full());
        
        let item = UserItem::default();
        for i in 0..80 {
            dialog.set_item(i, Some(item.clone()));
        }
        
        assert!(dialog.is_full());
    }

    #[test]
    fn test_storage_store_retrieve() {
        let mut dialog = StorageDialog::new();
        
        let item = UserItem {
            unique_id: 2001,
            item_index: 55,
            current_dura: 500,
            max_dura: 1000,
            count: 10,
            ..Default::default()
        };
        
        // 存入物品
        let success = dialog.store_item(item.clone());
        assert!(success);
        assert_eq!(dialog.count_items(), 1);
        
        // 取出物品
        let retrieved = dialog.retrieve_item(0);
        assert!(retrieved.is_some());
        assert_eq!(retrieved.unwrap().unique_id, 2001);
        assert_eq!(dialog.count_items(), 0);
    }

    #[test]
    fn test_storage_move_item() {
        let mut dialog = StorageDialog::new();
        
        let item1 = UserItem { unique_id: 1001, ..Default::default() };
        let item2 = UserItem { unique_id: 2002, ..Default::default() };
        
        dialog.set_item(0, Some(item1));
        dialog.set_item(5, Some(item2));
        
        // 移动物品
        dialog.move_item(0, 10);
        
        assert!(dialog.get_item(0).is_none());
        assert!(dialog.get_item(10).is_some());
        assert_eq!(dialog.get_item(10).unwrap().unique_id, 1001);
    }

    #[test]
    fn test_storage_select_slot() {
        let mut dialog = StorageDialog::new();
        
        let item = UserItem { unique_id: 3001, ..Default::default() };
        dialog.set_item(5, Some(item.clone()));
        
        dialog.select_slot(5);
        assert_eq!(dialog.selected_slot, Some(5));
        assert!(dialog.selected_item.is_some());
        assert_eq!(dialog.selected_item.as_ref().unwrap().unique_id, 3001);
        
        dialog.deselect();
        assert!(dialog.selected_slot.is_none());
        assert!(dialog.selected_item.is_none());
    }

    #[test]
    fn test_storage_clear() {
        let mut dialog = StorageDialog::new();
        
        let item = UserItem::default();
        for i in 0..10 {
            dialog.set_item(i, Some(item.clone()));
        }
        
        assert_eq!(dialog.count_items(), 10);
        
        dialog.clear_storage(StorageType::Storage1);
        assert_eq!(dialog.count_items(), 0);
    }

    #[test]
    fn test_expanded_storage() {
        let mut dialog = StorageDialog::new();
        
        assert!(!dialog.has_expanded_storage);
        assert_eq!(dialog.total_slots(), 80);
        
        // 启用扩展仓库
        dialog.enable_expanded_storage(Some(1000000));
        assert!(dialog.has_expanded_storage);
        assert_eq!(dialog.total_slots(), 160);
        
        // 禁用扩展仓库
        dialog.disable_expanded_storage();
        assert!(!dialog.has_expanded_storage);
        assert_eq!(dialog.total_slots(), 80);
    }

    #[test]
    fn test_rental_expiry() {
        let mut dialog = StorageDialog::new();
        
        let expiry_time = 1000000;
        dialog.enable_expanded_storage(Some(expiry_time));
        
        // 未过期
        assert!(!dialog.is_expanded_storage_expired(999000));
        
        // 已过期
        assert!(dialog.is_expanded_storage_expired(1000001));
        
        // 剩余时间
        let remaining = dialog.get_rental_time_remaining(999500);
        assert_eq!(remaining, Some(500));
    }

    #[test]
    fn test_storage_counting() {
        let mut dialog = StorageDialog::new();
        
        let item = UserItem::default();
        
        // Storage1: 5个物品
        for i in 0..5 {
            dialog.storage1[i] = Some(item.clone());
        }
        
        assert_eq!(dialog.count_items(), 5);
        assert_eq!(dialog.count_empty_slots(), 75);
        
        // 启用扩展仓库
        dialog.enable_expanded_storage(None);
        
        // Storage2: 3个物品
        for i in 0..3 {
            dialog.storage2[i] = Some(item.clone());
        }
        
        assert_eq!(dialog.count_items(), 8);
        assert_eq!(dialog.count_empty_slots(), 152); // 160 - 8
    }

    #[test]
    fn test_protect_mode() {
        let mut dialog = StorageDialog::new();
        assert!(!dialog.protect_mode);
        
        dialog.toggle_protect_mode();
        assert!(dialog.protect_mode);
        
        dialog.toggle_protect_mode();
        assert!(!dialog.protect_mode);
    }
}
