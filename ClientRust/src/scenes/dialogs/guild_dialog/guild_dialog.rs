// GuildDialog - 公会对话框
// 对应C#的GuildDialog类

use crate::scenes::dialogs::Dialog;

/// Guild member info - 公会成员信息
#[derive(Debug, Clone)]
pub struct GuildMember {
    pub name: String,
    pub rank_id: u8,
    pub rank_name: String,
    pub online: bool,
    pub level: u16,
    pub class: String,
}

impl GuildMember {
    pub fn new(name: String, rank_id: u8, rank_name: String) -> Self {
        Self {
            name,
            rank_id,
            rank_name,
            online: false,
            level: 1,
            class: String::from("Warrior"),
        }
    }
}

/// Guild dialog - 公会对话框
#[derive(Debug)]
pub struct GuildDialog {
    pub visible: bool,
    pub x: i32,
    pub y: i32,
    pub width: i32,
    pub height: i32,

    // 公会信息
    pub guild_name: String,
    pub guild_level: u16,
    pub guild_experience: u64,
    pub member_count: u16,
    pub max_members: u16,

    // 公会成员列表
    pub members: Vec<GuildMember>,

    // 公会排名
    pub guild_rank: u32,
    pub total_guilds: u32,

    // UI状态
    pub selected_tab: usize, // 0=成员, 1=设置, 2=排名
    pub create_button_pressed: bool,
    pub join_button_pressed: bool,
    pub leave_button_pressed: bool,
    pub manage_button_pressed: bool,
}

impl Default for GuildDialog {
    fn default() -> Self {
        Self {
            visible: false,
            x: 0,
            y: 0,
            width: 400,
            height: 500,
            guild_name: String::new(),
            guild_level: 0,
            guild_experience: 0,
            member_count: 0,
            max_members: 50,
            members: Vec::new(),
            guild_rank: 0,
            total_guilds: 0,
            selected_tab: 0,
            create_button_pressed: false,
            join_button_pressed: false,
            leave_button_pressed: false,
            manage_button_pressed: false,
        }
    }
}

impl Dialog for GuildDialog {
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
        // 更新公会对话框逻辑
    }

    fn draw(&self) {
        // 绘制公会对话框
    }

    fn name(&self) -> &str {
        "GuildDialog"
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