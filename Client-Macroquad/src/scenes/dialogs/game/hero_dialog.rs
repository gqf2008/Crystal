// ============================================================================
// HeroDialogHybrid - 英雄对话框（纯 Native 版本）
// ============================================================================
//
// C# 参考：Client/MirScenes/Dialogs/HeroDialogs.cs (895 行)
// + Client/MirScenes/Dialogs/CharacterDialog.cs (711 行, 复用为 HeroDialog)
//
// C# 中英雄 UI 拆分为：
// - HeroInfoPanel (info background 14, avatar 1400, HP/MP/Exp bars)
// - HeroBehaviourPanel (buttons 1840-1843, Prguse)
// - HeroMenuPanel (background 2179, 3 buttons: skills/inventory/equipment)
// - HeroInventoryDialog (background 1422, 8x5 grid)
// - HeroBeltDialog (background 1921, 2-slot)
// - CharacterDialog (background 504, Title) -> 用作 HeroDialog (skills/status/equipment)
//
// ============================================================================

use macroquad::prelude::*;
use crate::resources::LibraryName;
use crate::ui::text_renderer::draw_text_cn;
use super::native_ui_utils::DragHelper;
use super::character_dialog::SkillInfo;

/// 英雄行为模式
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum HeroBehaviour {
    Attack = 0,
    CounterAttack = 1,
    Follow = 2,
    Custom = 3,
}

impl TryFrom<u8> for HeroBehaviour {
    type Error = ();
    fn try_from(v: u8) -> Result<Self, Self::Error> {
        match v {
            0 => Ok(HeroBehaviour::Attack),
            1 => Ok(HeroBehaviour::CounterAttack),
            2 => Ok(HeroBehaviour::Follow),
            3 => Ok(HeroBehaviour::Custom),
            _ => Err(()),
        }
    }
}

/// 英雄信息
#[derive(Debug, Clone)]
pub struct HeroInfo {
    pub name: String,
    pub level: u16,
    pub class: u8,
    pub gender: u8,
    pub current_hp: i32,
    pub max_hp: i32,
    pub current_mp: i32,
    pub max_mp: i32,
    pub current_exp: i64,
    pub max_exp: i64,
    pub is_alive: bool,
    pub is_dangerous: bool, // HP < 20%
}

/// 英雄对话框动作
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum HeroDialogAction {
    None,
    SetBehaviour(HeroBehaviour),
    ChangeHero(u8),
    SetAutoHpPot { value: u32 },
    SetAutoMpPot { value: u32 },
}

/// 英雄管理列表条目 (from ManageHeroes packet)
#[derive(Debug, Clone)]
pub struct ManageHeroEntry {
    pub index: i32,
    pub name: String,
    pub level: u16,
    pub class: u8,
    pub gender: u8,
}

const HERO_SPAWN_UNSUMMONED: u8 = 1;
const HERO_SPAWN_DEAD: u8 = 3;

pub struct HeroDialogHybrid {
    position: Vec2,
    visible: bool,
    size: Vec2,
    hero_info: Option<HeroInfo>,
    current_behaviour: HeroBehaviour,
    bg_texture: Option<Texture2D>,
    drag_helper: DragHelper,
    pending_action: HeroDialogAction,
    // Info panel textures
    info_bg_texture: Option<Texture2D>,
    name_bg_texture: Option<Texture2D>,
    // Behaviour button textures
    behaviour_bg_textures: [Option<Texture2D>; 4],
    // Auto-pot state
    auto_pot_unlocked: bool,
    auto_hp_pot_value: u32,
    auto_mp_pot_value: u32,
    auto_hp_pot_item_id: u32,
    auto_mp_pot_item_id: u32,
    // Hero manage list (from ManageHeroes packet)
    manage_heroes: Vec<ManageHeroEntry>,
    // Current hero ID
    hero_id: u32,
    // Base stats (from HeroBaseStatsInfo packet)
    base_stats: Vec<i32>,
    // 技能列表（来自 MagicLearned / MagicLeveledUp / MagicRemoved）
    skills: Vec<SkillInfo>,
}

impl Default for HeroDialogHybrid {
    fn default() -> Self {
        Self::new()
    }
}

impl HeroDialogHybrid {
    const BEHAVIOUR_BTN_Y: f32 = 37.0;
    const BEHAVIOUR_BTN_START_X: f32 = 165.0;

    pub fn new() -> Self {
        Self {
            position: vec2(200.0, 50.0),
            visible: false,
            size: vec2(320.0, 250.0),
            hero_info: None,
            current_behaviour: HeroBehaviour::Follow,
            bg_texture: None,
            drag_helper: DragHelper::new(),
            pending_action: HeroDialogAction::None,
            info_bg_texture: None,
            name_bg_texture: None,
            behaviour_bg_textures: [None, None, None, None],
            auto_pot_unlocked: false,
            auto_hp_pot_value: 0,
            auto_mp_pot_value: 0,
            auto_hp_pot_item_id: 0,
            auto_mp_pot_item_id: 0,
            manage_heroes: Vec::new(),
            hero_id: 0,
            base_stats: Vec::new(),
            skills: Vec::new(),
        }
    }

    pub fn open(&mut self) {
        self.visible = true;
    }

    pub fn close(&mut self) {
        self.visible = false;
    }

    pub fn toggle(&mut self) {
        self.visible = !self.visible;
    }

    pub fn is_visible(&self) -> bool {
        self.visible
    }

    pub fn set_position(&mut self, pos: Vec2) {
        self.position = pos;
    }

    pub fn get_position(&self) -> Vec2 {
        self.position
    }

    pub fn contains(&self, point: Vec2) -> bool {
        if !self.visible {
            return false;
        }
        Rect::new(self.position.x, self.position.y, self.size.x, self.size.y).contains(point)
    }

    /// 更新英雄信息
    pub fn update_hero_info(&mut self, info: HeroInfo) {
        self.hero_info = Some(info);
    }

    /// 更新英雄行为模式
    pub fn set_behaviour(&mut self, behaviour: HeroBehaviour) {
        self.current_behaviour = behaviour;
    }

    /// 更新英雄等级
    pub fn update_hero_level(&mut self, new_level: u16) {
        if let Some(info) = &mut self.hero_info {
            info.level = new_level;
        }
    }

    /// 更新英雄 HP/MP
    pub fn update_health(&mut self, hp: i32, mp: i32) {
        if let Some(info) = &mut self.hero_info {
            info.current_hp = hp;
            info.current_mp = mp;
            info.is_dangerous = info.max_hp > 0 && (hp as f32 / info.max_hp as f32) < 0.2;
            info.is_alive = hp > 0;
        } else {
            self.hero_info = Some(HeroInfo {
                name: "英雄".to_string(),
                level: 1,
                class: 0,
                gender: 0,
                current_hp: hp,
                max_hp: hp.max(1),
                current_mp: mp,
                max_mp: mp.max(1),
                current_exp: 0,
                max_exp: 0,
                is_alive: hp > 0,
                is_dangerous: false,
            });
        }
    }

    /// 更新英雄召唤状态
    pub fn set_spawn_state(&mut self, state: u8) {
        match state {
            HERO_SPAWN_UNSUMMONED => self.hero_info = None,
            HERO_SPAWN_DEAD => {
                if let Some(info) = &mut self.hero_info {
                    info.is_alive = false;
                }
            }
            _ => {}
        }
    }

    /// 自动喝药解锁
    pub fn set_auto_pot_unlocked(&mut self, unlocked: bool) {
        self.auto_pot_unlocked = unlocked;
    }

    /// 设置自动喝药阈值
    pub fn set_auto_pot_value(&mut self, pot_type: u8, value: u32) {
        if pot_type == 0 {
            self.auto_hp_pot_value = value;
        } else {
            self.auto_mp_pot_value = value;
        }
    }

    /// 设置自动喝药物品
    pub fn set_auto_pot_item(&mut self, slot: i32, item_id: u32) {
        if slot == 0 {
            self.auto_hp_pot_item_id = item_id;
        } else {
            self.auto_mp_pot_item_id = item_id;
        }
    }

    /// 清除英雄信息（英雄被移除）
    pub fn clear_hero_info(&mut self) {
        self.hero_info = None;
    }

    /// 更新英雄管理列表 (from ManageHeroes packet)
    pub fn update_manage_list(&mut self, heroes: Vec<ManageHeroEntry>) {
        self.manage_heroes = heroes;
    }

    /// 设置当前英雄ID
    pub fn set_hero_id(&mut self, hero_id: u32) {
        self.hero_id = hero_id;
    }

    /// 设置英雄基础属性 (from HeroBaseStatsInfo)
    pub fn set_base_stats(&mut self, stats: Vec<i32>) {
        self.base_stats = stats;
    }

    /// 英雄学会/更新技能
    pub fn learn_skill(&mut self, spell_id: u8, name: String, level: u8, icon: u8) {
        if let Some(existing) = self.skills.iter_mut().find(|s| s.spell_id == spell_id) {
            existing.level = level;
            existing.name = name;
            existing.icon_index = icon as usize;
            return;
        }
        self.skills.push(SkillInfo { spell_id, name, level, icon_index: icon as usize, can_use: true });
    }

    /// 英雄技能升级
    pub fn level_up_skill(&mut self, spell_id: u8, level: u8) {
        if let Some(skill) = self.skills.iter_mut().find(|s| s.spell_id == spell_id) {
            skill.level = level;
        }
    }

    /// 英雄移除技能
    pub fn remove_skill(&mut self, spell_id: u8) {
        self.skills.retain(|s| s.spell_id != spell_id);
    }

    /// 切换英雄技能可用状态
    pub fn toggle_skill(&mut self, spell_id: u8, can_use: bool) {
        if let Some(skill) = self.skills.iter_mut().find(|s| s.spell_id == spell_id) {
            skill.can_use = can_use;
        }
    }

    /// 获取待处理动作
    pub fn take_action(&mut self) -> HeroDialogAction {
        std::mem::replace(&mut self.pending_action, HeroDialogAction::None)
    }

    /// 加载纹理
    pub fn load_textures(&mut self) {
        // 主背景 - CharacterDialog: Title[504]
        if let Some(texture) = LibraryName::Title.get_texture(504) {
            if let Some(tex) = texture.image {
                self.bg_texture = Some(tex);
                self.size = vec2(texture.width as f32, texture.height as f32);
            }
        }

        // Info panel: Prguse[14]
        if let Some(texture) = LibraryName::Prguse.get_texture(14) {
            if let Some(tex) = texture.image {
                self.info_bg_texture = Some(tex);
            }
        }

        // Name background: Prguse[10]
        if let Some(texture) = LibraryName::Prguse.get_texture(10) {
            if let Some(tex) = texture.image {
                self.name_bg_texture = Some(tex);
            }
        }

        // Behaviour buttons: Prguse[1840-1843]
        for (i, idx) in [1840, 1841, 1842, 1843].iter().enumerate() {
            if let Some(texture) = LibraryName::Prguse.get_texture(*idx) {
                if let Some(tex) = texture.image {
                    self.behaviour_bg_textures[i] = Some(tex);
                }
            }
        }
    }

    pub fn update_and_draw(&mut self) {
        if !self.visible {
            return;
        }

        let mouse_pos = vec2(mouse_position().0, mouse_position().1);

        // 拖拽
        let drag_area = Rect::new(self.position.x, self.position.y, self.size.x, 30.0);
        self.drag_helper.apply(drag_area, &mut self.position);

        // 绘制背景
        self.draw_background();

        // 绘制英雄信息
        self.draw_hero_info(mouse_pos);

        // 绘制行为按钮
        self.draw_behaviour_buttons(mouse_pos);
    }

    fn draw_background(&self) {
        if let Some(texture) = &self.bg_texture {
            draw_texture_ex(
                texture,
                self.position.x,
                self.position.y,
                WHITE,
                DrawTextureParams::default(),
            );
        }
    }

    fn draw_hero_info(&mut self, _mouse_pos: Vec2) {
        let Some(info) = &self.hero_info else {
            // 显示"无英雄"提示
            draw_text_cn(
                "暂无英雄",
                self.position.x + 100.0,
                self.position.y + 100.0,
                14.0,
                GRAY,
            );
            return;
        };

        // Info panel background
        if let Some(tex) = &self.info_bg_texture {
            draw_texture_ex(
                tex,
                self.position.x + 95.0,
                self.position.y + 48.0,
                WHITE,
                DrawTextureParams::default(),
            );
        }

        // Name background
        if let Some(tex) = &self.name_bg_texture {
            draw_texture_ex(
                tex,
                self.position.x + 95.0 + 26.0,
                self.position.y + 48.0 + 60.0,
                WHITE,
                DrawTextureParams::default(),
            );
        }

        // Hero name & level
        draw_text_cn(
            &info.name,
            self.position.x + 95.0 + 30.0,
            self.position.y + 48.0 + 64.0,
            12.0,
            WHITE,
        );
        draw_text_cn(
            &format!("Lv.{}", info.level),
            self.position.x + 95.0 + 30.0,
            self.position.y + 48.0 + 78.0,
            10.0,
            Color::from_rgba(200, 200, 100, 255),
        );

        // HP bar
        let bar_x = self.position.x + 95.0 + 57.0;
        let bar_y = self.position.y + 48.0 + 26.0;
        self.draw_bar(
            bar_x, bar_y,
            info.max_hp as i64, info.current_hp as i64,
            "HP",
        );

        // MP bar
        let mp_y = bar_y + 18.0;
        self.draw_bar(
            bar_x, mp_y,
            info.max_mp as i64, info.current_mp as i64,
            "MP",
        );

        // Exp bar
        let exp_y = mp_y + 18.0;
        self.draw_bar(
            bar_x, exp_y,
            {
                let m = info.max_exp;
                if m == 0 { 1 } else { m }
            },
            info.current_exp,
            "EXP",
        );

        // Dead/Danger overlay
        if !info.is_alive {
            draw_text_cn(
                "已死亡",
                self.position.x + 95.0 + 40.0,
                self.position.y + 48.0 + 10.0,
                14.0,
                Color::from_rgba(255, 80, 80, 255),
            );
        } else if info.is_dangerous {
            draw_text_cn(
                "危险!",
                self.position.x + 95.0 + 40.0,
                self.position.y + 48.0 + 10.0,
                12.0,
                Color::from_rgba(255, 165, 0, 255),
            );
        }
    }

    fn draw_bar(&self, x: f32, y: f32, max_val: i64, current_val: i64, label: &str) {
        let bar_w = 120.0;
        let bar_h = 14.0;
        let pct = if max_val > 0 { current_val as f32 / max_val as f32 } else { 0.0 };
        let draw_w = (bar_w * pct).clamp(0.0, bar_w);
        draw_rectangle(x, y, bar_w, bar_h, Color::from_rgba(40, 40, 40, 200));
        if draw_w > 0.0 {
            let color = if label == "HP" {
                Color::from_rgba(200, 60, 60, 200)
            } else if label == "MP" {
                Color::from_rgba(60, 60, 200, 200)
            } else {
                Color::from_rgba(200, 200, 60, 200)
            };
            draw_rectangle(x, y, draw_w, bar_h, color);
        }
        draw_rectangle_lines(x, y, bar_w, bar_h, 1.0, Color::from_rgba(100, 100, 100, 200));

        // 标签文字
        let label_color = if label == "HP" {
            Color::from_rgba(255, 100, 100, 255)
        } else if label == "MP" {
            Color::from_rgba(100, 100, 255, 255)
        } else {
            Color::from_rgba(255, 255, 100, 255)
        };
        draw_text_cn(label, x - 22.0, y + 10.0, 10.0, label_color);
    }

    fn draw_behaviour_buttons(&mut self, mouse_pos: Vec2) {
        let btn_y = self.position.y + Self::BEHAVIOUR_BTN_Y;
        let btn_spacing = 40.0;
        let labels = ["攻击", "反击", "跟随", "自定义"];
        let behaviours = [HeroBehaviour::Attack, HeroBehaviour::CounterAttack, HeroBehaviour::Follow, HeroBehaviour::Custom];

        let mut clicked: Option<HeroBehaviour> = None;

        for i in 0..4 {
            let btn_x = self.position.x + Self::BEHAVIOUR_BTN_START_X + i as f32 * btn_spacing;
            let is_active = self.current_behaviour == behaviours[i];
            let is_hovered = Rect::new(btn_x, btn_y, 36.0, 20.0).contains(mouse_pos);
            let is_pressed = is_hovered && is_mouse_button_down(MouseButton::Left);

            // 按钮背景
            let bg_color = if is_active {
                Color::from_rgba(80, 100, 120, 255)
            } else if is_pressed {
                Color::from_rgba(60, 70, 90, 255)
            } else if is_hovered {
                Color::from_rgba(70, 80, 100, 255)
            } else {
                Color::from_rgba(50, 50, 70, 255)
            };
            draw_rectangle(btn_x, btn_y, 36.0, 20.0, bg_color);
            draw_rectangle_lines(btn_x, btn_y, 36.0, 20.0, 1.0, Color::from_rgba(100, 100, 120, 200));

            draw_text_cn(labels[i], btn_x + 6.0, btn_y + 13.0, 10.0, WHITE);

            if is_hovered && is_mouse_button_pressed(MouseButton::Left) {
                clicked = Some(behaviours[i]);
            }
        }

        if let Some(behaviour) = clicked {
            self.current_behaviour = behaviour;
            self.pending_action = HeroDialogAction::SetBehaviour(behaviour);
        }
    }
}
