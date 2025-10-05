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