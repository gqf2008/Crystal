// MemoDialog - 备忘录对话框
// 对应C#的MemoDialog类

use crate::scenes::dialogs::Dialog;

/// Memo dialog - 备忘录对话框
#[derive(Debug)]
pub struct MemoDialog {
    pub visible: bool,
    pub x: i32,
    pub y: i32,
    pub width: i32,
    pub height: i32,

    // 备忘录内容
    pub memo_text: String,
    pub max_length: usize,
    pub friend_name: String,

    // UI状态
    pub save_button_pressed: bool,
    pub cancel_button_pressed: bool,
    pub text_changed: bool,
}

impl Default for MemoDialog {
    fn default() -> Self {
        Self {
            visible: false,
            x: 0,
            y: 0,
            width: 300,
            height: 200,
            memo_text: String::new(),
            max_length: 500,
            friend_name: String::new(),
            save_button_pressed: false,
            cancel_button_pressed: false,
            text_changed: false,
        }
    }
}

impl Dialog for MemoDialog {
    fn show(&mut self) {
        self.visible = true;
    }

    fn hide(&mut self) {
        self.visible = false;
    }

    fn is_visible(&self) -> bool {
        self.visible
    }

    fn update(&mut self, _delta_time: f32) {
        // 更新备忘录对话框逻辑
    }

    fn draw(&self) {
        // 绘制备忘录对话框
    }

    fn name(&self) -> &str {
        "MemoDialog"
    }

    fn contains_point(&self, x: i32, y: i32) -> bool {
        x >= self.x && x < self.x + self.width && y >= self.y && y < self.y + self.height
    }

    fn position(&self) -> (i32, i32) {
        (self.x, self.y)
    }

    fn size(&self) -> (i32, i32) {
        (self.width, self.height)
    }
}