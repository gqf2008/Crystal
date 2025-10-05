// Character Dura Panel - 角色耐久面板
// 对应C#的CharacterDuraPanel类

use crate::scenes::dialogs::Dialog;

/// 角色耐久面板
pub struct CharacterDuraPanel {
    visible: bool,
}

impl CharacterDuraPanel {
    /// 创建新的角色耐久面板
    pub fn new() -> Self {
        Self {
            visible: true,
        }
    }
}

impl Dialog for CharacterDuraPanel {
    fn show(&mut self) {
        self.visible = true;
    }

    fn hide(&mut self) {
        self.visible = false;
    }

    fn is_visible(&self) -> bool {
        self.visible
    }

    fn toggle(&mut self) {
        self.visible = !self.visible;
    }

    fn update(&mut self, _delta_time: f32) {
        // Update logic
    }

    fn draw(&self) {
        if !self.visible {
            return;
        }
        // Draw logic
    }

    fn name(&self) -> &str {
        "CharacterDuraPanel"
    }

    fn contains_point(&self, x: i32, y: i32) -> bool {
        x >= 0 && x < 150 && y >= 0 && y < 80
    }

    fn position(&self) -> (i32, i32) {
        (0, 0)
    }

    fn size(&self) -> (i32, i32) {
        (150, 80)
    }
}

impl Default for CharacterDuraPanel {
    fn default() -> Self {
        Self::new()
    }
}