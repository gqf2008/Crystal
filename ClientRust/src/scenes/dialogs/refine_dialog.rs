// Refine Dialog - 装备精炼对话框
// 放置材料精炼装备

use mir2_shared::UserItem;

/// 精炼槽数量 (4x4 = 16)
pub const REFINE_SLOT_COUNT: usize = 16;
pub const REFINE_ROWS: usize = 4;
pub const REFINE_COLS: usize = 4;

/// 装备精炼对话框
/// 
/// 功能:
/// - 16个材料槽 (4x4网格)
/// - 放置装备和精炼材料
/// - 精炼确认/取消
#[derive(Debug, Clone)]
pub struct RefineDialog {
    /// 材料槽 (4x4 = 16个)
    pub grid: [Option<UserItem>; REFINE_SLOT_COUNT],
    /// 是否可见
    pub visible: bool,
    /// 对话框位置
    pub position: (i32, i32),
}

impl Default for RefineDialog {
    fn default() -> Self {
        Self::new()
    }
}

impl RefineDialog {
    /// 创建新的精炼对话框
    pub fn new() -> Self {
        Self {
            grid: Default::default(),
            visible: false,
            position: (0, 225),
        }
    }

    /// 显示对话框
    pub fn show(&mut self) {
        self.visible = true;
    }

    /// 隐藏对话框 (取消精炼)
    pub fn hide(&mut self) {
        self.visible = false;
        self.refine_cancel();
    }

    /// 取消精炼 (清空槽位)
    pub fn refine_cancel(&mut self) {
        self.refine_reset();
    }

    /// 重置所有槽位
    pub fn refine_reset(&mut self) {
        for slot in &mut self.grid {
            *slot = None;
        }
    }

    /// 获取指定槽位的物品
    pub fn get_slot(&self, slot: usize) -> Option<&UserItem> {
        if slot < REFINE_SLOT_COUNT {
            self.grid[slot].as_ref()
        } else {
            None
        }
    }

    /// 设置指定槽位的物品
    pub fn set_slot(&mut self, slot: usize, item: Option<UserItem>) -> bool {
        if slot < REFINE_SLOT_COUNT {
            self.grid[slot] = item;
            true
        } else {
            false
        }
    }

    /// 通过物品ID查找槽位
    pub fn get_cell_by_id(&self, unique_id: u64) -> Option<usize> {
        for (i, slot) in self.grid.iter().enumerate() {
            if let Some(item) = slot {
                if item.unique_id == unique_id {
                    return Some(i);
                }
            }
        }
        None
    }

    /// 获取已使用的槽位数量
    pub fn used_slots(&self) -> usize {
        self.grid.iter().filter(|s| s.is_some()).count()
    }

    /// 获取槽位在对话框中的位置 (x, y)
    pub fn get_slot_position(&self, slot: usize) -> Option<(i32, i32)> {
        if slot >= REFINE_SLOT_COUNT {
            return None;
        }

        let col = slot % REFINE_COLS;
        let row = slot / REFINE_COLS;

        // C#: x * 34 + 12 + x, y * 32 + 37 + y
        let x = self.position.0 + (col as i32 * 34) + 12 + col as i32;
        let y = self.position.1 + (row as i32 * 32) + 37 + row as i32;

        Some((x, y))
    }
}
