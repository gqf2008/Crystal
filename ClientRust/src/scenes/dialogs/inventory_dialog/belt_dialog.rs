// Belt Dialog - 腰带快捷栏对话框
// 提供6个快捷物品槽，支持水平/垂直布局切换

use crate::scenes::dialogs::Dialog;
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
    visible: bool,
    /// 腰带物品槽 (6个)
    pub slots: [Option<UserItem>; BELT_SLOT_COUNT],
    /// 布局方向
    pub orientation: BeltOrientation,
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
            visible: true,
            slots: Default::default(),
            orientation: BeltOrientation::Horizontal,
            key_labels: ["1".to_string(), "2".to_string(), "3".to_string(),
                         "4".to_string(), "5".to_string(), "6".to_string()],
            position_x: 230,
            position_y: 150,
        }
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

impl Dialog for BeltDialog {
    fn show(&mut self) {
        self.visible = true;
    }

    fn hide(&mut self) {
        self.visible = false;
    }

    fn update(&mut self, _delta_time: f32) {
        // Update key bindings and item availability
    }

    fn draw(&self) {
        if !self.visible {
            return;
        }
        // Draw belt background, slots, key labels, and items
    }

    fn is_visible(&self) -> bool {
        self.visible
    }

    fn name(&self) -> &str { "BeltDialog" }

    fn contains_point(&self, x: i32, y: i32) -> bool {
        match self.orientation {
            BeltOrientation::Horizontal => {
                x >= self.position_x && x < self.position_x + 222 &&
                y >= self.position_y && y < self.position_y + 38
            }
            BeltOrientation::Vertical => {
                x >= self.position_x && x < self.position_x + 38 &&
                y >= self.position_y && y < self.position_y + 222
            }
        }
    }

    fn position(&self) -> (i32, i32) {
        (self.position_x, self.position_y)
    }

    fn size(&self) -> (i32, i32) {
        match self.orientation {
            BeltOrientation::Horizontal => (222, 38),
            BeltOrientation::Vertical => (38, 222),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_belt_dialog_creation() {
        let dialog = BeltDialog::new();
        assert!(dialog.is_visible());
        assert_eq!(dialog.orientation, BeltOrientation::Horizontal);
        assert_eq!(dialog.used_slots(), 0);
        assert!(dialog.has_empty_slot());
    }

    #[test]
    fn test_belt_flip() {
        let mut dialog = BeltDialog::new();

        // Initial state
        assert_eq!(dialog.orientation, BeltOrientation::Horizontal);
        assert_eq!(dialog.position_x, 230);
        assert_eq!(dialog.position_y, 150);

        // Flip to vertical
        dialog.flip();
        assert_eq!(dialog.orientation, BeltOrientation::Vertical);
        assert_eq!(dialog.position_x, 0);
        assert_eq!(dialog.position_y, 200);

        // Flip back to horizontal
        dialog.flip();
        assert_eq!(dialog.orientation, BeltOrientation::Horizontal);
        assert_eq!(dialog.position_x, 230);
        assert_eq!(dialog.position_y, 150);
    }

    #[test]
    fn test_slot_operations() {
        let mut dialog = BeltDialog::new();

        // Set item in slot 0
        let item = UserItem::default();
        assert!(dialog.set_item(0, Some(item.clone())));
        assert_eq!(dialog.used_slots(), 1);
        assert_eq!(dialog.get_item(0).unwrap().unique_id, item.unique_id);

        // Remove item
        let removed = dialog.remove_item(0);
        assert!(removed.is_some());
        assert_eq!(dialog.used_slots(), 0);
    }

    #[test]
    fn test_slot_positions() {
        let dialog = BeltDialog::new();

        // Horizontal layout
        let pos = dialog.get_slot_position(0).unwrap();
        assert_eq!(pos, (230 + 12, 150 + 3));

        let pos = dialog.get_slot_position(5).unwrap();
        assert_eq!(pos, (230 + 5 * 35 + 12, 150 + 3));

        // Flip to vertical
        let mut dialog = BeltDialog::new();
        dialog.flip();

        let pos = dialog.get_slot_position(0).unwrap();
        assert_eq!(pos, (0 + 3, 200 + 12));

        let pos = dialog.get_slot_position(5).unwrap();
        assert_eq!(pos, (0 + 3, 200 + 5 * 35 + 12));
    }
}