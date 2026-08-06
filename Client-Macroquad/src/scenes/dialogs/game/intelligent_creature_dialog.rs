// ============================================================================
// IntelligentCreatureDialogHybrid - 智能宠物对话框（纯 Native 版本）
// ============================================================================
//
// C# 参考：Client/MirScenes/Dialogs/IntelligentCreatureDialogs.cs (1,389 行)
// - Background: Title[468]
// - 10 个宠物槽位 (2x5), 每个显示宠物图标 + 饱满度
// - 召唤/解散/放生按钮
// - 自动/半自动模式切换
// - 选项子对话框
//
// ============================================================================

use super::native_ui_utils::DragHelper;
use crate::resources::LibraryName;
use crate::ui::text_renderer::draw_text_cn;
use macroquad::prelude::*;

/// 宠物模式
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CreatureMode {
    Automatic,
    SemiAutomatic,
}

/// 单个宠物数据
#[derive(Debug, Clone)]
pub struct CreatureEntry {
    pub name: String,
    pub creature_type: u8,
    pub fullness: u8,
    pub max_fullness: u8,
    pub is_summoned: bool,
    pub pearl_count: u32,
    pub deadline_days: i32,
}

/// 智能宠物对话框动作
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CreatureDialogAction {
    None,
    SummonCreature(usize),
    DismissCreature,
    ReleaseCreature(usize),
    ToggleMode,
    OpenOptions,
}

pub struct IntelligentCreatureDialogHybrid {
    position: Vec2,
    visible: bool,
    size: Vec2,
    mode: CreatureMode,
    selected_creature: Option<usize>,
    creatures: Vec<CreatureEntry>,
    drag_helper: DragHelper,
    pending_action: CreatureDialogAction,
    // 纹理
    bg_texture: Option<Texture2D>,
    close_texture: Option<Texture2D>,
    summon_texture: Option<Texture2D>,
    summon_alt_texture: Option<Texture2D>,
    dismiss_texture: Option<Texture2D>,
    release_texture: Option<Texture2D>,
    options_texture: Option<Texture2D>,
    auto_mode_texture: Option<Texture2D>,
    semi_auto_texture: Option<Texture2D>,
    fullness_bg_texture: Option<Texture2D>,
    fullness_fg_texture: Option<Texture2D>,
    pearl_texture: Option<Texture2D>,
    // Server-driven state
    can_rename: bool,
    auto_pickup: bool,
}

impl Default for IntelligentCreatureDialogHybrid {
    fn default() -> Self {
        Self::new()
    }
}

impl IntelligentCreatureDialogHybrid {
    const SLOT_W: f32 = 50.0;
    const SLOT_H: f32 = 60.0;
    const SLOT_COLS: usize = 5;
    const SLOT_START_X: f32 = 15.0;
    const SLOT_START_Y: f32 = 40.0;
    const SLOT_GAP: f32 = 8.0;

    pub fn new() -> Self {
        Self {
            position: vec2(200.0, 80.0),
            visible: false,
            size: vec2(320.0, 300.0),
            mode: CreatureMode::SemiAutomatic,
            selected_creature: None,
            creatures: Vec::new(),
            drag_helper: DragHelper::new(),
            pending_action: CreatureDialogAction::None,
            bg_texture: None,
            close_texture: None,
            summon_texture: None,
            summon_alt_texture: None,
            dismiss_texture: None,
            release_texture: None,
            options_texture: None,
            auto_mode_texture: None,
            semi_auto_texture: None,
            fullness_bg_texture: None,
            fullness_fg_texture: None,
            pearl_texture: None,
            can_rename: false,
            auto_pickup: false,
        }
    }

    pub fn open(&mut self) {
        self.visible = true;
    }

    pub fn close(&mut self) {
        self.visible = false;
        self.selected_creature = None;
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

    /// 更新宠物列表
    pub fn update_creatures(&mut self, creatures: Vec<CreatureEntry>) {
        self.creatures = creatures;
    }

    /// 更新模式
    pub fn set_mode(&mut self, mode: CreatureMode) {
        self.mode = mode;
    }

    /// 获取当前模式
    pub fn get_mode(&self) -> CreatureMode {
        self.mode
    }

    /// 获取待处理动作
    pub fn take_action(&mut self) -> CreatureDialogAction {
        std::mem::replace(&mut self.pending_action, CreatureDialogAction::None)
    }

    pub fn set_can_rename(&mut self, can_rename: bool) {
        self.can_rename = can_rename;
    }

    pub fn set_auto_pickup(&mut self, enabled: bool) {
        self.auto_pickup = enabled;
    }

    /// 加载纹理
    pub fn load_textures(&mut self) {
        // 主背景: Title[468]
        if let Some(texture) = LibraryName::Title.get_texture(468) {
            if let Some(tex) = texture.image {
                self.bg_texture = Some(tex);
                self.size = vec2(texture.width as f32, texture.height as f32);
            }
        }

        // 关闭按钮: Prguse2[360]
        if let Some(texture) = LibraryName::Prguse2.get_texture(360) {
            self.close_texture = texture.image;
        }

        // 召唤: Title[576]
        if let Some(texture) = LibraryName::Title.get_texture(576) {
            self.summon_texture = texture.image;
        }
        // 召唤替代（已有召唤时）: Title[593]
        if let Some(texture) = LibraryName::Title.get_texture(593) {
            self.summon_alt_texture = texture.image;
        }

        // 解散: Title[580]
        if let Some(texture) = LibraryName::Title.get_texture(580) {
            self.dismiss_texture = texture.image;
        }

        // 放生: Title[583]
        if let Some(texture) = LibraryName::Title.get_texture(583) {
            self.release_texture = texture.image;
        }

        // 选项菜单: Title[573]
        if let Some(texture) = LibraryName::Title.get_texture(573) {
            self.options_texture = texture.image;
        }

        // 自动模式: Title[610]
        if let Some(texture) = LibraryName::Title.get_texture(610) {
            self.auto_mode_texture = texture.image;
        }

        // 半自动模式: Title[613]
        if let Some(texture) = LibraryName::Title.get_texture(613) {
            self.semi_auto_texture = texture.image;
        }

        // 饱满度条: Prguse2[530/531]
        if let Some(texture) = LibraryName::Prguse2.get_texture(530) {
            self.fullness_bg_texture = texture.image;
        }
        if let Some(texture) = LibraryName::Prguse2.get_texture(531) {
            self.fullness_fg_texture = texture.image;
        }

        // 珍珠: Prguse2[427]
        if let Some(texture) = LibraryName::Prguse2.get_texture(427) {
            self.pearl_texture = texture.image;
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

        // 绘制宠物槽位
        self.draw_creature_slots(mouse_pos);

        // 绘制操作按钮
        self.draw_action_buttons(mouse_pos);

        // 绘制模式切换
        self.draw_mode_toggle(mouse_pos);
    }

    fn draw_background(&self) {
        if let Some(tex) = &self.bg_texture {
            draw_texture_ex(
                tex,
                self.position.x,
                self.position.y,
                WHITE,
                DrawTextureParams::default(),
            );
        }
    }

    fn draw_creature_slots(&mut self, mouse_pos: Vec2) {
        let mut clicked: Option<usize> = None;

        for i in 0..10 {
            let col = i % Self::SLOT_COLS;
            let row = i / Self::SLOT_COLS;
            let slot_x =
                self.position.x + Self::SLOT_START_X + col as f32 * (Self::SLOT_W + Self::SLOT_GAP);
            let slot_y =
                self.position.y + Self::SLOT_START_Y + row as f32 * (Self::SLOT_H + Self::SLOT_GAP);
            let slot_rect = Rect::new(slot_x, slot_y, Self::SLOT_W, Self::SLOT_H);

            let is_selected = self.selected_creature == Some(i);
            let is_hovered = slot_rect.contains(mouse_pos);
            let has_creature = i < self.creatures.len();

            // 槽位边框
            let border_color = if is_selected {
                Color::from_rgba(100, 150, 200, 255)
            } else if is_hovered && has_creature {
                Color::from_rgba(200, 200, 100, 255)
            } else {
                Color::from_rgba(80, 80, 80, 200)
            };
            draw_rectangle_lines(
                slot_x,
                slot_y,
                Self::SLOT_W,
                Self::SLOT_H,
                1.0,
                border_color,
            );

            // 槽位背景
            draw_rectangle(
                slot_x + 1.0,
                slot_y + 1.0,
                Self::SLOT_W - 2.0,
                Self::SLOT_H - 2.0,
                Color::from_rgba(30, 30, 30, 150),
            );

            if has_creature {
                let creature = &self.creatures[i];

                // 宠物名称
                draw_text_cn(&creature.name, slot_x + 2.0, slot_y + 10.0, 9.0, WHITE);

                // 召唤状态标记
                if creature.is_summoned {
                    draw_text_cn(
                        "召",
                        slot_x + 2.0,
                        slot_y + 20.0,
                        8.0,
                        Color::from_rgba(100, 255, 100, 255),
                    );
                }

                // 饱满度条
                let bar_x = slot_x + 4.0;
                let bar_y = slot_y + 32.0;
                let bar_w = Self::SLOT_W - 8.0;
                let bar_h = 6.0;
                let fullness_pct = if creature.max_fullness > 0 {
                    creature.fullness as f32 / creature.max_fullness as f32
                } else {
                    0.0
                };
                draw_rectangle(
                    bar_x,
                    bar_y,
                    bar_w,
                    bar_h,
                    Color::from_rgba(40, 40, 40, 200),
                );
                if fullness_pct > 0.0 {
                    let bar_color = if fullness_pct > 0.5 {
                        Color::from_rgba(60, 180, 60, 200)
                    } else if fullness_pct > 0.2 {
                        Color::from_rgba(200, 180, 60, 200)
                    } else {
                        Color::from_rgba(200, 60, 60, 200)
                    };
                    draw_rectangle(bar_x, bar_y, bar_w * fullness_pct, bar_h, bar_color);
                }
                draw_rectangle_lines(
                    bar_x,
                    bar_y,
                    bar_w,
                    bar_h,
                    0.5,
                    Color::from_rgba(80, 80, 80, 150),
                );

                // 饱满度数值
                draw_text_cn(
                    &format!("{}", creature.fullness),
                    slot_x + 2.0,
                    slot_y + 50.0,
                    8.0,
                    GRAY,
                );
            } else {
                draw_text_cn(
                    "空",
                    slot_x + 16.0,
                    slot_y + 20.0,
                    10.0,
                    Color::from_rgba(60, 60, 60, 200),
                );
            }

            // 点击检测
            if is_hovered && has_creature && is_mouse_button_pressed(MouseButton::Left) {
                clicked = Some(i);
            }
        }

        if let Some(idx) = clicked {
            self.selected_creature = Some(idx);
        }

        // 珍珠显示
        if let Some(creature) = self.selected_creature.and_then(|i| self.creatures.get(i)) {
            let pearl_x = self.position.x + 200.0;
            let pearl_y = self.position.y + 40.0;
            if let Some(tex) = &self.pearl_texture {
                draw_texture_ex(tex, pearl_x, pearl_y, WHITE, DrawTextureParams::default());
            }
            draw_text_cn(
                &format!("x{}", creature.pearl_count),
                pearl_x + 16.0,
                pearl_y + 4.0,
                10.0,
                WHITE,
            );
        }
    }

    fn draw_action_buttons(&mut self, mouse_pos: Vec2) {
        let btn_y = self.position.y + 200.0;
        let btn_w = 60.0;
        let btn_h = 24.0;
        let btn_gap = 10.0;

        let buttons: Vec<(&str, CreatureDialogAction, Color)> = vec![
            (
                "召唤",
                CreatureDialogAction::SummonCreature(self.selected_creature.unwrap_or(0)),
                Color::from_rgba(60, 150, 60, 200),
            ),
            (
                "解散",
                CreatureDialogAction::DismissCreature,
                Color::from_rgba(150, 100, 60, 200),
            ),
            (
                "放生",
                CreatureDialogAction::ReleaseCreature(self.selected_creature.unwrap_or(0)),
                Color::from_rgba(150, 60, 60, 200),
            ),
        ];

        let total_w = buttons.len() as f32 * (btn_w + btn_gap) - btn_gap;
        let start_x = self.position.x + (self.size.x - total_w) / 2.0;

        for (i, (label, action, color)) in buttons.iter().enumerate() {
            let btn_x = start_x + i as f32 * (btn_w + btn_gap);
            let btn_rect = Rect::new(btn_x, btn_y, btn_w, btn_h);
            let is_hovered = btn_rect.contains(mouse_pos);
            let is_pressed = is_hovered && is_mouse_button_down(MouseButton::Left);

            let btn_color = if is_pressed {
                Color::from_rgba(100, 120, 140, 255)
            } else if is_hovered {
                lighten_color(*color, 30)
            } else {
                *color
            };
            draw_rectangle(btn_x, btn_y, btn_w, btn_h, btn_color);
            draw_rectangle_lines(
                btn_x,
                btn_y,
                btn_w,
                btn_h,
                1.0,
                Color::from_rgba(100, 100, 120, 200),
            );

            draw_text_cn(label, btn_x + 12.0, btn_y + 15.0, 11.0, WHITE);

            if is_hovered && is_mouse_button_pressed(MouseButton::Left) {
                self.pending_action = *action;
            }
        }

        // 选项按钮
        let opt_x = self.position.x + self.size.x - 40.0;
        let opt_y = self.position.y + 200.0;
        let opt_rect = Rect::new(opt_x, opt_y, 30.0, 24.0);
        let is_opt_hovered = opt_rect.contains(mouse_pos);

        if let Some(tex) = &self.options_texture {
            draw_texture_ex(tex, opt_x, opt_y, WHITE, DrawTextureParams::default());
        } else {
            draw_rectangle(opt_x, opt_y, 30.0, 24.0, Color::from_rgba(70, 70, 90, 200));
            draw_text_cn("选项", opt_x + 2.0, opt_y + 14.0, 10.0, WHITE);
        }

        if is_opt_hovered && is_mouse_button_pressed(MouseButton::Left) {
            self.pending_action = CreatureDialogAction::OpenOptions;
        }
    }

    fn draw_mode_toggle(&mut self, mouse_pos: Vec2) {
        let toggle_x = self.position.x + 20.0;
        let toggle_y = self.position.y + 200.0;
        let toggle_w = 60.0;
        let toggle_h = 24.0;
        let toggle_rect = Rect::new(toggle_x, toggle_y, toggle_w, toggle_h);
        let is_hovered = toggle_rect.contains(mouse_pos);
        let is_pressed = is_hovered && is_mouse_button_down(MouseButton::Left);

        let mode_label = match self.mode {
            CreatureMode::Automatic => "自动",
            CreatureMode::SemiAutomatic => "半自动",
        };

        let bg_color = if is_pressed {
            Color::from_rgba(100, 120, 140, 255)
        } else if is_hovered {
            Color::from_rgba(80, 100, 120, 255)
        } else {
            Color::from_rgba(60, 70, 90, 255)
        };
        draw_rectangle(toggle_x, toggle_y, toggle_w, toggle_h, bg_color);
        draw_rectangle_lines(
            toggle_x,
            toggle_y,
            toggle_w,
            toggle_h,
            1.0,
            Color::from_rgba(100, 100, 120, 200),
        );
        draw_text_cn(mode_label, toggle_x + 12.0, toggle_y + 15.0, 11.0, WHITE);

        if is_hovered && is_mouse_button_pressed(MouseButton::Left) {
            let new_mode = match self.mode {
                CreatureMode::Automatic => CreatureMode::SemiAutomatic,
                CreatureMode::SemiAutomatic => CreatureMode::Automatic,
            };
            self.mode = new_mode;
            self.pending_action = CreatureDialogAction::ToggleMode;
        }
    }
}

fn lighten_color(color: Color, amount: u8) -> Color {
    Color {
        r: (color.r + amount as f32 / 255.0).min(1.0),
        g: (color.g + amount as f32 / 255.0).min(1.0),
        b: (color.b + amount as f32 / 255.0).min(1.0),
        a: color.a,
    }
}
