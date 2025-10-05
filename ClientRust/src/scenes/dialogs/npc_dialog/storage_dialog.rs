// Storage Dialog - 仓库对话框
// 显示玩家的仓库物品 (10x16网格布局，总共160个槽位)

use crate::scenes::dialogs::Dialog;
use crate::network::protocol::UserItem;

/// 仓库类型
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum StorageType {
    Storage1,  // 仓库1 (基础存储)
    Storage2,  // 仓库2 (扩展存储，需要租赁)
}

/// 仓库对话框
pub struct StorageDialog {
    visible: bool,
    x: i32,
    y: i32,
    width: i32,
    height: i32,
    current_storage: StorageType,

    // 仓库网格 (10x16 = 160个槽位)
    pub grid: Vec<Option<UserItem>>,

    // UI状态
    pub storage1_selected: bool,  // Storage1按钮选中状态
    pub storage2_selected: bool,  // Storage2按钮选中状态
    pub rent_button_visible: bool, // 租赁按钮可见性
    pub protect_button_visible: bool, // 保护按钮可见性
    pub locked_page_visible: bool, // 锁定页面可见性

    // 扩展仓库状态
    pub has_expanded_storage: bool, // 是否已租赁扩展仓库
    pub rental_expiry: Option<i64>, // 租赁到期时间(Unix时间戳)
    pub rental_label_text: String, // 租赁状态文本
    pub rental_label_color: (u8, u8, u8), // 租赁标签颜色 (RGB)

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
            x: 100,
            y: 100,
            width: 450,
            height: 550,
            current_storage: StorageType::Storage1,
            grid: vec![None; 160], // 10x16网格
            storage1_selected: true,
            storage2_selected: false,
            rent_button_visible: false,
            protect_button_visible: true,
            locked_page_visible: false,
            has_expanded_storage: false,
            rental_expiry: None,
            rental_label_text: "扩展仓库已锁定".to_string(),
            rental_label_color: (255, 0, 0), // 红色
            selected_slot: None,
            selected_item: None,
            protect_mode: false,
        }
    }

    /// 切换到仓库1
    pub fn refresh_storage1(&mut self) {
        self.current_storage = StorageType::Storage1;
        self.storage1_selected = true;
        self.storage2_selected = false;
        self.rent_button_visible = false;
        self.locked_page_visible = false;
        // 租赁标签不可见
    }

    /// 切换到仓库2 (扩展存储)
    pub fn refresh_storage2(&mut self) {
        self.current_storage = StorageType::Storage2;
        self.storage1_selected = false;
        self.storage2_selected = true;

        if self.has_expanded_storage {
            self.rent_button_visible = true;
            self.locked_page_visible = false;
            self.rental_label_text = format!("扩展仓库到期时间: {}",
                self.rental_expiry.map_or("未知".to_string(), |t| t.to_string()));
            self.rental_label_color = (255, 255, 255); // 白色
        } else {
            self.rental_label_text = "扩展仓库已锁定".to_string();
            self.rental_label_color = (255, 0, 0); // 红色
            self.rent_button_visible = true;
            self.locked_page_visible = true;
        }
    }

    /// 获取当前仓库类型
    pub fn get_current_storage(&self) -> StorageType {
        self.current_storage
    }

    /// 获取网格中的物品 (通过槽位索引)
    pub fn get_item(&self, slot: usize) -> Option<&UserItem> {
        self.grid.get(slot)?.as_ref()
    }

    /// 设置网格中的物品
    pub fn set_item(&mut self, slot: usize, item: Option<UserItem>) {
        if slot < self.grid.len() {
            self.grid[slot] = item;
        }
    }

    /// 通过物品ID获取单元格
    pub fn get_cell(&self, unique_id: u64) -> Option<usize> {
        self.grid.iter().position(|item| {
            item.as_ref().map_or(false, |i| i.unique_id == unique_id)
        })
    }

    /// 选中槽位
    pub fn select_slot(&mut self, slot: usize) {
        if slot < self.grid.len() {
            self.selected_slot = Some(slot);
            self.selected_item = self.grid[slot].clone();
        }
    }

    /// 取消选中
    pub fn deselect(&mut self) {
        self.selected_slot = None;
        self.selected_item = None;
    }

    /// 查找空槽位
    pub fn find_empty_slot(&self) -> Option<usize> {
        self.grid.iter().position(|slot| slot.is_none())
    }

    /// 检查仓库是否已满
    pub fn is_full(&self) -> bool {
        self.find_empty_slot().is_none()
    }

    /// 移动物品
    pub fn move_item(&mut self, from: usize, to: usize) -> bool {
        if from < self.grid.len() && to < self.grid.len() {
            // 交换物品
            let temp = self.grid[from].take();
            self.grid[from] = self.grid[to].take();
            self.grid[to] = temp;
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
        if slot < self.grid.len() {
            self.grid[slot].take()
        } else {
            None
        }
    }

    /// 清空仓库
    pub fn clear_storage(&mut self) {
        self.grid.iter_mut().for_each(|slot| *slot = None);
    }

    /// 启用扩展仓库
    pub fn enable_expanded_storage(&mut self, expiry_time: Option<i64>) {
        self.has_expanded_storage = true;
        self.rental_expiry = expiry_time;
        self.refresh_storage2();
    }

    /// 禁用扩展仓库
    pub fn disable_expanded_storage(&mut self) {
        self.has_expanded_storage = false;
        self.rental_expiry = None;
        // 清空扩展存储区域 (假设后80个槽位是扩展存储)
        for i in 80..160 {
            self.grid[i] = None;
        }
        if self.current_storage == StorageType::Storage2 {
            self.refresh_storage1();
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
        self.grid.iter().filter(|slot| slot.is_some()).count()
    }

    /// 统计总槽位数量
    pub fn total_slots(&self) -> usize {
        self.grid.len()
    }

    /// 统计空槽位数量
    pub fn count_empty_slots(&self) -> usize {
        self.total_slots() - self.count_items()
    }

    /// 获取网格位置 (将槽位索引转换为x,y坐标)
    pub fn get_grid_position(&self, slot: usize) -> Option<(i32, i32)> {
        if slot >= self.grid.len() {
            return None;
        }
        let x = (slot % 10) as i32;
        let y = (slot / 10) as i32;
        Some((x, y))
    }

    /// 获取槽位索引 (将x,y坐标转换为槽位索引)
    pub fn get_slot_index(&self, x: i32, y: i32) -> Option<usize> {
        if x >= 0 && x < 10 && y >= 0 && y < 16 {
            Some((y * 10 + x) as usize)
        } else {
            None
        }
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
        self.refresh_storage1();
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

    fn name(&self) -> &str {
        "StorageDialog"
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
    fn test_storage_dialog_creation() {
        let dialog = StorageDialog::new();
        assert!(!dialog.is_visible());
        assert_eq!(dialog.grid.len(), 160);
        assert!(!dialog.has_expanded_storage);
    }

    #[test]
    fn test_storage_switch() {
        let mut dialog = StorageDialog::new();
        assert_eq!(dialog.get_current_storage(), StorageType::Storage1);

        dialog.refresh_storage2();
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
        for i in 0..160 {
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

        dialog.clear_storage();
        assert_eq!(dialog.count_items(), 0);
    }

    #[test]
    fn test_expanded_storage() {
        let mut dialog = StorageDialog::new();

        assert!(!dialog.has_expanded_storage);
        assert_eq!(dialog.total_slots(), 160);

        // 启用扩展仓库
        dialog.enable_expanded_storage(Some(1000000));
        assert!(dialog.has_expanded_storage);

        // 禁用扩展仓库
        dialog.disable_expanded_storage();
        assert!(!dialog.has_expanded_storage);
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

        // 添加一些物品
        for i in 0..8 {
            dialog.grid[i] = Some(item.clone());
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

    #[test]
    fn test_grid_position_conversion() {
        let dialog = StorageDialog::new();

        // 测试位置转换
        assert_eq!(dialog.get_grid_position(0), Some((0, 0)));
        assert_eq!(dialog.get_grid_position(9), Some((9, 0)));
        assert_eq!(dialog.get_grid_position(10), Some((0, 1)));
        assert_eq!(dialog.get_grid_position(159), Some((9, 15)));
        assert_eq!(dialog.get_grid_position(160), None);

        // 测试索引转换
        assert_eq!(dialog.get_slot_index(0, 0), Some(0));
        assert_eq!(dialog.get_slot_index(9, 0), Some(9));
        assert_eq!(dialog.get_slot_index(0, 1), Some(10));
        assert_eq!(dialog.get_slot_index(9, 15), Some(159));
        assert_eq!(dialog.get_slot_index(10, 0), None);
        assert_eq!(dialog.get_slot_index(0, 16), None);
    }

    #[test]
    fn test_get_cell_by_id() {
        let mut dialog = StorageDialog::new();

        let item1 = UserItem { unique_id: 1001, ..Default::default() };
        let item2 = UserItem { unique_id: 2002, ..Default::default() };

        dialog.set_item(5, Some(item1));
        dialog.set_item(10, Some(item2));

        assert_eq!(dialog.get_cell(1001), Some(5));
        assert_eq!(dialog.get_cell(2002), Some(10));
        assert_eq!(dialog.get_cell(3003), None);
    }

    #[test]
    fn test_refresh_storage_ui() {
        let mut dialog = StorageDialog::new();

        // 测试Storage1
        dialog.refresh_storage1();
        assert!(dialog.storage1_selected);
        assert!(!dialog.storage2_selected);
        assert!(!dialog.rent_button_visible);
        assert!(!dialog.locked_page_visible);

        // 测试Storage2 (未扩展)
        dialog.refresh_storage2();
        assert!(!dialog.storage1_selected);
        assert!(dialog.storage2_selected);
        assert!(dialog.rent_button_visible);
        assert!(dialog.locked_page_visible);
        assert_eq!(dialog.rental_label_text, "扩展仓库已锁定");
        assert_eq!(dialog.rental_label_color, (255, 0, 0));

        // 测试Storage2 (已扩展)
        dialog.enable_expanded_storage(Some(1234567890));
        dialog.refresh_storage2();
        assert!(!dialog.locked_page_visible);
        assert_eq!(dialog.rental_label_color, (255, 255, 255));
    }
}