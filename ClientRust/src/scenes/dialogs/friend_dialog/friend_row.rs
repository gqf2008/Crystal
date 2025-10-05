// FriendRow - 好友行控件
// 对应C#的FriendRow类

/// Friend row - 好友行控件
#[derive(Debug)]
pub struct FriendRow {
    pub visible: bool,
    pub x: i32,
    pub y: i32,
    pub width: i32,
    pub height: i32,

    // 好友信息
    pub friend_name: String,
    pub friend_level: u16,
    pub friend_class: String,
    pub is_online: bool,
    pub last_seen: String,

    // 行状态
    pub is_selected: bool,
    pub row_index: usize,
}

impl Default for FriendRow {
    fn default() -> Self {
        Self {
            visible: true,
            x: 0,
            y: 0,
            width: 320,
            height: 25,
            friend_name: String::new(),
            friend_level: 0,
            friend_class: String::new(),
            is_online: false,
            last_seen: String::new(),
            is_selected: false,
            row_index: 0,
        }
    }
}