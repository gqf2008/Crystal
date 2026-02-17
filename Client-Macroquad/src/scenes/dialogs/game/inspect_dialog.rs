// ============================================================================
// InspectDialogHybrid - 查看他人装备（对齐 C# InspectDialog）
// ============================================================================
//
// C# 参考：Client/MirScenes/Dialogs/MainDialogs.cs:2095-2500
// - 背景：Prguse[430]
// - 角色页面：Prguse[340]（装备模型显示区）
// - 关闭按钮：Prguse2[360-362]
// - 14 个装备槽：武器/盔甲/头盔/火把/项链/左右手镯/左右戒指/护符/腰带/靴子/宝石/坐骑
// - 互动按钮：组队/好友/邮件/交易/伴侣/观战
//
// ============================================================================

use macroquad::prelude::*;
use crate::resources::LibraryName;
use crate::ui::text_renderer::draw_text_cn;
use super::native_ui_utils::*;

// ============================================================================
// 常量
// ============================================================================

/// 装备槽总数
const EQUIP_SLOTS: usize = 14;

// ============================================================================
// 类型定义
// ============================================================================

/// 装备槽位类型
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum InspectEquipSlot {
    Weapon = 0,
    Armor = 1,
    Helmet = 2,
    Torch = 3,
    Necklace = 4,
    BraceletL = 5,
    BraceletR = 6,
    RingL = 7,
    RingR = 8,
    Amulet = 9,
    Belt = 10,
    Boots = 11,
    Stone = 12,
    Mount = 13,
}

impl InspectEquipSlot {
    pub fn name(&self) -> &'static str {
        match self {
            Self::Weapon => "武器",
            Self::Armor => "盔甲",
            Self::Helmet => "头盔",
            Self::Torch => "火把",
            Self::Necklace => "项链",
            Self::BraceletL => "左手镯",
            Self::BraceletR => "右手镯",
            Self::RingL => "左戒指",
            Self::RingR => "右戒指",
            Self::Amulet => "护符",
            Self::Belt => "腰带",
            Self::Boots => "靴子",
            Self::Stone => "宝石",
            Self::Mount => "坐骑",
        }
    }

    /// 获取槽位在面板中的相对位置 (x, y)
    pub fn position(&self) -> (f32, f32) {
        match self {
            // 左侧列
            Self::Weapon    => (14.0, 94.0),
            Self::Armor     => (14.0, 130.0),
            Self::Helmet    => (14.0, 166.0),
            Self::Torch     => (14.0, 202.0),
            Self::Necklace  => (14.0, 238.0),
            Self::BraceletL => (14.0, 274.0),
            Self::RingL     => (14.0, 310.0),
            // 右侧列
            Self::BraceletR => (206.0, 94.0),
            Self::RingR     => (206.0, 130.0),
            Self::Amulet    => (206.0, 166.0),
            Self::Belt      => (206.0, 202.0),
            Self::Boots     => (206.0, 238.0),
            Self::Stone     => (206.0, 274.0),
            Self::Mount     => (206.0, 310.0),
        }
    }

    pub fn from_index(idx: usize) -> Option<Self> {
        match idx {
            0 => Some(Self::Weapon),
            1 => Some(Self::Armor),
            2 => Some(Self::Helmet),
            3 => Some(Self::Torch),
            4 => Some(Self::Necklace),
            5 => Some(Self::BraceletL),
            6 => Some(Self::BraceletR),
            7 => Some(Self::RingL),
            8 => Some(Self::RingR),
            9 => Some(Self::Amulet),
            10 => Some(Self::Belt),
            11 => Some(Self::Boots),
            12 => Some(Self::Stone),
            13 => Some(Self::Mount),
            _ => None,
        }
    }
}

/// 被查看玩家的装备数据
#[derive(Debug, Clone)]
pub struct InspectEquipItem {
    pub icon_index: usize,
    pub name: String,
    pub description: String,
}

/// 查看他人装备操作
#[derive(Debug, Clone)]
pub enum InspectAction {
    Close,
    InviteGroup,
    AddFriend,
    SendMail,
    RequestTrade,
}

/// 查看他人装备对话框
pub struct InspectDialogHybrid {
    /// 是否可见
    visible: bool,
    /// 窗口位置
    position: Vec2,

    // === 被查看的玩家信息 ===
    pub player_name: String,
    pub player_guild: String,
    pub player_level: u16,
    pub player_class: String,

    // === 装备数据 ===
    items: [Option<InspectEquipItem>; EQUIP_SLOTS],

    // === 纹理 ===
    bg_texture: BackgroundTexture,
    char_page_texture: Option<Texture2D>,
    char_page_size: Vec2,
    close_btn: ButtonTextures,
    group_btn: ButtonTextures,
    friend_btn: ButtonTextures,
    mail_btn: ButtonTextures,
    trade_btn: ButtonTextures,

    // === 拖动 ===
    drag_helper: DragHelper,

    // === 交互 ===
    hovered_slot: Option<usize>,
}

impl InspectDialogHybrid {
    pub fn new() -> Self {
        Self {
            visible: false,
            position: vec2(536.0, 0.0),

            player_name: String::new(),
            player_guild: String::new(),
            player_level: 0,
            player_class: String::new(),

            items: Default::default(),

            bg_texture: BackgroundTexture::new(),
            char_page_texture: None,
            char_page_size: Vec2::ZERO,
            close_btn: ButtonTextures::new(),
            group_btn: ButtonTextures::new(),
            friend_btn: ButtonTextures::new(),
            mail_btn: ButtonTextures::new(),
            trade_btn: ButtonTextures::new(),

            drag_helper: DragHelper::new(),

            hovered_slot: None,
        }
    }

    /// 加载纹理
    pub fn load_textures(&mut self) {
        println!("🔍 InspectDialog: 加载纹理...");

        // 背景 (Prguse[430])
        self.bg_texture = BackgroundTexture::load(LibraryName::Prguse, 430, None);

        // 角色页面 (Prguse[340])
        if let Some(info) = LibraryName::Prguse.get_texture(340) {
            self.char_page_size = vec2(info.width as f32, info.height as f32);
            self.char_page_texture = info.image;
        }

        // 关闭按钮 (Prguse2[360-362])
        self.close_btn = ButtonTextures::load_from_indices(LibraryName::Prguse2, [360, 361, 362]);

        // 互动按钮
        self.group_btn = ButtonTextures::load_from_indices(LibraryName::Prguse, [431, 432, 433]);
        self.friend_btn = ButtonTextures::load_from_indices(LibraryName::Prguse, [434, 435, 436]);
        self.mail_btn = ButtonTextures::load_from_indices(LibraryName::Prguse, [437, 438, 439]);
        self.trade_btn = ButtonTextures::load_from_indices(LibraryName::Prguse, [440, 441, 442]);

        println!("  ✅ 查看装备对话框纹理加载完成");
    }

    // === 公共 API ===

    pub fn visible(&self) -> bool {
        self.visible
    }

    pub fn set_visible(&mut self, visible: bool) {
        self.visible = visible;
    }

    /// 显示查看面板
    pub fn show_player(&mut self, name: &str, guild: &str, level: u16, class: &str) {
        self.player_name = name.to_string();
        self.player_guild = guild.to_string();
        self.player_level = level;
        self.player_class = class.to_string();
        self.items = Default::default();
        self.visible = true;
    }

    /// 设置装备
    pub fn set_item(&mut self, slot: usize, item: Option<InspectEquipItem>) {
        if slot < EQUIP_SLOTS {
            self.items[slot] = item;
        }
    }

    // === 绘制 ===

    pub fn draw(&mut self) -> Option<InspectAction> {
        if !self.visible {
            return None;
        }

        let mouse_pos = vec2(mouse_position().0, mouse_position().1);
        let mut action = None;

        // 窗口拖动
        let bg_size = self.bg_texture.size;
        let drag_rect = Rect::new(self.position.x, self.position.y, bg_size.x, bg_size.y);
        self.position = self.drag_helper.update(drag_rect, self.position, mouse_pos);
        let pos = self.position;

        // 背景
        self.bg_texture.draw(pos);

        // 角色页面（模型区域）
        if let Some(tex) = &self.char_page_texture {
            draw_texture_ex(
                tex,
                pos.x + 8.0,
                pos.y + 70.0,
                WHITE,
                DrawTextureParams::default(),
            );
        }

        // 玩家信息
        draw_text_cn(&self.player_name, pos.x + 80.0, pos.y + 22.0, 14.0, WHITE);
        if !self.player_guild.is_empty() {
            draw_text_cn(
                &format!("<{}>", self.player_guild),
                pos.x + 80.0,
                pos.y + 40.0,
                10.0,
                Color::new(0.7, 0.9, 1.0, 1.0),
            );
        }
        draw_text_cn(
            &format!("Lv.{} {}", self.player_level, self.player_class),
            pos.x + 80.0,
            pos.y + 55.0,
            10.0,
            Color::new(0.8, 0.8, 0.8, 1.0),
        );

        // 装备槽位
        self.hovered_slot = None;
        for i in 0..EQUIP_SLOTS {
            if let Some(slot_type) = InspectEquipSlot::from_index(i) {
                let (sx, sy) = slot_type.position();
                let slot_x = pos.x + sx;
                let slot_y = pos.y + sy;
                let slot_rect = Rect::new(slot_x, slot_y, 32.0, 32.0);

                // 槽位边框
                draw_rectangle_lines(slot_x, slot_y, 32.0, 32.0, 1.0, Color::new(0.4, 0.4, 0.4, 0.5));

                // 装备图标
                if let Some(item) = &self.items[i] {
                    if let Some(info) = LibraryName::Items.get_texture(item.icon_index) {
                        if let Some(tex) = &info.image {
                            draw_texture_ex(
                                tex,
                                slot_x,
                                slot_y,
                                WHITE,
                                DrawTextureParams {
                                    dest_size: Some(vec2(32.0, 32.0)),
                                    ..Default::default()
                                },
                            );
                        }
                    }
                } else {
                    // 空槽位标签
                    draw_text_cn(
                        slot_type.name(),
                        slot_x + 2.0,
                        slot_y + 20.0,
                        7.0,
                        Color::new(0.4, 0.4, 0.4, 0.5),
                    );
                }

                // 悬停
                if slot_rect.contains(mouse_pos) {
                    self.hovered_slot = Some(i);
                    draw_rectangle(slot_x, slot_y, 32.0, 32.0, Color::new(1.0, 1.0, 1.0, 0.15));
                }
            }
        }

        // 互动按钮行
        let btn_y = pos.y + 357.0;
        let btn_defs: [(f32, &ButtonTextures, InspectAction); 4] = [
            (55.0, &self.group_btn.clone(), InspectAction::InviteGroup),
            (85.0, &self.friend_btn.clone(), InspectAction::AddFriend),
            (115.0, &self.mail_btn.clone(), InspectAction::SendMail),
            (145.0, &self.trade_btn.clone(), InspectAction::RequestTrade),
        ];
        for (offset_x, btn_tex, btn_action) in &btn_defs {
            let bx = pos.x + offset_x;
            let btn_rect = Rect::new(bx, btn_y, btn_tex.size.x, btn_tex.size.y);
            let state = ButtonState::from_mouse(btn_rect, mouse_pos);
            btn_tex.draw(vec2(bx, btn_y), state);
            if ButtonState::is_clicked(btn_rect, mouse_pos) {
                action = Some(btn_action.clone());
            }
        }

        // 关闭按钮
        let close_x = pos.x + 241.0;
        let close_y = pos.y + 3.0;
        let close_rect = Rect::new(close_x, close_y, 20.0, 20.0);
        let close_state = ButtonState::from_mouse(close_rect, mouse_pos);
        self.close_btn.draw(vec2(close_x, close_y), close_state);
        if ButtonState::is_clicked(close_rect, mouse_pos) {
            action = Some(InspectAction::Close);
        }

        // 工具提示
        if let Some(idx) = self.hovered_slot {
            if let Some(item) = &self.items[idx] {
                let slot_name = InspectEquipSlot::from_index(idx)
                    .map(|s| s.name())
                    .unwrap_or("");
                let tooltip = format!("[{}] {}\n{}", slot_name, item.name, item.description);
                let tip_x = mouse_pos.x + 15.0;
                let tip_y = mouse_pos.y + 15.0;
                let lines: Vec<&str> = tooltip.lines().collect();
                let tip_w = 200.0;
                let tip_h = lines.len() as f32 * 16.0 + 8.0;

                draw_rectangle(tip_x, tip_y, tip_w, tip_h, Color::new(0.0, 0.0, 0.0, 0.85));
                draw_rectangle_lines(tip_x, tip_y, tip_w, tip_h, 1.0, Color::new(0.6, 0.6, 0.6, 0.8));
                for (j, line) in lines.iter().enumerate() {
                    draw_text_cn(
                        line,
                        tip_x + 6.0,
                        tip_y + 14.0 + j as f32 * 16.0,
                        12.0,
                        WHITE,
                    );
                }
            }
        }

        action
    }
}

// ============================================================================
// Tests
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_equip_slot_from_index() {
        assert_eq!(InspectEquipSlot::from_index(0), Some(InspectEquipSlot::Weapon));
        assert_eq!(InspectEquipSlot::from_index(13), Some(InspectEquipSlot::Mount));
        assert_eq!(InspectEquipSlot::from_index(14), None);
    }

    #[test]
    fn test_equip_slot_names() {
        assert_eq!(InspectEquipSlot::Weapon.name(), "武器");
        assert_eq!(InspectEquipSlot::Mount.name(), "坐骑");
    }

    #[test]
    fn test_inspect_dialog_show_player() {
        let mut dialog = InspectDialogHybrid::new();
        assert!(!dialog.visible());

        dialog.show_player("战士A", "联盟公会", 45, "战士");
        assert!(dialog.visible());
        assert_eq!(dialog.player_name, "战士A");
        assert_eq!(dialog.player_level, 45);
    }

    #[test]
    fn test_inspect_dialog_set_item() {
        let mut dialog = InspectDialogHybrid::new();
        dialog.set_item(0, Some(InspectEquipItem {
            icon_index: 100,
            name: "屠龙刀".to_string(),
            description: "攻击 50-80".to_string(),
        }));
        assert!(dialog.items[0].is_some());
        assert_eq!(dialog.items[0].as_ref().unwrap().name, "屠龙刀");

        // Out of bounds
        dialog.set_item(EQUIP_SLOTS + 1, Some(InspectEquipItem {
            icon_index: 0,
            name: "test".to_string(),
            description: String::new(),
        }));
    }
}
