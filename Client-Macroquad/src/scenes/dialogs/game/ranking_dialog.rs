// ============================================================================
// RankingDialogHybrid - 排行榜对话框
// ============================================================================
// 显示服务器排行榜信息（等级/金币/声望等）
// ============================================================================

use macroquad::prelude::*;
use crate::ui::text_renderer::draw_text_cn;
use super::native_ui_utils::*;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RankingTab {
    Level = 0,
    Gold = 1,
    Reputation = 2,
}

#[derive(Debug, Clone)]
pub struct RankingEntry {
    pub rank: u32,
    pub name: String,
    pub value: String,
}

#[derive(Debug)]
pub enum RankingDialogAction {
    None,
    Refresh { tab: u8 },
}

pub struct RankingDialogHybrid {
    position: Vec2,
    size: Vec2,
    visible: bool,
    drag_helper: DragHelper,

    current_tab: RankingTab,
    entries: [Vec<RankingEntry>; 3],
    scroll_offsets: [f32; 3],

    hovered_close: bool,
    hovered_refresh: bool,
    hovered_tab: Option<usize>,

    close_btn: ButtonTextures,
    pending_action: RankingDialogAction,
}

impl Default for RankingDialogHybrid {
    fn default() -> Self { Self::new() }
}

impl RankingDialogHybrid {
    const WIDTH: f32 = 320.0;
    const HEIGHT: f32 = 380.0;
    const ENTRY_H: f32 = 28.0;
    const VISIBLE_ENTRIES: usize = 10;

    pub fn new() -> Self {
        Self {
            position: vec2(200.0, 150.0),
            size: vec2(Self::WIDTH, Self::HEIGHT),
            visible: false,
            drag_helper: DragHelper::new(),
            current_tab: RankingTab::Level,
            entries: [Vec::new(), Vec::new(), Vec::new()],
            scroll_offsets: [0.0, 0.0, 0.0],
            hovered_close: false,
            hovered_refresh: false,
            hovered_tab: None,
            close_btn: ButtonTextures::new(),
            pending_action: RankingDialogAction::None,
        }
    }

    pub fn open(&mut self) {
        if !self.visible {
            self.visible = true;
            self.pending_action = RankingDialogAction::Refresh { tab: self.current_tab as u8 };
        }
    }

    pub fn close(&mut self) {
        self.visible = false;
    }

    pub fn is_visible(&self) -> bool {
        self.visible
    }

    pub fn contains(&self, pos: Vec2) -> bool {
        self.visible && Rect::new(self.position.x, self.position.y, self.size.x, self.size.y).contains(pos)
    }

    pub fn take_action(&mut self) -> RankingDialogAction {
        std::mem::replace(&mut self.pending_action, RankingDialogAction::None)
    }

    pub fn set_rankings(&mut self, tab: RankingTab, entries: Vec<RankingEntry>) {
        self.entries[tab as usize] = entries;
        self.scroll_offsets[tab as usize] = 0.0;
    }

    pub fn update_and_draw(&mut self) {
        if !self.visible {
            return;
        }

        let mouse = mouse_pos();

        // 窗口拖动
        let drag_area = Rect::new(self.position.x, self.position.y, self.size.x - 24.0, 30.0);
        self.drag_helper.apply(drag_area, &mut self.position);

        // 关闭按钮
        self.hovered_close = Rect::new(self.position.x + self.size.x - 24.0, self.position.y + 4.0, 20.0, 20.0).contains(mouse);
        if is_mouse_button_pressed(MouseButton::Left) && self.hovered_close {
            self.close();
            return;
        }

        // 背景
        draw_rectangle(self.position.x, self.position.y, self.size.x, self.size.y, Color::from_rgba(30, 30, 40, 240));
        draw_rectangle_lines(self.position.x, self.position.y, self.size.x, self.size.y, 1.0, Color::from_rgba(100, 100, 120, 255));

        // 标题
        draw_text_cn("排行榜", self.position.x + 120.0, self.position.y + 8.0, 16.0, YELLOW);

        // 标签页
        let tab_names = ["等级", "金币", "声望"];
        let tab_w = 80.0;
        let tab_h = 24.0;
        let tab_y = self.position.y + 32.0;
        for (i, &name) in tab_names.iter().enumerate() {
            let tab_x = self.position.x + 20.0 + i as f32 * (tab_w + 5.0);
            let rect = Rect::new(tab_x, tab_y, tab_w, tab_h);
            let is_current = self.current_tab as usize == i;
            let color = if is_current {
                Color::from_rgba(80, 80, 100, 255)
            } else {
                Color::from_rgba(50, 50, 60, 255)
            };
            draw_rectangle(rect.x, rect.y, rect.w, rect.h, color);
            draw_rectangle_lines(rect.x, rect.y, rect.w, rect.h, 1.0, Color::from_rgba(150, 150, 170, 255));
            draw_text_cn(name, rect.x + 24.0, rect.y + 14.0, 13.0, if is_current { YELLOW } else { WHITE });

            if is_mouse_button_pressed(MouseButton::Left) && rect.contains(mouse) {
                self.current_tab = RankingTab::from_index(i);
                self.hovered_tab = Some(i);
            }
        }

        // 刷新按钮
        let refresh_rect = Rect::new(self.position.x + self.size.x - 90.0, tab_y, 60.0, tab_h);
        self.hovered_refresh = refresh_rect.contains(mouse);
        let refresh_color = if self.hovered_refresh {
            Color::from_rgba(80, 120, 80, 255)
        } else {
            Color::from_rgba(50, 80, 50, 255)
        };
        draw_rectangle(refresh_rect.x, refresh_rect.y, refresh_rect.w, refresh_rect.h, refresh_color);
        draw_rectangle_lines(refresh_rect.x, refresh_rect.y, refresh_rect.w, refresh_rect.h, 1.0, Color::from_rgba(150, 150, 170, 255));
        draw_text_cn("刷新", refresh_rect.x + 14.0, refresh_rect.y + 14.0, 13.0, WHITE);
        if is_mouse_button_pressed(MouseButton::Left) && self.hovered_refresh {
            self.pending_action = RankingDialogAction::Refresh { tab: self.current_tab as u8 };
        }

        // 列表区域
        let list_y = tab_y + tab_h + 5.0;
        let list_h = self.size.y - (list_y - self.position.y) - 10.0;
        let list_rect = Rect::new(self.position.x + 10.0, list_y, self.size.x - 20.0, list_h);
        draw_rectangle_lines(list_rect.x, list_rect.y, list_rect.w, list_rect.h, 1.0, Color::from_rgba(80, 80, 100, 255));

        // 滚动
        let entries = &self.entries[self.current_tab as usize];
        let max_scroll = (entries.len().saturating_sub(Self::VISIBLE_ENTRIES) as f32) * Self::ENTRY_H;
        if max_scroll > 0.0 {
            let wheel = mouse_wheel().1;
            if list_rect.contains(mouse) && wheel != 0.0 {
                self.scroll_offsets[self.current_tab as usize] = (self.scroll_offsets[self.current_tab as usize] - wheel * 30.0).clamp(0.0, max_scroll);
            }
        }

        // 表头
        let header_y = list_y + 2.0;
        draw_text_cn("排名", list_rect.x + 10.0, header_y + 2.0, 12.0, Color::from_rgba(200, 200, 220, 255));
        draw_text_cn("角色", list_rect.x + 60.0, header_y + 2.0, 12.0, Color::from_rgba(200, 200, 220, 255));
        draw_text_cn("数值", list_rect.x + 200.0, header_y + 2.0, 12.0, Color::from_rgba(200, 200, 220, 255));

        // 列表项（裁剪）
        let scroll = self.scroll_offsets[self.current_tab as usize];
        for (i, entry) in entries.iter().enumerate() {
            let item_y = list_y + 20.0 + i as f32 * Self::ENTRY_H - scroll;
            if item_y + Self::ENTRY_H < list_y + 20.0 || item_y > list_y + list_h {
                continue;
            }
            let rank_color = match entry.rank {
                1 => Color::from_rgba(255, 215, 0, 255),
                2 => Color::from_rgba(192, 192, 192, 255),
                3 => Color::from_rgba(205, 127, 50, 255),
                _ => WHITE,
            };
            draw_text_cn(&format!("#{}", entry.rank), list_rect.x + 10.0, item_y + 2.0, 13.0, rank_color);
            draw_text_cn(&entry.name, list_rect.x + 60.0, item_y + 2.0, 13.0, WHITE);
            draw_text_cn(&entry.value, list_rect.x + 200.0, item_y + 2.0, 13.0, YELLOW);
        }

        // 关闭按钮图标
        if let Some(ref tex) = self.close_btn.textures[0] {
            draw_texture(tex, self.position.x + self.size.x - 22.0, self.position.y + 4.0, WHITE);
        }
    }

    pub fn load_textures(&mut self) {
        if let Some(tex) = crate::resources::LibraryName::Prguse2.get_texture(360).and_then(|i| i.image) {
            self.close_btn.textures[0] = Some(tex);
        }
    }
}

impl RankingTab {
    pub fn from_index(idx: usize) -> Self {
        match idx {
            0 => RankingTab::Level,
            1 => RankingTab::Gold,
            2 => RankingTab::Reputation,
            _ => RankingTab::Level,
        }
    }
}
