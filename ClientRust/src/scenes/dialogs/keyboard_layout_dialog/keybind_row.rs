// KeybindRow - 键盘绑定行控件
// Mirrors Client/MirScenes/Dialogs/KeyboardLayoutDialog.KeybindRow

use crate::scenes::dialogs::Dialog;

/// 键盘绑定行 - 显示单个键盘绑定的控件
pub struct KeybindRow {
    visible: bool,
    pub position: (i32, i32),
    pub size: (i32, i32),
    pub keybind_option: super::keyboard_layout_dialog::KeybindOption,
    pub keybind_info: super::keyboard_layout_dialog::KeybindInfo,
    pub is_selected: bool,
    pub is_editing: bool,
}

impl KeybindRow {
    pub fn new(option: super::keyboard_layout_dialog::KeybindOption, info: super::keyboard_layout_dialog::KeybindInfo) -> Self {
        Self {
            visible: true,
            position: (0, 0),
            size: (380, 25),
            keybind_option: option,
            keybind_info: info,
            is_selected: false,
            is_editing: false,
        }
    }

    pub fn set_position(&mut self, x: i32, y: i32) {
        self.position = (x, y);
    }

    pub fn select(&mut self) {
        self.is_selected = true;
    }

    pub fn deselect(&mut self) {
        self.is_selected = false;
    }

    pub fn start_editing(&mut self) {
        self.is_editing = true;
    }

    pub fn stop_editing(&mut self) {
        self.is_editing = false;
    }

    pub fn update_keybind(&mut self, info: super::keyboard_layout_dialog::KeybindInfo) {
        self.keybind_info = info;
        self.is_editing = false;
    }
}

impl Dialog for KeybindRow {
    fn show(&mut self) {
        self.visible = true;
    }

    fn hide(&mut self) {
        self.visible = false;
    }

    fn update(&mut self, _delta_time: f32) {
        // Update editing state
    }

    fn draw(&self) {
        if !self.visible {
            return;
        }
        // Draw row background, text, key display
    }

    fn is_visible(&self) -> bool {
        self.visible
    }

    fn name(&self) -> &str { "KeybindRow" }
    fn contains_point(&self, x: i32, y: i32) -> bool {
        x >= self.position.0 && x < self.position.0 + self.size.0 &&
        y >= self.position.1 && y < self.position.1 + self.size.1
    }
    fn position(&self) -> (i32, i32) { self.position }
    fn size(&self) -> (i32, i32) { self.size }
}