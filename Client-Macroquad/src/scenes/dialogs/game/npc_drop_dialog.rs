// ============================================================================
// NPCDropDialogHybrid - NPC 赠送物品对话框
// ============================================================================
//
// C# 参考: Client/MIRScenes/Dialogs/NPCDialogs.cs (~883 行)
// - 显示 NPC 赠送给玩家的物品列表
// - 确认领取按钮
//
// ============================================================================

use macroquad::prelude::*;
use crate::ui::text_renderer::draw_text_cn;
use mir2_shared::data::item::UserItem;

/// NPC 赠送物品对话框
#[derive(Default)]
pub struct NPCDropDialogHybrid {
    pub visible: bool,
    pub npc_name: String,
    pub items: Vec<UserItem>,
}

impl NPCDropDialogHybrid {
    pub fn new() -> Self {
        Self::default()
    }

    /// 显示 NPC 赠送物品
    pub fn show(&mut self, npc_name: String, items: Vec<UserItem>) {
        self.npc_name = npc_name;
        self.items = items;
        self.visible = !self.items.is_empty();
    }

    /// 关闭
    pub fn close(&mut self) {
        self.visible = false;
        self.items.clear();
    }

    /// 领取所有物品（返回物品列表供上层处理）
    pub fn take_items(&mut self) -> Vec<UserItem> {
        std::mem::take(&mut self.items)
    }

    /// 绘制
    pub fn draw(&mut self, screen_w: f32, screen_h: f32, mouse_pos: Vec2, left_clicked: bool) -> bool {
        if !self.visible {
            return false;
        }

        let padding = 15.0;
        let item_h = 25.0;
        let title_h = 30.0;
        let btn_h = 30.0;
        let dialog_w = 300.0;
        let dialog_h = title_h + (self.items.len() as f32) * item_h + btn_h + padding * 3.0;

        let dialog_x = (screen_w - dialog_w) / 2.0;
        let dialog_y = (screen_h - dialog_h) / 2.0;

        let mouse_over = mouse_pos.x >= dialog_x && mouse_pos.x <= dialog_x + dialog_w
            && mouse_pos.y >= dialog_y && mouse_pos.y <= dialog_y + dialog_h;

        // 背景
        draw_rectangle(dialog_x, dialog_y, dialog_w, dialog_h, Color::from_rgba(30, 30, 40, 230));

        // 标题
        let title = format!("{} 赠送的物品", self.npc_name);
        draw_text_cn(&title, dialog_x + 15.0, dialog_y + 10.0, 16.0,
            Color::from_rgba(255, 220, 100, 255));

        // 物品列表
        for (i, item) in self.items.iter().enumerate() {
            let y = dialog_y + title_h + padding + i as f32 * item_h;
            let name = item.info.as_ref().map(|info| info.name.as_str()).unwrap_or("未知物品");
            let count_str = if item.count > 1 {
                format!("{} x{}", name, item.count)
            } else {
                name.to_string()
            };
            draw_text_cn(&count_str, dialog_x + 15.0, y + 5.0, 14.0,
                Color::from_rgba(200, 200, 200, 255));
        }

        // 领取按钮
        let btn_w = 100.0;
        let btn_x = dialog_x + (dialog_w - btn_w) / 2.0;
        let btn_y = dialog_y + dialog_h - btn_h - padding;
        let mouse_over_btn = mouse_pos.x >= btn_x && mouse_pos.x <= btn_x + btn_w
            && mouse_pos.y >= btn_y && mouse_pos.y <= btn_y + btn_h;

        let btn_color = if mouse_over_btn {
            Color::from_rgba(80, 180, 80, 255)
        } else {
            Color::from_rgba(50, 120, 50, 255)
        };
        draw_rectangle(btn_x, btn_y, btn_w, btn_h, btn_color);
        draw_text_cn("领取", btn_x + 25.0, btn_y + 7.0, 16.0, WHITE);

        if left_clicked && mouse_over_btn {
            // 领取逻辑由调用方通过 take_items 处理
            self.close();
        }

        mouse_over
    }
}
