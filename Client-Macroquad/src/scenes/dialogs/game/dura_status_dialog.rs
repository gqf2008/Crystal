// ============================================================================
// DuraStatusDialogHybrid - 装备耐久度状态对话框
// ============================================================================
//
// C# 参考: Client/MIRScenes/Dialogs/MainDialogs.cs (~3886 行)
// - 显示装备耐久度状态覆盖层
// - 低耐久度时红色警告
// - 点击耐久图标时弹出
//
// ============================================================================

use macroquad::prelude::*;
use crate::ui::text_renderer::draw_text_cn;

/// 装备耐久度条目
#[derive(Debug, Clone)]
pub struct DuraEntry {
    pub slot_name: String,
    pub current: u32,
    pub max: u32,
}

/// 耐久度状态对话框
#[derive(Default)]
pub struct DuraStatusDialogHybrid {
    pub visible: bool,
    pub items: Vec<DuraEntry>,
}

impl DuraStatusDialogHybrid {
    pub fn new() -> Self {
        Self::default()
    }

    /// 更新耐久度列表
    pub fn update_dura(&mut self, items: Vec<DuraEntry>) {
        self.items = items;
        // 有低耐久装备时自动弹出
        let has_low = self.items.iter().any(|i| i.current < i.max / 4);
        if has_low {
            self.visible = true;
        }
    }

    /// 切换显示/隐藏
    pub fn toggle(&mut self) {
        if self.items.is_empty() {
            self.visible = false;
        } else {
            self.visible = !self.visible;
        }
    }

    /// 关闭
    pub fn close(&mut self) {
        self.visible = false;
    }

    /// 绘制
    pub fn draw(&mut self, screen_w: f32, screen_h: f32, mouse_pos: Vec2, left_clicked: bool) -> bool {
        if !self.visible || self.items.is_empty() {
            return false;
        }

        let padding = 10.0;
        let item_h = 22.0;
        let title_h = 25.0;
        let dialog_w = 180.0;
        let dialog_h = title_h + self.items.len() as f32 * item_h + padding * 2.0 + 10.0;

        let dialog_x = screen_w - dialog_w - padding;
        let dialog_y = screen_h * 0.4;

        let mouse_over = mouse_pos.x >= dialog_x && mouse_pos.x <= dialog_x + dialog_w
            && mouse_pos.y >= dialog_y && mouse_pos.y <= dialog_y + dialog_h;

        // 背景
        let bg_color = Color::from_rgba(25, 25, 25, 220);
        draw_rectangle(dialog_x, dialog_y, dialog_w, dialog_h, bg_color);

        // 标题
        draw_text_cn("装备耐久度", dialog_x + 10.0, dialog_y + 8.0, 13.0,
            Color::from_rgba(255, 200, 50, 255));

        // 每个装备条目
        for (i, entry) in self.items.iter().enumerate() {
            let y = dialog_y + title_h + padding + i as f32 * item_h;
            let ratio = if entry.max > 0 { entry.current as f32 / entry.max as f32 } else { 0.0 };

            // 耐久条背景
            let bar_w = dialog_w - 20.0;
            let bar_h = 10.0;
            let bar_x = dialog_x + 10.0;
            let bar_y = y + 2.0;

            let bar_bg = Color::from_rgba(50, 50, 50, 200);
            draw_rectangle(bar_x, bar_y, bar_w, bar_h, bar_bg);

            // 耐久条填充（颜色根据耐久比例）
            let fill_color = if ratio > 0.5 {
                Color::from_rgba(50, 200, 50, 255)
            } else if ratio > 0.25 {
                Color::from_rgba(200, 200, 50, 255)
            } else {
                Color::from_rgba(200, 50, 50, 255)
            };
            draw_rectangle(bar_x, bar_y, bar_w * ratio, bar_h, fill_color);

            // 装备名称
            draw_text_cn(&entry.slot_name, bar_x + 2.0, bar_y + 1.0, 8.0,
                Color::from_rgba(200, 200, 200, 255));
        }

        // 关闭按钮
        let close_y = dialog_y + dialog_h - 22.0;
        let mouse_over_close = mouse_pos.x >= dialog_x + 60.0 && mouse_pos.x <= dialog_x + dialog_w - 60.0
            && mouse_pos.y >= close_y && mouse_pos.y <= close_y + 18.0;

        let close_color = if mouse_over_close {
            Color::from_rgba(150, 50, 50, 255)
        } else {
            Color::from_rgba(100, 30, 30, 255)
        };
        draw_rectangle(dialog_x + 60.0, close_y, dialog_w - 120.0, 18.0, close_color);
        draw_text_cn("关闭", dialog_x + 70.0, close_y + 4.0, 12.0, WHITE);

        if left_clicked && mouse_over_close {
            self.close();
        }

        mouse_over
    }
}
