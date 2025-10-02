// Mount Dialog - 坐骑管理对话框
// 管理坐骑装备槽 (缰绳、铃铛、马鞍、彩带、面具)

use crate::game::items::UserItem;

/// 坐骑装备槽类型
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MountSlot {
    Reins = 0,   // 缰绳
    Bells = 1,   // 铃铛
    Saddle = 2,  // 马鞍
    Ribbon = 3,  // 彩带
    Mask = 4,    // 面具
}

/// 坐骑类型 (4槽或5槽)
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MountType {
    /// 4槽坐骑 (无面具槽)
    FourSlot,
    /// 5槽坐骑 (有面具槽)
    FiveSlot,
}

/// 坐骑对话框
/// 
/// 功能:
/// - 显示坐骑装备槽 (4-5个)
/// - 显示坐骑名称和忠诚度
/// - 上马/下马按钮
/// - 坐骑动画显示
#[derive(Debug, Clone)]
pub struct MountDialog {
    /// 坐骑装备槽 (最多5个)
    pub slots: Vec<Option<UserItem>>,
    /// 坐骑类型
    pub mount_type: MountType,
    /// 坐骑名称
    pub mount_name: String,
    /// 当前忠诚度
    pub current_loyalty: u32,
    /// 最大忠诚度
    pub max_loyalty: u32,
    /// 是否可见
    pub visible: bool,
    /// 坐骑动画索引
    pub mount_animation_index: i32,
    /// 对话框索引 (160或167)
    pub dialog_index: i32,
}

impl Default for MountDialog {
    fn default() -> Self {
        Self::new()
    }
}

impl MountDialog {
    /// 创建新的坐骑对话框
    pub fn new() -> Self {
        Self {
            slots: vec![None; 5],
            mount_type: MountType::FiveSlot,
            mount_name: String::new(),
            current_loyalty: 0,
            max_loyalty: 0,
            visible: false,
            mount_animation_index: 0,
            dialog_index: 167,
        }
    }

    /// 显示对话框
    pub fn show(&mut self, mount_item: &UserItem) {
        self.mount_name = mount_item.name.clone();
        self.current_loyalty = mount_item.current_dura;
        self.max_loyalty = mount_item.max_dura;
        
        let slot_count = mount_item.slots.len();
        self.mount_type = if slot_count == 4 {
            self.dialog_index = 160;
            MountType::FourSlot
        } else {
            self.dialog_index = 167;
            MountType::FiveSlot
        };

        self.slots = mount_item.slots.clone();
        if self.slots.len() < 5 {
            self.slots.resize(5, None);
        }

        self.visible = true;
    }

    /// 隐藏对话框
    pub fn hide(&mut self) {
        self.visible = false;
    }

    /// 获取指定槽位的物品
    pub fn get_slot(&self, slot: MountSlot) -> Option<&UserItem> {
        self.slots.get(slot as usize).and_then(|s| s.as_ref())
    }

    /// 设置指定槽位的物品
    pub fn set_slot(&mut self, slot: MountSlot, item: Option<UserItem>) {
        if (slot as usize) < self.slots.len() {
            self.slots[slot as usize] = item;
        }
    }

    /// 检查是否可以骑乘
    pub fn can_ride(&self) -> bool {
        self.mount_animation_index >= 0
    }

    /// 获取忠诚度文本
    pub fn get_loyalty_text(&self) -> String {
        format!("{} / {} Loyalty", self.current_loyalty, self.max_loyalty)
    }

    /// 获取槽位位置 (用于渲染)
    pub fn get_slot_position(&self, slot: MountSlot) -> (i32, i32) {
        let (x_offset, y_offset) = match self.mount_type {
            MountType::FourSlot => (1, 1),
            MountType::FiveSlot => (0, 0),
        };

        let base_y = 323 + y_offset;
        match slot {
            MountSlot::Reins => (36 + x_offset, base_y),
            MountSlot::Bells => (90 + x_offset, base_y),
            MountSlot::Saddle => (144 + x_offset, base_y),
            MountSlot::Ribbon => (198 + x_offset, base_y),
            MountSlot::Mask => (252 + x_offset, base_y),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn create_test_mount(name: &str, slot_count: usize) -> UserItem {
        UserItem {
            unique_id: 1,
            name: name.to_string(),
            current_dura: 5000,
            max_dura: 10000,
            slots: vec![None; slot_count],
            ..Default::default()
        }
    }

    #[test]
    fn test_mount_dialog_new() {
        let dialog = MountDialog::new();
        assert!(!dialog.visible);
        assert_eq!(dialog.mount_type, MountType::FiveSlot);
    }

    #[test]
    fn test_show_five_slot_mount() {
        let mut dialog = MountDialog::new();
        let mount = create_test_mount("Horse", 5);
        
        dialog.show(&mount);
        assert!(dialog.visible);
        assert_eq!(dialog.mount_type, MountType::FiveSlot);
        assert_eq!(dialog.dialog_index, 167);
        assert_eq!(dialog.mount_name, "Horse");
    }

    #[test]
    fn test_show_four_slot_mount() {
        let mut dialog = MountDialog::new();
        let mount = create_test_mount("Pony", 4);
        
        dialog.show(&mount);
        assert_eq!(dialog.mount_type, MountType::FourSlot);
        assert_eq!(dialog.dialog_index, 160);
    }

    #[test]
    fn test_get_loyalty_text() {
        let mut dialog = MountDialog::new();
        dialog.current_loyalty = 5000;
        dialog.max_loyalty = 10000;
        
        assert_eq!(dialog.get_loyalty_text(), "5000 / 10000 Loyalty");
    }
}
