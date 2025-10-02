// Fishing Dialog - 钓鱼系统对话框
// 管理钓鱼装备和显示钓鱼进度

use mir2_shared::UserItem;

/// 钓鱼装备槽
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FishingSlot {
    Hook = 0,   // 鱼钩
    Float = 1,  // 浮标
    Bait = 2,   // 鱼饵
    Finder = 3, // 探鱼器
    Reel = 4,   // 渔线轮
}

/// 钓鱼对话框 - 装备管理
#[derive(Debug, Clone)]
pub struct FishingDialog {
    pub slots: [Option<UserItem>; 5],
    pub visible: bool,
    pub rod_name: String,
}

impl Default for FishingDialog {
    fn default() -> Self {
        Self::new()
    }
}

impl FishingDialog {
    pub fn new() -> Self {
        Self {
            slots: Default::default(),
            visible: false,
            rod_name: String::new(),
        }
    }

    pub fn show(&mut self, rod: &UserItem) {
        self.rod_name = rod.name.clone();
        if rod.slots.len() >= 5 {
            for i in 0..5 {
                self.slots[i] = rod.slots[i].clone();
            }
        }
        self.visible = true;
    }

    pub fn hide(&mut self) {
        self.visible = false;
    }

    pub fn get_slot(&self, slot: FishingSlot) -> Option<&UserItem> {
        self.slots[slot as usize].as_ref()
    }
}

/// 钓鱼状态对话框 - 显示钓鱼进度
#[derive(Debug, Clone)]
pub struct FishingStatusDialog {
    pub visible: bool,
    pub chance_percent: i32,      // 成功率 (0-100)
    pub progress_percent: i32,     // 进度 (0-100)
    pub auto_cast: bool,           // 自动抛竿
    pub can_auto_cast: bool,       // 是否可自动抛竿
    pub esc_exit: bool,            // ESC退出
}

impl Default for FishingStatusDialog {
    fn default() -> Self {
        Self::new()
    }
}

impl FishingStatusDialog {
    pub fn new() -> Self {
        Self {
            visible: false,
            chance_percent: 0,
            progress_percent: 0,
            auto_cast: false,
            can_auto_cast: false,
            esc_exit: false,
        }
    }

    pub fn show(&mut self) {
        self.visible = true;
    }

    pub fn hide(&mut self) {
        self.visible = false;
    }

    pub fn update_chance(&mut self, percent: i32) {
        self.chance_percent = percent.clamp(0, 100);
    }

    pub fn update_progress(&mut self, percent: i32) {
        self.progress_percent = percent.clamp(0, 100);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_fishing_dialog() {
        let mut dialog = FishingDialog::new();
        assert!(!dialog.visible);
    }

    #[test]
    fn test_fishing_status() {
        let mut status = FishingStatusDialog::new();
        status.update_chance(75);
        assert_eq!(status.chance_percent, 75);
        
        status.update_progress(50);
        assert_eq!(status.progress_percent, 50);
    }
}
