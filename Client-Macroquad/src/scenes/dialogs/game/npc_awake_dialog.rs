// ============================================================================
// NPCAwakeDialogHybrid - 装备觉醒对话框
// ============================================================================
//
// C# 参考: Client/MIRScenes/Dialogs/NPCDialogs.cs (~1317 行)
// - 显示装备觉醒材料需求
// - 觉醒属性预览
// - 锁定/解锁装备
//
// ============================================================================

use macroquad::prelude::*;
use crate::ui::text_renderer::draw_text_cn;

/// 觉醒材料条目
#[derive(Debug, Clone)]
pub struct AwakeningMaterial {
    pub name: String,
    pub required: u32,
    pub have: u32,
}

/// 装备觉醒对话框
#[derive(Default)]
pub struct NPCAwakeDialogHybrid {
    pub visible: bool,
    pub item_name: String,
    pub materials: Vec<AwakeningMaterial>,
    pub can_awaken: bool,
    pub is_locked: bool,
}

impl NPCAwakeDialogHybrid {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn show(&mut self, item_name: String, materials: Vec<AwakeningMaterial>) {
        self.item_name = item_name;
        self.materials = materials;
        self.can_awaken = self.materials.iter().all(|m| m.have >= m.required);
        self.visible = true;
    }

    pub fn close(&mut self) {
        self.visible = false;
        self.materials.clear();
    }

    /// 更新锁定状态
    pub fn set_locked(&mut self, locked: bool) {
        self.is_locked = locked;
    }

    /// 绘制
    pub fn draw(&mut self, screen_w: f32, screen_h: f32, mouse_pos: Vec2,
                left_clicked: bool) -> bool {
        if !self.visible {
            return false;
        }

        let padding = 15.0;
        let title_h = 30.0;
        let item_h = 25.0;
        let btn_h = 30.0;
        let dialog_w = 320.0;
        let dialog_h = title_h + (self.materials.len() as f32) * item_h + btn_h * 2.0 + padding * 4.0;

        let dialog_x = (screen_w - dialog_w) / 2.0;
        let dialog_y = (screen_h - dialog_h) / 2.0;

        let mouse_over = mouse_pos.x >= dialog_x && mouse_pos.x <= dialog_x + dialog_w
            && mouse_pos.y >= dialog_y && mouse_pos.y <= dialog_y + dialog_h;

        // 背景
        draw_rectangle(dialog_x, dialog_y, dialog_w, dialog_h, Color::from_rgba(25, 25, 40, 230));

        // 标题
        let title = format!("装备觉醒: {}", self.item_name);
        draw_text_cn(&title, dialog_x + 15.0, dialog_y + 10.0, 15.0,
            Color::from_rgba(255, 180, 50, 255));

        // 锁定状态
        let lock_text = if self.is_locked { "🔒 已锁定" } else { "🔓 未锁定" };
        draw_text_cn(lock_text, dialog_x + dialog_w - 100.0, dialog_y + 10.0, 12.0, WHITE);

        // 材料列表
        let mat_y = dialog_y + title_h + padding;
        for (i, mat) in self.materials.iter().enumerate() {
            let y = mat_y + i as f32 * item_h;
            let has_enough = mat.have >= mat.required;

            let line = format!("{}: {}/{}", mat.name, mat.have, mat.required);
            draw_text_cn(&line, dialog_x + 15.0, y + 5.0, 13.0,
                if has_enough {
                    Color::from_rgba(100, 220, 100, 255)
                } else {
                    Color::from_rgba(220, 100, 100, 255)
                });
        }

        // 觉醒按钮
        let awaken_y = mat_y + (self.materials.len() as f32) * item_h + padding;
        let btn_w = 120.0;
        let btn_x = dialog_x + (dialog_w - btn_w * 2.0 - 10.0) / 2.0;

        let can_click = self.can_awaken && !self.is_locked;
        let awaken_color = if can_click {
            Color::from_rgba(80, 160, 80, 255)
        } else {
            Color::from_rgba(60, 60, 60, 255)
        };
        draw_rectangle(btn_x, awaken_y, btn_w, btn_h, awaken_color);
        draw_text_cn("觉醒", btn_x + 25.0, awaken_y + 7.0, 14.0, WHITE);

        // 关闭按钮
        let close_x = btn_x + btn_w + 10.0;
        let mouse_over_close = mouse_pos.x >= close_x && mouse_pos.x <= close_x + btn_w
            && mouse_pos.y >= awaken_y && mouse_pos.y <= awaken_y + btn_h;
        let close_color = if mouse_over_close {
            Color::from_rgba(150, 50, 50, 255)
        } else {
            Color::from_rgba(100, 30, 30, 255)
        };
        draw_rectangle(close_x, awaken_y, btn_w, btn_h, close_color);
        draw_text_cn("关闭", close_x + 25.0, awaken_y + 7.0, 14.0, WHITE);

        if left_clicked && mouse_over_close {
            self.close();
        }

        mouse_over
    }
}
