// CreatureButton - 生物按钮控件
// 对应C#的CreatureButton类

/// Creature button - 生物按钮控件
#[derive(Debug)]
pub struct CreatureButton {
    pub visible: bool,
    pub x: i32,
    pub y: i32,
    pub width: i32,
    pub height: i32,

    // 按钮数据
    pub creature_index: u32,
    pub creature_name: String,
    pub creature_level: u16,
    pub creature_type: u32,
    pub is_selected: bool,
    pub is_summoned: bool,
    pub button_pressed: bool,
}

impl Default for CreatureButton {
    fn default() -> Self {
        Self {
            visible: true,
            x: 0,
            y: 0,
            width: 80,
            height: 80,
            creature_index: 0,
            creature_name: String::new(),
            creature_level: 0,
            creature_type: 0,
            is_selected: false,
            is_summoned: false,
            button_pressed: false,
        }
    }
}