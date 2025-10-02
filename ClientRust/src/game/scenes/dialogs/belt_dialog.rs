// Belt Dialog - 腰带快捷栏对话框
// 提供6个快捷物品槽，支持水平/垂直布局切换

use mir2_shared::UserItem;

/// 腰带槽数量
pub const BELT_SLOT_COUNT: usize = 6;

/// 腰带布局方向
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BeltOrientation {
    /// 水平布局 (默认，在屏幕底部)
    Horizontal,
    /// 垂直布局 (在屏幕左侧)
    Vertical,
}

/// 腰带对话框 - 快捷物品栏
/// 
/// 功能:
/// - 6个快捷物品槽
/// - 布局切换 (水平/垂直)
/// - 快捷键绑定 (1-6)
/// - 透明背景
#[derive(Debug, Clone)]
pub struct BeltDialog {
    /// 腰带物品槽 (6个)
    pub slots: [Option<UserItem>; BELT_SLOT_COUNT],
    /// 布局方向
    pub orientation: BeltOrientation,
    /// 是否可见
    pub visible: bool,
    /// 快捷键标签 (1-6)
    pub key_labels: [String; BELT_SLOT_COUNT],
    /// 水平位置 (x坐标)
    pub position_x: i32,
    /// 垂直位置 (y坐标)
    pub position_y: i32,
}

impl Default for BeltDialog {
    fn default() -> Self {
        Self::new()
    }
}

impl BeltDialog {
    /// 创建新的腰带对话框
    pub fn new() -> Self {
        Self {
            slots: Default::default(),
            orientation: BeltOrientation::Horizontal,
            visible: true,
            key_labels: ["1".to_string(), "2".to_string(), "3".to_string(), 
                         "4".to_string(), "5".to_string(), "6".to_string()],
            position_x: 230,
            position_y: 150,
        }
    }

    /// 显示对话框
    pub fn show(&mut self) {
        self.visible = true;
    }

    /// 隐藏对话框
    pub fn hide(&mut self) {
        self.visible = false;
    }

    /// 切换布局方向
    pub fn flip(&mut self) {
        self.orientation = match self.orientation {
            BeltOrientation::Horizontal => {
                // 切换到垂直布局，移动到左侧
                self.position_x = 0;
                self.position_y = 200;
                BeltOrientation::Vertical
            }
            BeltOrientation::Vertical => {
                // 切换到水平布局，移动到底部
                self.position_x = 230;
                self.position_y = 150;
                BeltOrientation::Horizontal
            }
        };
    }

    /// 获取指定槽位的物品
    pub fn get_item(&self, slot: usize) -> Option<&UserItem> {
        if slot < BELT_SLOT_COUNT {
            self.slots[slot].as_ref()
        } else {
            None
        }
    }

    /// 设置指定槽位的物品
    pub fn set_item(&mut self, slot: usize, item: Option<UserItem>) -> bool {
        if slot < BELT_SLOT_COUNT {
            self.slots[slot] = item;
            true
        } else {
            false
        }
    }

    /// 移除指定槽位的物品
    pub fn remove_item(&mut self, slot: usize) -> Option<UserItem> {
        if slot < BELT_SLOT_COUNT {
            self.slots[slot].take()
        } else {
            None
        }
    }

    /// 通过物品ID查找槽位
    pub fn find_slot_by_id(&self, unique_id: u64) -> Option<usize> {
        for (i, slot) in self.slots.iter().enumerate() {
            if let Some(item) = slot {
                if item.unique_id == unique_id {
                    return Some(i);
                }
            }
        }
        None
    }

    /// 获取空闲槽位
    pub fn get_empty_slot(&self) -> Option<usize> {
        for (i, slot) in self.slots.iter().enumerate() {
            if slot.is_none() {
                return Some(i);
            }
        }
        None
    }

    /// 清空所有槽位
    pub fn clear(&mut self) {
        for slot in &mut self.slots {
            *slot = None;
        }
    }

    /// 获取当前布局的槽位位置 (用于渲染)
    pub fn get_slot_position(&self, slot: usize) -> Option<(i32, i32)> {
        if slot >= BELT_SLOT_COUNT {
            return None;
        }

        match self.orientation {
            BeltOrientation::Horizontal => {
                // 水平排列: 每个槽位宽35像素
                Some((self.position_x + (slot as i32 * 35) + 12, self.position_y + 3))
            }
            BeltOrientation::Vertical => {
                // 垂直排列: 每个槽位高35像素
                Some((self.position_x + 3, self.position_y + (slot as i32 * 35) + 12))
            }
        }
    }

    /// 获取快捷键标签位置
    pub fn get_key_label_position(&self, slot: usize) -> Option<(i32, i32)> {
        if slot >= BELT_SLOT_COUNT {
            return None;
        }

        match self.orientation {
            BeltOrientation::Horizontal => {
                Some((self.position_x + 8 + (slot as i32 * 35), self.position_y + 2))
            }
            BeltOrientation::Vertical => {
                Some((self.position_x - 1, self.position_y + 11 + (slot as i32 * 35)))
            }
        }
    }

    /// 检查物品是否在腰带中
    pub fn contains_item(&self, unique_id: u64) -> bool {
        self.find_slot_by_id(unique_id).is_some()
    }

    /// 获取已使用的槽位数量
    pub fn used_slots(&self) -> usize {
        self.slots.iter().filter(|s| s.is_some()).count()
    }

    /// 是否有空槽
    pub fn has_empty_slot(&self) -> bool {
        self.get_empty_slot().is_some()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn create_test_item(unique_id: u64, name: &str) -> UserItem {
        UserItem {
            unique_id,
            item_index: 100,
            name: name.to_string(),
            ..Default::default()
        }
    }

    #[test]
    fn test_belt_dialog_new() {
        let belt = BeltDialog::new();
        assert!(belt.visible);
        assert_eq!(belt.orientation, BeltOrientation::Horizontal);
        assert_eq!(belt.slots.len(), BELT_SLOT_COUNT);
        assert_eq!(belt.used_slots(), 0);
    }

    #[test]
    fn test_set_and_get_item() {
        let mut belt = BeltDialog::new();
        let item = create_test_item(1, "Potion");

        assert!(belt.set_item(0, Some(item.clone())));
        assert!(belt.get_item(0).is_some());
        assert_eq!(belt.get_item(0).unwrap().name, "Potion");
        assert_eq!(belt.used_slots(), 1);
    }

    #[test]
    fn test_remove_item() {
        let mut belt = BeltDialog::new();
        let item = create_test_item(1, "Potion");

        belt.set_item(0, Some(item));
        assert!(belt.get_item(0).is_some());

        let removed = belt.remove_item(0);
        assert!(removed.is_some());
        assert_eq!(removed.unwrap().name, "Potion");
        assert!(belt.get_item(0).is_none());
    }

    #[test]
    fn test_find_slot_by_id() {
        let mut belt = BeltDialog::new();
        let item1 = create_test_item(100, "Potion");
        let item2 = create_test_item(200, "Scroll");

        belt.set_item(2, Some(item1));
        belt.set_item(4, Some(item2));

        assert_eq!(belt.find_slot_by_id(100), Some(2));
        assert_eq!(belt.find_slot_by_id(200), Some(4));
        assert_eq!(belt.find_slot_by_id(999), None);
    }

    #[test]
    fn test_flip_orientation() {
        let mut belt = BeltDialog::new();
        assert_eq!(belt.orientation, BeltOrientation::Horizontal);

        belt.flip();
        assert_eq!(belt.orientation, BeltOrientation::Vertical);
        assert_eq!(belt.position_x, 0);
        assert_eq!(belt.position_y, 200);

        belt.flip();
        assert_eq!(belt.orientation, BeltOrientation::Horizontal);
        assert_eq!(belt.position_x, 230);
        assert_eq!(belt.position_y, 150);
    }

    #[test]
    fn test_slot_positions() {
        let belt = BeltDialog::new();

        // 水平布局
        let pos0 = belt.get_slot_position(0).unwrap();
        let pos1 = belt.get_slot_position(1).unwrap();
        assert_eq!(pos1.0 - pos0.0, 35); // 35像素间距

        let mut belt = BeltDialog::new();
        belt.flip();

        // 垂直布局
        let pos0 = belt.get_slot_position(0).unwrap();
        let pos1 = belt.get_slot_position(1).unwrap();
        assert_eq!(pos1.1 - pos0.1, 35); // 35像素间距
    }

    #[test]
    fn test_get_empty_slot() {
        let mut belt = BeltDialog::new();
        assert_eq!(belt.get_empty_slot(), Some(0));

        belt.set_item(0, Some(create_test_item(1, "Item1")));
        assert_eq!(belt.get_empty_slot(), Some(1));

        // 填满所有槽位
        for i in 1..BELT_SLOT_COUNT {
            belt.set_item(i, Some(create_test_item(i as u64 + 1, &format!("Item{}", i + 1))));
        }
        assert_eq!(belt.get_empty_slot(), None);
        assert!(!belt.has_empty_slot());
    }

    #[test]
    fn test_clear() {
        let mut belt = BeltDialog::new();
        for i in 0..BELT_SLOT_COUNT {
            belt.set_item(i, Some(create_test_item(i as u64, &format!("Item{}", i))));
        }
        assert_eq!(belt.used_slots(), BELT_SLOT_COUNT);

        belt.clear();
        assert_eq!(belt.used_slots(), 0);
        assert!(belt.has_empty_slot());
    }

    #[test]
    fn test_show_hide() {
        let mut belt = BeltDialog::new();
        assert!(belt.visible);

        belt.hide();
        assert!(!belt.visible);

        belt.show();
        assert!(belt.visible);
    }

    #[test]
    fn test_key_labels() {
        let belt = BeltDialog::new();
        assert_eq!(belt.key_labels.len(), BELT_SLOT_COUNT);
        assert_eq!(belt.key_labels[0], "1");
        assert_eq!(belt.key_labels[5], "6");
    }

    #[test]
    fn test_contains_item() {
        let mut belt = BeltDialog::new();
        let item = create_test_item(123, "TestItem");

        assert!(!belt.contains_item(123));
        belt.set_item(3, Some(item));
        assert!(belt.contains_item(123));
    }
}
