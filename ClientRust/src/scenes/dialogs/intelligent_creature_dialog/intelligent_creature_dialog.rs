// IntelligentCreatureDialog - 智能生物对话框
// 对应C#的IntelligentCreatureDialog类

use crate::scenes::dialogs::Dialog;

/// Intelligent creature dialog - 智能生物对话框
#[derive(Debug)]
pub struct IntelligentCreatureDialog {
    pub visible: bool,
    pub x: i32,
    pub y: i32,
    pub width: i32,
    pub height: i32,

    // 生物信息
    pub creature_name: String,
    pub creature_deadline: String,
    pub creature_pearls: u32,
    pub creature_info: String,
    pub creature_info1: String,
    pub creature_info2: String,

    // 饱食度
    pub fullness_current: u32,
    pub fullness_max: u32,

    // 黑石生产
    pub pearl_count: u32,
    pub blackstone_produce_time: u64,

    // 按钮状态
    pub close_button_pressed: bool,
    pub help_button_pressed: bool,
    pub rename_button_pressed: bool,
    pub summon_button_pressed: bool,
    pub dismiss_button_pressed: bool,
    pub release_button_pressed: bool,
    pub automatic_mode_pressed: bool,
    pub semi_auto_mode_pressed: bool,
    pub options_menu_pressed: bool,

    // 生物按钮
    pub creature_buttons: Vec<crate::scenes::dialogs::CreatureButton>,
    pub selected_creature_slot: i32,

    // 动画状态
    pub switch_anim_time: u64,
    pub anim_switched: bool,
    pub anim_need_switch: bool,
}

impl Default for IntelligentCreatureDialog {
    fn default() -> Self {
        Self {
            visible: false,
            x: 0,
            y: 0,
            width: 400,
            height: 300,
            creature_name: String::new(),
            creature_deadline: String::new(),
            creature_pearls: 0,
            creature_info: String::new(),
            creature_info1: String::new(),
            creature_info2: String::new(),
            fullness_current: 0,
            fullness_max: 100,
            pearl_count: 0,
            blackstone_produce_time: 10800, // 3小时
            close_button_pressed: false,
            help_button_pressed: false,
            rename_button_pressed: false,
            summon_button_pressed: false,
            dismiss_button_pressed: false,
            release_button_pressed: false,
            automatic_mode_pressed: false,
            semi_auto_mode_pressed: false,
            options_menu_pressed: false,
            creature_buttons: Vec::new(),
            selected_creature_slot: -1,
            switch_anim_time: 0,
            anim_switched: false,
            anim_need_switch: false,
        }
    }
}

impl Dialog for IntelligentCreatureDialog {
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
        // 更新智能生物对话框逻辑
    }

    fn draw(&self) {
        // 绘制智能生物对话框
    }

    fn name(&self) -> &str {
        "IntelligentCreatureDialog"
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