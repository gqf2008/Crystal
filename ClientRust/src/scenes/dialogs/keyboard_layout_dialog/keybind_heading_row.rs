// KeybindHeadingRow - 键盘绑定标题行控件
// Mirrors Client/MirScenes/Dialogs/KeyboardLayoutDialog.KeybindHeadingRow

use crate::scenes::dialogs::Dialog;

/// 键盘绑定标题行 - 显示分组标题的控件
pub struct KeybindHeadingRow {
    visible: bool,
    pub position: (i32, i32),
    pub size: (i32, i32),
    pub heading_text: String,
    pub is_expanded: bool,
}

impl KeybindHeadingRow {
    pub fn new(heading: &str) -> Self {
        Self {
            visible: true,
            position: (0, 0),
            size: (380, 30),
            heading_text: heading.to_string(),
            is_expanded: true,
        }
    }

    pub fn set_position(&mut self, x: i32, y: i32) {
        self.position = (x, y);
    }

    pub fn toggle_expanded(&mut self) {
        self.is_expanded = !self.is_expanded;
    }

    pub fn expand(&mut self) {
        self.is_expanded = true;
    }

    pub fn collapse(&mut self) {
        self.is_expanded = false;
    }
}

impl Dialog for KeybindHeadingRow {
    fn show(&mut self) {
        self.visible = true;
    }

    fn hide(&mut self) {
        self.visible = false;
    }

    fn update(&mut self, _delta_time: f32) {
        // Update expansion state
    }

    fn draw(&self) {
        if !self.visible {
            return;
        }
        // Draw heading background, text, expand/collapse indicator
    }

    fn is_visible(&self) -> bool {
        self.visible
    }

    fn name(&self) -> &str { "KeybindHeadingRow" }
    fn contains_point(&self, x: i32, y: i32) -> bool {
        x >= self.position.0 && x < self.position.0 + self.size.0 &&
        y >= self.position.1 && y < self.position.1 + self.size.1
    }
    fn position(&self) -> (i32, i32) { self.position }
    fn size(&self) -> (i32, i32) { self.size }
}