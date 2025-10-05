// GuildBuffButton - 公会增益按钮控件
// 对应C#的GuildBuffButton类

/// Guild buff button - 公会增益按钮控件
#[derive(Debug)]
pub struct GuildBuffButton {
    pub visible: bool,
    pub x: i32,
    pub y: i32,
    pub width: i32,
    pub height: i32,

    // 增益信息
    pub buff_name: String,
    pub buff_description: String,
    pub buff_level: u16,
    pub max_buff_level: u16,
    pub buff_cost: u32,

    // 按钮状态
    pub is_active: bool,
    pub button_pressed: bool,
    pub can_upgrade: bool,
}

impl Default for GuildBuffButton {
    fn default() -> Self {
        Self {
            visible: true,
            x: 0,
            y: 0,
            width: 80,
            height: 80,
            buff_name: String::new(),
            buff_description: String::new(),
            buff_level: 0,
            max_buff_level: 10,
            buff_cost: 0,
            is_active: false,
            button_pressed: false,
            can_upgrade: false,
        }
    }
}