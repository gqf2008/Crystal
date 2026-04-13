// ============================================================================
// RefineDialogHybrid - 装备精炼对话框
// ============================================================================
//
// C# 参考: Client/MIRScenes/Dialogs/NPCDialogs.cs (~2167 行)
// - 显示精炼属性预览
// - 精炼材料需求
// - 确认精炼/取消精炼
//
// ============================================================================

use macroquad::prelude::*;
use crate::ui::text_renderer::draw_text_cn;

/// 精炼属性条目
#[derive(Debug, Clone)]
pub struct RefineStat {
    pub name: String,
    pub current: i32,
    pub after: i32,
}

/// 精炼对话框
#[derive(Default)]
pub struct RefineDialogHybrid {
    pub visible: bool,
    pub item_name: String,
    pub stats: Vec<RefineStat>,
    pub material_name: String,
    pub material_have: u32,
    pub material_need: u32,
}

impl RefineDialogHybrid {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn show(&mut self, item_name: String, stats: Vec<RefineStat>,
                material_name: String, material_have: u32, material_need: u32) {
        self.item_name = item_name;
        self.stats = stats;
        self.material_name = material_name;
        self.material_have = material_have;
        self.material_need = material_need;
        self.visible = true;
    }

    pub fn close(&mut self) {
        self.visible = false;
    }

    /// 绘制
    pub fn draw(&mut self, screen_w: f32, screen_h: f32, mouse_pos: Vec2,
                left_clicked: bool) -> bool {
        if !self.visible {
            return false;
        }

        let padding = 15.0;
        let title_h = 30.0;
        let stat_h = 22.0;
        let btn_h = 30.0;
        let dialog_w = 300.0;
        let dialog_h = title_h + (self.stats.len() as f32) * stat_h + btn_h * 2.0 + padding * 4.0 + 25.0;

        let dialog_x = (screen_w - dialog_w) / 2.0;
        let dialog_y = (screen_h - dialog_h) / 2.0;

        let mouse_over = mouse_pos.x >= dialog_x && mouse_pos.x <= dialog_x + dialog_w
            && mouse_pos.y >= dialog_y && mouse_pos.y <= dialog_y + dialog_h;

        // 背景
        draw_rectangle(dialog_x, dialog_y, dialog_w, dialog_h, Color::from_rgba(30, 25, 40, 230));
        draw_text_cn(&format!("精炼: {}", self.item_name), dialog_x + 15.0, dialog_y + 10.0, 16.0,
            Color::from_rgba(200, 150, 255, 255));

        // 属性预览
        let stat_y = dialog_y + title_h + padding;
        for (i, stat) in self.stats.iter().enumerate() {
            let y = stat_y + i as f32 * stat_h;
            let delta = stat.after - stat.current;
            let line = if delta > 0 {
                format!("{}: {} → {} (+{})", stat.name, stat.current, stat.after, delta)
            } else {
                format!("{}: {}", stat.name, stat.current)
            };
            draw_text_cn(&line, dialog_x + 15.0, y + 5.0, 12.0,
                if delta > 0 { Color::from_rgba(100, 220, 100, 255) }
                else { Color::from_rgba(200, 200, 200, 255) });
        }

        // 材料需求
        let mat_y = stat_y + (self.stats.len() as f32) * stat_h + padding;
        let has_enough = self.material_have >= self.material_need;
        draw_text_cn(&format!("材料: {} {}/{}", self.material_name, self.material_have, self.material_need),
            dialog_x + 15.0, mat_y + 5.0, 13.0,
            if has_enough { Color::from_rgba(100, 220, 100, 255) }
            else { Color::from_rgba(220, 100, 100, 255) });

        // 按钮
        let btn_y = mat_y + padding + 25.0;
        let btn_w = 100.0;
        let btn_gap = 10.0;

        // 精炼按钮
        let refine_x = dialog_x + padding;
        let can_refine = has_enough;
        let refine_color = if can_refine {
            Color::from_rgba(80, 160, 220, 255)
        } else {
            Color::from_rgba(60, 60, 60, 255)
        };
        draw_rectangle(refine_x, btn_y, btn_w, btn_h, refine_color);
        draw_text_cn("精炼", refine_x + 25.0, btn_y + 7.0, 14.0, WHITE);

        // 关闭按钮
        let close_x = refine_x + btn_w + btn_gap;
        let mouse_over_close = mouse_pos.x >= close_x && mouse_pos.x <= close_x + btn_w
            && mouse_pos.y >= btn_y && mouse_pos.y <= btn_y + btn_h;
        draw_rectangle(close_x, btn_y, btn_w, btn_h,
            if mouse_over_close { Color::from_rgba(150, 50, 50, 255) }
            else { Color::from_rgba(100, 30, 30, 255) });
        draw_text_cn("关闭", close_x + 25.0, btn_y + 7.0, 14.0, WHITE);

        if left_clicked && mouse_over_close {
            self.close();
        }

        mouse_over
    }
}
