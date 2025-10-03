// Socket Dialog - 宝石镶嵌对话框
// 显示装备的宝石槽，支持镶嵌/拆卸宝石

use mir2_shared::UserItem;

/// 最大宝石槽数量 (6x2 = 12)
pub const MAX_SOCKET_SLOTS: usize = 12;

/// 宝石槽布局 (6列x2行)
pub const SOCKET_COLUMNS: usize = 6;
pub const SOCKET_ROWS: usize = 2;

/// 宝石镶嵌对话框
/// 
/// 功能:
/// - 显示装备的宝石槽 (1-12个)
/// - 宝石镶嵌
/// - 宝石拆卸
/// - 动态大小 (根据槽数调整)
#[derive(Debug, Clone)]
pub struct SocketDialog {
    /// 当前选中的装备
    pub selected_item: Option<UserItem>,
    /// 宝石槽 (最多12个)
    pub sockets: Vec<Option<UserItem>>,
    /// 是否可见
    pub visible: bool,
    /// 对话框位置 (x, y)
    pub position: (i32, i32),
    /// 对话框索引 (20 + socket_count - 1)
    pub dialog_index: i32,
}

impl Default for SocketDialog {
    fn default() -> Self {
        Self::new()
    }
}

impl SocketDialog {
    /// 创建新的宝石镶嵌对话框
    pub fn new() -> Self {
        Self {
            selected_item: None,
            sockets: Vec::new(),
            visible: false,
            position: (0, 0),
            dialog_index: 20,
        }
    }

    /// 显示对话框 (选择装备)
    pub fn show(&mut self, item: UserItem) {
        if item.slots.is_empty() {
            self.selected_item = None;
            self.visible = false;
            return;
        }

        self.selected_item = Some(item.clone());
        
        // 初始化宝石槽
        let slot_count = item.slots.len().min(MAX_SOCKET_SLOTS);
        self.sockets = item.slots.iter().take(slot_count).cloned().collect();
        
        // 设置对话框索引 (20 + slot_count - 1)
        self.dialog_index = 20 + slot_count as i32 - 1;
        
        self.visible = true;
    }

    /// 隐藏对话框
    pub fn hide(&mut self) {
        self.selected_item = None;
        self.sockets.clear();
        self.visible = false;
    }

    /// 获取指定槽位的宝石
    pub fn get_socket(&self, slot: usize) -> Option<&UserItem> {
        if slot < self.sockets.len() {
            self.sockets[slot].as_ref()
        } else {
            None
        }
    }

    /// 镶嵌宝石到指定槽位
    pub fn socket_gem(&mut self, slot: usize, gem: UserItem) -> bool {
        if slot >= self.sockets.len() {
            return false;
        }

        if self.sockets[slot].is_some() {
            // 槽位已有宝石
            return false;
        }

        self.sockets[slot] = Some(gem.clone());
        
        // 更新选中装备的槽位
        if let Some(ref mut item) = self.selected_item {
            if slot < item.slots.len() {
                item.slots[slot] = Some(gem);
            }
        }
        
        true
    }

    /// 从指定槽位拆卸宝石
    pub fn unsocket_gem(&mut self, slot: usize) -> Option<UserItem> {
        if slot >= self.sockets.len() {
            return None;
        }

        let gem = self.sockets[slot].take();
        
        // 更新选中装备的槽位
        if let Some(ref mut item) = self.selected_item {
            if slot < item.slots.len() {
                item.slots[slot] = None;
            }
        }
        
        gem
    }

    /// 获取宝石槽数量
    pub fn socket_count(&self) -> usize {
        self.sockets.len()
    }

    /// 获取已镶嵌宝石数量
    pub fn socketed_count(&self) -> usize {
        self.sockets.iter().filter(|s| s.is_some()).count()
    }

    /// 检查是否有空槽
    pub fn has_empty_socket(&self) -> bool {
        self.sockets.iter().any(|s| s.is_none())
    }

    /// 获取第一个空槽索引
    pub fn get_first_empty_socket(&self) -> Option<usize> {
        self.sockets.iter().position(|s| s.is_none())
    }

    /// 获取槽位在对话框中的位置 (用于渲染)
    pub fn get_socket_position(&self, slot: usize) -> Option<(i32, i32)> {
        if slot >= self.sockets.len() {
            return None;
        }

        let col = slot % SOCKET_COLUMNS;
        let row = slot / SOCKET_COLUMNS;
        
        // C#: x * 36 + 23 + x, y * 33 + 15 + y
        let x = self.position.0 + (col as i32 * 36) + 23 + col as i32;
        let y = self.position.1 + (row as i32 * 33) + 15 + row as i32;
        
        Some((x, y))
    }

    /// 设置对话框位置 (相对于背包或角色对话框)
    pub fn set_position_relative_to_inventory(&mut self, inventory_x: i32, inventory_y: i32, inventory_width: i32, inventory_height: i32) {
        // 居中显示在背包对话框下方
        let dialog_width = 264; // 假设宽度
        let x = inventory_x + (inventory_width - dialog_width) / 2;
        let y = inventory_y + inventory_height + 5;
        self.position = (x, y);
    }

    /// 设置对话框位置 (相对于角色对话框)
    pub fn set_position_relative_to_character(&mut self, character_x: i32, character_y: i32, character_width: i32, character_height: i32) {
        // 居中显示在角色对话框下方
        let dialog_width = 264;
        let x = character_x + (character_width - dialog_width) / 2;
        let y = character_y + character_height + 5;
        self.position = (x, y);
    }

    /// 检查宝石是否在槽位中
    pub fn contains_gem(&self, unique_id: u64) -> bool {
        self.sockets.iter().any(|s| {
            if let Some(gem) = s {
                gem.unique_id == unique_id
            } else {
                false
            }
        })
    }

    /// 通过宝石ID查找槽位
    pub fn find_socket_by_gem_id(&self, unique_id: u64) -> Option<usize> {
        self.sockets.iter().position(|s| {
            if let Some(gem) = s {
                gem.unique_id == unique_id
            } else {
                false
            }
        })
    }

    /// 清空所有槽位
    pub fn clear_sockets(&mut self) {
        for socket in &mut self.sockets {
            *socket = None;
        }
        
        if let Some(ref mut item) = self.selected_item {
            for slot in &mut item.slots {
                *slot = None;
            }
        }
    }

    /// 获取选中装备的名称
    pub fn get_selected_item_name(&self) -> Option<String> {
        self.selected_item.as_ref().and_then(|item| {
            item.info.as_ref().map(|info| info.name.clone())
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn create_test_item(unique_id: u64, name: &str, socket_count: usize) -> UserItem {
        let mut item = UserItem {
            unique_id,
            item_index: 100,
            name: name.to_string(),
            slots: vec![None; socket_count],
            ..Default::default()
        };
        item
    }

    fn create_test_gem(unique_id: u64, name: &str) -> UserItem {
        UserItem {
            unique_id,
            item_index: 200,
            name: name.to_string(),
            ..Default::default()
        }
    }

    #[test]
    fn test_socket_dialog_new() {
        let dialog = SocketDialog::new();
        assert!(!dialog.visible);
        assert!(dialog.selected_item.is_none());
        assert_eq!(dialog.socket_count(), 0);
    }

    #[test]
    fn test_show_with_sockets() {
        let mut dialog = SocketDialog::new();
        let item = create_test_item(1, "Helmet", 4);

        dialog.show(item);
        assert!(dialog.visible);
        assert_eq!(dialog.socket_count(), 4);
        assert_eq!(dialog.dialog_index, 23); // 20 + 4 - 1
    }

    #[test]
    fn test_show_without_sockets() {
        let mut dialog = SocketDialog::new();
        let item = create_test_item(1, "Helmet", 0);

        dialog.show(item);
        assert!(!dialog.visible);
        assert!(dialog.selected_item.is_none());
    }

    #[test]
    fn test_socket_gem() {
        let mut dialog = SocketDialog::new();
        let item = create_test_item(1, "Helmet", 4);
        dialog.show(item);

        let gem = create_test_gem(10, "Ruby");
        assert!(dialog.socket_gem(0, gem.clone()));
        assert_eq!(dialog.socketed_count(), 1);
        
        let socketed = dialog.get_socket(0).unwrap();
        assert_eq!(socketed.name, "Ruby");
    }

    #[test]
    fn test_socket_gem_to_occupied_slot() {
        let mut dialog = SocketDialog::new();
        let item = create_test_item(1, "Helmet", 4);
        dialog.show(item);

        let gem1 = create_test_gem(10, "Ruby");
        let gem2 = create_test_gem(11, "Sapphire");

        assert!(dialog.socket_gem(0, gem1));
        assert!(!dialog.socket_gem(0, gem2)); // 槽位已占用
        assert_eq!(dialog.socketed_count(), 1);
    }

    #[test]
    fn test_unsocket_gem() {
        let mut dialog = SocketDialog::new();
        let item = create_test_item(1, "Helmet", 4);
        dialog.show(item);

        let gem = create_test_gem(10, "Ruby");
        dialog.socket_gem(0, gem);
        assert_eq!(dialog.socketed_count(), 1);

        let removed = dialog.unsocket_gem(0);
        assert!(removed.is_some());
        assert_eq!(removed.unwrap().name, "Ruby");
        assert_eq!(dialog.socketed_count(), 0);
    }

    #[test]
    fn test_get_first_empty_socket() {
        let mut dialog = SocketDialog::new();
        let item = create_test_item(1, "Helmet", 4);
        dialog.show(item);

        assert_eq!(dialog.get_first_empty_socket(), Some(0));

        dialog.socket_gem(0, create_test_gem(10, "Ruby"));
        assert_eq!(dialog.get_first_empty_socket(), Some(1));

        dialog.socket_gem(1, create_test_gem(11, "Sapphire"));
        assert_eq!(dialog.get_first_empty_socket(), Some(2));

        // 填满所有槽位
        dialog.socket_gem(2, create_test_gem(12, "Emerald"));
        dialog.socket_gem(3, create_test_gem(13, "Diamond"));
        assert!(dialog.get_first_empty_socket().is_none());
        assert!(!dialog.has_empty_socket());
    }

    #[test]
    fn test_contains_gem() {
        let mut dialog = SocketDialog::new();
        let item = create_test_item(1, "Helmet", 4);
        dialog.show(item);

        assert!(!dialog.contains_gem(10));

        dialog.socket_gem(0, create_test_gem(10, "Ruby"));
        assert!(dialog.contains_gem(10));
        assert!(!dialog.contains_gem(11));
    }

    #[test]
    fn test_find_socket_by_gem_id() {
        let mut dialog = SocketDialog::new();
        let item = create_test_item(1, "Helmet", 4);
        dialog.show(item);

        dialog.socket_gem(0, create_test_gem(10, "Ruby"));
        dialog.socket_gem(2, create_test_gem(20, "Sapphire"));

        assert_eq!(dialog.find_socket_by_gem_id(10), Some(0));
        assert_eq!(dialog.find_socket_by_gem_id(20), Some(2));
        assert_eq!(dialog.find_socket_by_gem_id(99), None);
    }

    #[test]
    fn test_clear_sockets() {
        let mut dialog = SocketDialog::new();
        let item = create_test_item(1, "Helmet", 4);
        dialog.show(item);

        dialog.socket_gem(0, create_test_gem(10, "Ruby"));
        dialog.socket_gem(1, create_test_gem(11, "Sapphire"));
        assert_eq!(dialog.socketed_count(), 2);

        dialog.clear_sockets();
        assert_eq!(dialog.socketed_count(), 0);
        assert!(dialog.has_empty_socket());
    }

    #[test]
    fn test_get_socket_position() {
        let mut dialog = SocketDialog::new();
        dialog.position = (100, 200);
        let item = create_test_item(1, "Helmet", 6);
        dialog.show(item);

        // 第一个槽位
        let pos0 = dialog.get_socket_position(0).unwrap();
        assert_eq!(pos0, (123, 215)); // 100 + 23, 200 + 15

        // 第二个槽位 (同一行)
        let pos1 = dialog.get_socket_position(1).unwrap();
        assert_eq!(pos1.1, pos0.1); // 同一行
        assert!(pos1.0 > pos0.0); // 在右边

        // 第七个槽位 (第二行) - 如果有的话
        if dialog.socket_count() > 6 {
            let pos6 = dialog.get_socket_position(6).unwrap();
            assert!(pos6.1 > pos0.1); // 在下面
        }
    }

    #[test]
    fn test_hide() {
        let mut dialog = SocketDialog::new();
        let item = create_test_item(1, "Helmet", 4);
        dialog.show(item);
        assert!(dialog.visible);

        dialog.hide();
        assert!(!dialog.visible);
        assert!(dialog.selected_item.is_none());
        assert_eq!(dialog.socket_count(), 0);
    }

    #[test]
    fn test_get_selected_item_name() {
        let mut dialog = SocketDialog::new();
        assert!(dialog.get_selected_item_name().is_none());

        let item = create_test_item(1, "Dragon Helmet", 4);
        dialog.show(item);
        assert_eq!(dialog.get_selected_item_name(), Some("Dragon Helmet".to_string()));
    }

    #[test]
    fn test_max_socket_slots() {
        let mut dialog = SocketDialog::new();
        let mut item = create_test_item(1, "Epic Armor", 15); // 超过最大值
        dialog.show(item);

        // 应该限制为MAX_SOCKET_SLOTS
        assert_eq!(dialog.socket_count(), MAX_SOCKET_SLOTS);
    }

    #[test]
    fn test_dialog_index_calculation() {
        let mut dialog = SocketDialog::new();
        
        let item1 = create_test_item(1, "Item1", 1);
        dialog.show(item1);
        assert_eq!(dialog.dialog_index, 20); // 20 + 1 - 1

        let item4 = create_test_item(2, "Item4", 4);
        dialog.show(item4);
        assert_eq!(dialog.dialog_index, 23); // 20 + 4 - 1

        let item12 = create_test_item(3, "Item12", 12);
        dialog.show(item12);
        assert_eq!(dialog.dialog_index, 31); // 20 + 12 - 1
    }
}
