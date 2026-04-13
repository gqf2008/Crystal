// ============================================================================
// GuildTerritoryDialogHybrid - 行会领地管理对话框
// ============================================================================
//
// C# 参考: Client/MIRScenes/Dialogs/GuildTerritoryDialog.cs (~361 行)
// - 显示行会领地列表
// - 购买/出售领地
// - 分页导航
//
// ============================================================================

use macroquad::prelude::*;
use crate::ui::text_renderer::draw_text_cn;

/// 领地条目
#[derive(Debug, Clone)]
pub struct TerritoryEntry {
    pub name: String,
    pub map_name: String,
    pub owner: String,
    pub price: u32,
    pub is_purchased: bool,
}

/// 行会领地管理对话框
pub struct GuildTerritoryDialogHybrid {
    pub visible: bool,
    pub entries: Vec<TerritoryEntry>,
    pub current_page: i32,
    pub total_pages: i32,
    scroll_offset: f32,
}

impl Default for GuildTerritoryDialogHybrid {
    fn default() -> Self {
        Self {
            visible: false,
            entries: Vec::new(),
            current_page: 1,
            total_pages: 1,
            scroll_offset: 0.0,
        }
    }
}

impl GuildTerritoryDialogHybrid {
    pub fn new() -> Self {
        Self::default()
    }

    /// 更新领地列表
    pub fn update_territories(&mut self, entries: Vec<TerritoryEntry>, page: i32, total: i32) {
        self.entries = entries;
        self.current_page = page;
        self.total_pages = total.max(1);
        self.scroll_offset = 0.0;
    }

    pub fn show(&mut self) {
        self.visible = true;
    }

    pub fn close(&mut self) {
        self.visible = false;
    }

    pub fn toggle(&mut self) {
        self.visible = !self.visible;
    }

    /// 处理鼠标和绘制
    pub fn draw(&mut self, screen_w: f32, screen_h: f32, mouse_pos: Vec2,
                mouse_wheel: f32, left_clicked: bool) -> bool {
        if !self.visible {
            return false;
        }

        let padding = 15.0;
        let title_h = 30.0;
        let item_h = 35.0;
        let page_bar_h = 30.0;
        let dialog_w = 400.0;
        let max_items = 6;
        let dialog_h = title_h + (max_items.min(self.entries.len()) as f32) * item_h
            + page_bar_h + padding * 3.0;

        let dialog_x = (screen_w - dialog_w) / 2.0;
        let dialog_y = (screen_h - dialog_h) / 2.0;

        let mouse_over = mouse_pos.x >= dialog_x && mouse_pos.x <= dialog_x + dialog_w
            && mouse_pos.y >= dialog_y && mouse_pos.y <= dialog_y + dialog_h;

        // 滚动
        if mouse_over && mouse_wheel != 0.0 {
            self.scroll_offset = (self.scroll_offset - mouse_wheel * 20.0).max(0.0);
        }

        // 背景
        draw_rectangle(dialog_x, dialog_y, dialog_w, dialog_h, Color::from_rgba(25, 25, 35, 230));

        // 标题
        draw_text_cn("行会领地管理", dialog_x + 15.0, dialog_y + 10.0, 16.0,
            Color::from_rgba(255, 220, 100, 255));

        // 领地列表
        let content_y = dialog_y + title_h + padding;
        for (i, entry) in self.entries.iter().enumerate() {
            let y = content_y + i as f32 * item_h - self.scroll_offset;
            if y < content_y || y + item_h > content_y + max_items as f32 * item_h {
                continue;
            }

            // 领地信息
            let status = if entry.is_purchased { "已占领" } else { &entry.owner };
            let line1 = format!("{} - {}", entry.name, entry.map_name);
            let line2 = format!("状态: {} | 价格: {}", status, entry.price);

            draw_text_cn(&line1, dialog_x + 15.0, y + 5.0, 13.0,
                if entry.is_purchased { Color::from_rgba(100, 200, 100, 255) }
                else { Color::from_rgba(200, 200, 200, 255) });
            draw_text_cn(&line2, dialog_x + 15.0, y + 20.0, 11.0,
                Color::from_rgba(150, 150, 150, 255));
        }

        // 分页栏
        let page_y = dialog_y + dialog_h - page_bar_h - padding;
        let page_text = format!("第 {}/{} 页", self.current_page, self.total_pages);
        draw_text_cn(&page_text, dialog_x + 30.0, page_y + 5.0, 13.0, WHITE);

        // 关闭按钮
        let close_x = dialog_x + dialog_w - 70.0;
        let mouse_over_close = mouse_pos.x >= close_x && mouse_pos.x <= close_x + 55.0
            && mouse_pos.y >= page_y && mouse_pos.y <= page_y + 25.0;
        let close_color = if mouse_over_close {
            Color::from_rgba(150, 50, 50, 255)
        } else {
            Color::from_rgba(100, 30, 30, 255)
        };
        draw_rectangle(close_x, page_y, 55.0, 25.0, close_color);
        draw_text_cn("关闭", close_x + 12.0, page_y + 6.0, 14.0, WHITE);

        if left_clicked && mouse_over_close {
            self.close();
        }

        mouse_over
    }
}
