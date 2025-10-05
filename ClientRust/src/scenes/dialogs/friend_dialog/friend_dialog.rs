// FriendDialog - 好友对话框
// 对应C#的FriendDialog类

use crate::scenes::dialogs::Dialog;

/// Friend status - 好友状态
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FriendStatus {
    Online,  // 在线
    Offline, // 离线
    Blocked, // 被屏蔽
}

/// Friend info - 好友信息
#[derive(Debug, Clone)]
pub struct Friend {
    pub name: String,
    pub status: FriendStatus,
    pub level: u16,
    pub class: String,
    pub memo: String,     // 备注
    pub blocked: bool,    // 是否在黑名单
    pub added_time: i64,  // 添加时间(Unix时间戳)
}

impl Friend {
    /// 创建新好友
    pub fn new(name: String) -> Self {
        Self {
            name,
            status: FriendStatus::Offline,
            level: 1,
            class: String::from("Warrior"),
            memo: String::new(),
            blocked: false,
            added_time: 0,
        }
    }
}

/// Friend dialog - 好友对话框
#[derive(Debug)]
pub struct FriendDialog {
    pub visible: bool,
    pub x: i32,
    pub y: i32,
    pub width: i32,
    pub height: i32,

    // 好友列表
    pub friends: Vec<Friend>,
    pub selected_tab: usize, // 0=好友列表, 1=黑名单

    // 好友搜索
    pub search_text: String,
    pub search_results: Vec<String>,

    // UI状态
    pub add_button_pressed: bool,
    pub remove_button_pressed: bool,
    pub whisper_button_pressed: bool,
    pub block_button_pressed: bool,
    pub memo_button_pressed: bool,
}

impl Default for FriendDialog {
    fn default() -> Self {
        Self {
            visible: false,
            x: 0,
            y: 0,
            width: 350,
            height: 400,
            friends: Vec::new(),
            selected_tab: 0,
            search_text: String::new(),
            search_results: Vec::new(),
            add_button_pressed: false,
            remove_button_pressed: false,
            whisper_button_pressed: false,
            block_button_pressed: false,
            memo_button_pressed: false,
        }
    }
}

impl Dialog for FriendDialog {
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
        // 更新好友对话框逻辑
    }

    fn draw(&self) {
        // 绘制好友对话框
    }

    fn name(&self) -> &str {
        "FriendDialog"
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