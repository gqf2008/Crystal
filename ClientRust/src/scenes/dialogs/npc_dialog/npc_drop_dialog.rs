// NPC Drop Dialog - NPC掉落对话框
// 对应C#的NPCDropDialog类

use crate::scenes::dialogs::Dialog;

/// NPC掉落对话框
pub struct NPCDropDialog {
    visible: bool,
    x: i32,
    y: i32,
    width: i32,
    height: i32,
}

impl NPCDropDialog {
    /// 创建新的NPC掉落对话框
    pub fn new() -> Self {
        Self {
            visible: false,
            x: 0,
            y: 0,
            width: 400,
            height: 300,
        }
    }
}

impl Default for NPCDropDialog {
    fn default() -> Self {
        Self::new()
    }
}

impl Dialog for NPCDropDialog {
    fn show(&mut self) {
        self.visible = true;
    }

    fn hide(&mut self) {
        self.visible = false;
    }

    fn update(&mut self, _delta_time: f32) {
        // TODO: 实现更新逻辑
    }

    fn draw(&self) {
        if !self.visible {
            return;
        }
        // TODO: 实现渲染逻辑
    }

    fn is_visible(&self) -> bool {
        self.visible
    }

    fn name(&self) -> &str {
        "NPCDropDialog"
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