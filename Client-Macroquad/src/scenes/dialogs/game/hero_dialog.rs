// ============================================================================
// HeroDialogHybrid - 英雄系统对话框（对齐 C# HeroDialogs.cs）
// ============================================================================
//
// C# 参考：Client/MirScenes/Dialogs/HeroDialogs.cs
// - HeroManageDialog: 英雄头像选择（8 格），召唤/解散/管理按钮，英雄信息
// - HeroInventoryDialog: 46 格背包（类似 inventory），英雄装备面板
// - HeroBeltDialog: 6 格快捷栏
//
// ============================================================================

use macroquad::prelude::*;
use super::native_ui_utils::*;

// ============================================================================
// 常量
// ============================================================================

const HERO_SLOTS: usize = 8;
const HERO_INVENTORY_SLOTS: usize = 46;
const HERO_BELT_SLOTS: usize = 6;
const HERO_INV_COLS: usize = 6;
const CELL_SIZE: f32 = 34.0;
const CELL_GAP: f32 = 2.0;

const MANAGE_WIDTH: f32 = 264.0;
const MANAGE_HEIGHT: f32 = 220.0;
const INV_WIDTH: f32 = 264.0;
const INV_HEIGHT: f32 = 360.0;
const BELT_WIDTH: f32 = 220.0;
const BELT_HEIGHT: f32 = 50.0;

// ============================================================================
// 类型定义
// ============================================================================

/// 英雄信息
#[derive(Debug, Clone)]
pub struct HeroInfo {
    pub name: String,
    pub level: u16,
    pub class_id: u8,
    pub hp: u32,
    pub max_hp: u32,
}

impl HeroInfo {
    pub fn new(name: &str, level: u16, class_id: u8, hp: u32, max_hp: u32) -> Self {
        Self {
            name: name.to_string(),
            level,
            class_id,
            hp,
            max_hp,
        }
    }

    /// HP 百分比 (0.0 ~ 1.0)
    pub fn hp_percent(&self) -> f32 {
        if self.max_hp == 0 {
            0.0
        } else {
            self.hp as f32 / self.max_hp as f32
        }
    }
}

/// 英雄操作事件
#[derive(Debug, Clone, PartialEq)]
pub enum HeroAction {
    /// 召唤英雄
    Summon(usize),
    /// 解散英雄
    Dismiss,
    /// 管理英雄
    ManageHero(usize),
    /// 移动物品
    MoveItem { from: usize, to: usize },
    /// 使用物品
    UseItem(usize),
    /// 关闭
    Close,
}

// ============================================================================
// HeroManageDialogHybrid
// ============================================================================

/// 英雄管理对话框
pub struct HeroManageDialogHybrid {
    pub visible: bool,
    pub heroes: Vec<Option<HeroInfo>>,
    pub selected_index: Option<usize>,
    pub active_hero: Option<usize>,
    position: Vec2,
    drag_helper: DragHelper,
}

impl HeroManageDialogHybrid {
    pub fn new() -> Self {
        let mut heroes = Vec::with_capacity(HERO_SLOTS);
        for _ in 0..HERO_SLOTS {
            heroes.push(None);
        }
        Self {
            visible: false,
            heroes,
            selected_index: None,
            active_hero: None,
            position: Vec2::new(300.0, 150.0),
            drag_helper: DragHelper::new(),
        }
    }

    /// 设置英雄到指定槽位
    pub fn set_hero(&mut self, slot: usize, hero: Option<HeroInfo>) {
        if slot < HERO_SLOTS {
            self.heroes[slot] = hero;
        }
    }

    /// 获取当前活跃英雄信息
    pub fn get_active_hero(&self) -> Option<&HeroInfo> {
        self.active_hero.and_then(|idx| self.heroes.get(idx).and_then(|h| h.as_ref()))
    }

    /// 绘制并处理输入
    pub fn draw(&mut self) -> Option<HeroAction> {
        if !self.visible {
            return None;
        }

        let mouse = mouse_pos();
        let mut action = None;

        let title_rect = Rect::new(self.position.x, self.position.y, MANAGE_WIDTH, 22.0);
        self.position = self.drag_helper.update(title_rect, self.position, mouse);

        let x = self.position.x;
        let y = self.position.y;

        // 背景
        draw_rectangle(x, y, MANAGE_WIDTH, MANAGE_HEIGHT, Color::new(0.0, 0.0, 0.0, 0.85));
        draw_rectangle_lines(x, y, MANAGE_WIDTH, MANAGE_HEIGHT, 1.0, DARKGRAY);
        draw_text("英雄管理", x + 10.0, y + 16.0, 14.0, GOLD);

        // 英雄头像槽位 (2行4列)
        for i in 0..HERO_SLOTS {
            let col = i % 4;
            let row = i / 4;
            let slot_x = x + 16.0 + col as f32 * (CELL_SIZE + 24.0);
            let slot_y = y + 32.0 + row as f32 * (CELL_SIZE + 24.0);
            let slot_rect = Rect::new(slot_x, slot_y, CELL_SIZE, CELL_SIZE);

            let highlight = if self.selected_index == Some(i) {
                CellHighlight::Selected
            } else if slot_rect.contains(mouse) {
                CellHighlight::Hovered
            } else {
                CellHighlight::None
            };
            draw_cell_frame(slot_rect, highlight, &CellStyle::default());

            if let Some(hero) = &self.heroes[i] {
                let initial = hero.name.chars().next().unwrap_or('?').to_string();
                draw_text(&initial, slot_x + 10.0, slot_y + 22.0, 14.0, WHITE);

                if self.active_hero == Some(i) {
                    draw_rectangle_lines(slot_x - 1.0, slot_y - 1.0, CELL_SIZE + 2.0, CELL_SIZE + 2.0, 2.0, LIME);
                }
            }

            if is_mouse_over(slot_rect) && is_mouse_button_pressed(MouseButton::Left) {
                self.selected_index = Some(i);
            }
        }

        // 英雄信息面板
        if let Some(idx) = self.selected_index {
            if let Some(hero) = &self.heroes[idx] {
                let info_y = y + 120.0;
                draw_text(&format!("{} Lv.{}", hero.name, hero.level), x + 10.0, info_y, 12.0, WHITE);
                draw_text(&format!("HP: {}/{}", hero.hp, hero.max_hp), x + 10.0, info_y + 16.0, 11.0, LIME);

                // HP 条
                let bar_x = x + 10.0;
                let bar_y = info_y + 22.0;
                let bar_w = 120.0;
                draw_rectangle(bar_x, bar_y, bar_w, 6.0, Color::new(0.3, 0.0, 0.0, 1.0));
                draw_rectangle(bar_x, bar_y, bar_w * hero.hp_percent(), 6.0, Color::new(0.0, 0.8, 0.0, 1.0));
            }
        }

        // 操作按钮
        let btn_y = y + MANAGE_HEIGHT - 32.0;

        let summon_rect = Rect::new(x + 10.0, btn_y, 55.0, 20.0);
        draw_rectangle_lines(summon_rect.x, summon_rect.y, summon_rect.w, summon_rect.h, 1.0, GRAY);
        draw_text("召唤", summon_rect.x + 14.0, summon_rect.y + 14.0, 11.0, GRAY);
        if is_mouse_over(summon_rect) && is_mouse_button_pressed(MouseButton::Left) {
            if let Some(idx) = self.selected_index {
                action = Some(HeroAction::Summon(idx));
            }
        }

        let dismiss_rect = Rect::new(x + 75.0, btn_y, 55.0, 20.0);
        draw_rectangle_lines(dismiss_rect.x, dismiss_rect.y, dismiss_rect.w, dismiss_rect.h, 1.0, GRAY);
        draw_text("解散", dismiss_rect.x + 14.0, dismiss_rect.y + 14.0, 11.0, GRAY);
        if is_mouse_over(dismiss_rect) && is_mouse_button_pressed(MouseButton::Left) {
            action = Some(HeroAction::Dismiss);
        }

        let manage_rect = Rect::new(x + 140.0, btn_y, 55.0, 20.0);
        draw_rectangle_lines(manage_rect.x, manage_rect.y, manage_rect.w, manage_rect.h, 1.0, GRAY);
        draw_text("管理", manage_rect.x + 14.0, manage_rect.y + 14.0, 11.0, GRAY);
        if is_mouse_over(manage_rect) && is_mouse_button_pressed(MouseButton::Left) {
            if let Some(idx) = self.selected_index {
                action = Some(HeroAction::ManageHero(idx));
            }
        }

        // 关闭
        let close_rect = Rect::new(x + MANAGE_WIDTH - 20.0, y + 2.0, 16.0, 16.0);
        draw_text("X", close_rect.x + 3.0, close_rect.y + 12.0, 12.0, GRAY);
        if is_mouse_over(close_rect) && is_mouse_button_pressed(MouseButton::Left) {
            self.visible = false;
            action = Some(HeroAction::Close);
        }

        action
    }
}

// ============================================================================
// HeroInventoryDialogHybrid
// ============================================================================

/// 英雄背包对话框 (46 格)
pub struct HeroInventoryDialogHybrid {
    pub visible: bool,
    pub items: Vec<Option<usize>>,
    pub selected_slot: Option<usize>,
    position: Vec2,
    drag_helper: DragHelper,
}

impl HeroInventoryDialogHybrid {
    pub fn new() -> Self {
        Self {
            visible: false,
            items: vec![None; HERO_INVENTORY_SLOTS],
            selected_slot: None,
            position: Vec2::new(350.0, 100.0),
            drag_helper: DragHelper::new(),
        }
    }

    /// 绘制并处理输入
    pub fn draw(&mut self) -> Option<HeroAction> {
        if !self.visible {
            return None;
        }

        let mouse = mouse_pos();
        let mut action = None;

        let title_rect = Rect::new(self.position.x, self.position.y, INV_WIDTH, 22.0);
        self.position = self.drag_helper.update(title_rect, self.position, mouse);

        let x = self.position.x;
        let y = self.position.y;

        // 背景
        draw_rectangle(x, y, INV_WIDTH, INV_HEIGHT, Color::new(0.0, 0.0, 0.0, 0.85));
        draw_rectangle_lines(x, y, INV_WIDTH, INV_HEIGHT, 1.0, DARKGRAY);
        draw_text("英雄背包", x + 10.0, y + 16.0, 14.0, GOLD);

        // 物品格子
        let grid_x = x + 10.0;
        let grid_y = y + 28.0;
        for slot in 0..HERO_INVENTORY_SLOTS {
            let col = slot % HERO_INV_COLS;
            let row = slot / HERO_INV_COLS;
            let cell_x = grid_x + col as f32 * (CELL_SIZE + CELL_GAP);
            let cell_y = grid_y + row as f32 * (CELL_SIZE + CELL_GAP);
            let cell_rect = Rect::new(cell_x, cell_y, CELL_SIZE, CELL_SIZE);

            let highlight = if self.selected_slot == Some(slot) {
                CellHighlight::Selected
            } else if cell_rect.contains(mouse) {
                CellHighlight::Hovered
            } else {
                CellHighlight::None
            };
            draw_cell_frame(cell_rect, highlight, &CellStyle::default());

            if self.items[slot].is_some() {
                draw_rectangle(cell_x + 4.0, cell_y + 4.0, CELL_SIZE - 8.0, CELL_SIZE - 8.0, Color::new(0.3, 0.3, 0.6, 0.5));
            }

            if is_mouse_over(cell_rect) && is_mouse_button_pressed(MouseButton::Left) {
                if let Some(prev) = self.selected_slot {
                    if prev != slot {
                        action = Some(HeroAction::MoveItem { from: prev, to: slot });
                    }
                    self.selected_slot = None;
                } else {
                    self.selected_slot = Some(slot);
                }
            }
        }

        // 关闭
        let close_rect = Rect::new(x + INV_WIDTH - 20.0, y + 2.0, 16.0, 16.0);
        draw_text("X", close_rect.x + 3.0, close_rect.y + 12.0, 12.0, GRAY);
        if is_mouse_over(close_rect) && is_mouse_button_pressed(MouseButton::Left) {
            self.visible = false;
            action = Some(HeroAction::Close);
        }

        action
    }
}

// ============================================================================
// HeroBeltDialogHybrid
// ============================================================================

/// 英雄快捷栏 (6 格)
pub struct HeroBeltDialogHybrid {
    pub visible: bool,
    pub items: Vec<Option<usize>>,
    position: Vec2,
    drag_helper: DragHelper,
}

impl HeroBeltDialogHybrid {
    pub fn new() -> Self {
        Self {
            visible: false,
            items: vec![None; HERO_BELT_SLOTS],
            position: Vec2::new(400.0, 450.0),
            drag_helper: DragHelper::new(),
        }
    }

    /// 绘制并处理输入
    pub fn draw(&mut self) -> Option<HeroAction> {
        if !self.visible {
            return None;
        }

        let mouse = mouse_pos();
        let mut action = None;

        let title_rect = Rect::new(self.position.x, self.position.y, BELT_WIDTH, 14.0);
        self.position = self.drag_helper.update(title_rect, self.position, mouse);

        let x = self.position.x;
        let y = self.position.y;

        // 背景
        draw_rectangle(x, y, BELT_WIDTH, BELT_HEIGHT, Color::new(0.0, 0.0, 0.0, 0.75));
        draw_rectangle_lines(x, y, BELT_WIDTH, BELT_HEIGHT, 1.0, DARKGRAY);

        // 6 格快捷栏
        for slot in 0..HERO_BELT_SLOTS {
            let cell_x = x + 4.0 + slot as f32 * (CELL_SIZE + CELL_GAP);
            let cell_y = y + 8.0;
            let cell_rect = Rect::new(cell_x, cell_y, CELL_SIZE, CELL_SIZE);

            let highlight = if cell_rect.contains(mouse) {
                CellHighlight::Hovered
            } else {
                CellHighlight::None
            };
            draw_cell_frame(cell_rect, highlight, &CellStyle::default());

            if self.items[slot].is_some() {
                draw_rectangle(cell_x + 4.0, cell_y + 4.0, CELL_SIZE - 8.0, CELL_SIZE - 8.0, Color::new(0.3, 0.3, 0.6, 0.5));
            }

            if is_mouse_over(cell_rect) && is_mouse_button_pressed(MouseButton::Left) {
                action = Some(HeroAction::UseItem(slot));
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
    fn test_hero_info_hp_percent() {
        let hero = HeroInfo::new("TestHero", 50, 1, 500, 1000);
        assert!((hero.hp_percent() - 0.5).abs() < f32::EPSILON);

        let zero_hp = HeroInfo::new("Dead", 1, 0, 0, 0);
        assert!((zero_hp.hp_percent() - 0.0).abs() < f32::EPSILON);
    }

    #[test]
    fn test_hero_manage_set_hero() {
        let mut dialog = HeroManageDialogHybrid::new();
        assert_eq!(dialog.heroes.len(), HERO_SLOTS);
        assert!(dialog.heroes[0].is_none());

        dialog.set_hero(0, Some(HeroInfo::new("Hero1", 10, 1, 100, 200)));
        assert!(dialog.heroes[0].is_some());
        assert_eq!(dialog.heroes[0].as_ref().unwrap().name, "Hero1");

        // Out-of-bounds set is ignored
        dialog.set_hero(99, Some(HeroInfo::new("Bad", 1, 0, 0, 0)));
        assert_eq!(dialog.heroes.len(), HERO_SLOTS);
    }

    #[test]
    fn test_hero_manage_active_hero() {
        let mut dialog = HeroManageDialogHybrid::new();
        assert!(dialog.get_active_hero().is_none());

        dialog.set_hero(2, Some(HeroInfo::new("Active", 30, 2, 300, 500)));
        dialog.active_hero = Some(2);
        assert_eq!(dialog.get_active_hero().unwrap().name, "Active");
    }

    #[test]
    fn test_hero_inventory_creation() {
        let dialog = HeroInventoryDialogHybrid::new();
        assert_eq!(dialog.items.len(), HERO_INVENTORY_SLOTS);
        assert!(!dialog.visible);
        assert!(dialog.selected_slot.is_none());
    }
}
