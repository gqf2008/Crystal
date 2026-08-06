// ============================================================================
// ItemRentalDialogHybrid - 物品租赁对话框
// ============================================================================
//
// C# 参考: Client/MIRScenes/Dialogs/ItemRentalDialog.cs + ItemRentingDialog.cs + GuestItemRentDialog.cs
// - 租赁物品列表
// - 设置租赁费用/期限
// - 锁定/确认流程
//
// ============================================================================

use crate::ui::text_renderer::draw_text_cn;
use macroquad::prelude::*;
use mir2_shared::data::item::UserItem;

/// 租赁中的物品条目
#[derive(Debug, Clone)]
pub struct RentalItem {
    pub slot: usize,
    pub item: Option<UserItem>,
}

/// 物品租赁对话框
pub struct ItemRentalDialogHybrid {
    pub visible: bool,
    /// 租赁物品槽位（最多6个）
    pub items: Vec<RentalItem>,
    /// 租赁费用（金币）
    pub fee: u32,
    /// 租赁期限（天）
    pub period: u32,
    /// 我方是否已锁定
    pub locked_by_me: bool,
    /// 对方是否已锁定
    pub locked_by_partner: bool,
    /// 对方名称
    pub partner_name: String,
    /// 确认按钮是否被点击（由 draw 阶段设置，上层读取后清除）
    pub confirm_clicked: bool,
}

impl Default for ItemRentalDialogHybrid {
    fn default() -> Self {
        Self {
            visible: false,
            items: (0..6).map(|slot| RentalItem { slot, item: None }).collect(),
            fee: 0,
            period: 1,
            locked_by_me: false,
            locked_by_partner: false,
            partner_name: String::new(),
            confirm_clicked: false,
        }
    }
}

impl ItemRentalDialogHybrid {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn show(&mut self, partner_name: String) {
        self.partner_name = partner_name;
        self.items.iter_mut().for_each(|item| item.item = None);
        self.fee = 0;
        self.period = 1;
        self.locked_by_me = false;
        self.locked_by_partner = false;
        self.confirm_clicked = false;
        self.visible = true;
    }

    pub fn close(&mut self) {
        self.visible = false;
        self.confirm_clicked = false;
    }

    pub fn update_fee(&mut self, fee: u32) {
        self.fee = fee;
    }

    pub fn update_period(&mut self, period: u32) {
        self.period = period;
    }

    pub fn set_locked(&mut self, locked: bool) {
        self.locked_by_me = locked;
    }

    pub fn set_partner_locked(&mut self, locked: bool) {
        self.locked_by_partner = locked;
    }

    /// 设置物品到槽位
    pub fn set_item(&mut self, slot: usize, item: Option<UserItem>) {
        if let Some(entry) = self.items.iter_mut().find(|e| e.slot == slot) {
            entry.item = item;
        }
    }

    /// 绘制
    pub fn draw(
        &mut self,
        screen_w: f32,
        screen_h: f32,
        mouse_pos: Vec2,
        left_clicked: bool,
    ) -> bool {
        if !self.visible {
            return false;
        }

        let padding = 15.0;
        let title_h = 30.0;
        let item_h = 35.0;
        let btn_h = 28.0;
        let dialog_w = 380.0;
        let dialog_h = title_h + 6.0 * item_h + btn_h * 3.0 + padding * 4.0;

        let dialog_x = (screen_w - dialog_w) / 2.0;
        let dialog_y = (screen_h - dialog_h) / 2.0;

        let mouse_over = mouse_pos.x >= dialog_x
            && mouse_pos.x <= dialog_x + dialog_w
            && mouse_pos.y >= dialog_y
            && mouse_pos.y <= dialog_y + dialog_h;

        // 背景
        draw_rectangle(
            dialog_x,
            dialog_y,
            dialog_w,
            dialog_h,
            Color::from_rgba(25, 25, 40, 230),
        );

        // 标题
        let title = format!("租赁 - {}", self.partner_name);
        draw_text_cn(
            &title,
            dialog_x + 15.0,
            dialog_y + 10.0,
            16.0,
            Color::from_rgba(100, 200, 255, 255),
        );

        // 物品槽位
        let items_y = dialog_y + title_h + padding;
        for (i, entry) in self.items.iter().enumerate() {
            let y = items_y + i as f32 * item_h;

            // 槽位背景
            let slot_bg = if entry.item.is_some() {
                Color::from_rgba(50, 50, 30, 150)
            } else {
                Color::from_rgba(30, 30, 30, 100)
            };
            draw_rectangle(dialog_x + 10.0, y, dialog_w - 20.0, item_h - 3.0, slot_bg);

            // 槽位编号
            draw_text_cn(
                &format!("槽位 {}", i + 1),
                dialog_x + 15.0,
                y + 5.0,
                12.0,
                Color::from_rgba(150, 150, 150, 255),
            );

            // 物品名称
            if let Some(ref item) = entry.item {
                let name = item
                    .info
                    .as_ref()
                    .map(|info| info.name.as_str())
                    .unwrap_or("未知物品");
                draw_text_cn(
                    name,
                    dialog_x + 15.0,
                    y + 18.0,
                    12.0,
                    Color::from_rgba(200, 200, 200, 255),
                );
            } else {
                draw_text_cn(
                    "[空]",
                    dialog_x + 60.0,
                    y + 18.0,
                    11.0,
                    Color::from_rgba(100, 100, 100, 255),
                );
            }
        }

        // 费用/期限
        let info_y = items_y + 6.0 * item_h + padding;
        draw_text_cn(
            &format!("租赁费用: {} 金币 | 期限: {} 天", self.fee, self.period),
            dialog_x + 15.0,
            info_y + 5.0,
            13.0,
            WHITE,
        );

        // 锁定状态
        let lock_y = info_y + 20.0;
        let my_lock = if self.locked_by_me {
            "[已锁定]"
        } else {
            "[未锁定]"
        };
        let partner_lock = if self.locked_by_partner {
            "[已锁定]"
        } else {
            "[未锁定]"
        };
        draw_text_cn(
            &format!(
                "我: {}  |  {}: {}",
                my_lock, self.partner_name, partner_lock
            ),
            dialog_x + 15.0,
            lock_y + 5.0,
            12.0,
            if self.locked_by_me && self.locked_by_partner {
                Color::from_rgba(100, 220, 100, 255)
            } else {
                Color::from_rgba(200, 200, 200, 255)
            },
        );

        // 按钮
        let btn_y = lock_y + 25.0;
        let btn_w = 80.0;
        let btn_gap = 10.0;
        let both_locked = self.locked_by_me && self.locked_by_partner;

        // 锁定/解锁按钮
        let lock_x = dialog_x + padding;
        let lock_label = if self.locked_by_me {
            "解锁"
        } else {
            "锁定"
        };
        draw_rectangle(
            lock_x,
            btn_y,
            btn_w,
            btn_h,
            if self.locked_by_me {
                Color::from_rgba(160, 80, 80, 255)
            } else {
                Color::from_rgba(80, 160, 80, 255)
            },
        );
        draw_text_cn(lock_label, lock_x + 20.0, btn_y + 7.0, 14.0, WHITE);

        if left_clicked
            && mouse_pos.x >= lock_x
            && mouse_pos.x <= lock_x + btn_w
            && mouse_pos.y >= btn_y
            && mouse_pos.y <= btn_y + btn_h
        {
            self.set_locked(!self.locked_by_me);
        }

        // 确认按钮（仅双方都锁定后可用）
        let confirm_x = lock_x + btn_w + btn_gap;
        let can_confirm = both_locked;
        draw_rectangle(
            confirm_x,
            btn_y,
            btn_w,
            btn_h,
            if can_confirm {
                Color::from_rgba(80, 120, 220, 255)
            } else {
                Color::from_rgba(60, 60, 60, 255)
            },
        );
        draw_text_cn("确认", confirm_x + 20.0, btn_y + 7.0, 14.0, WHITE);

        if left_clicked
            && can_confirm
            && mouse_pos.x >= confirm_x
            && mouse_pos.x <= confirm_x + btn_w
            && mouse_pos.y >= btn_y
            && mouse_pos.y <= btn_y + btn_h
        {
            self.confirm_clicked = true;
        }

        // 关闭按钮
        let close_x = confirm_x + btn_w + btn_gap;
        let mouse_over_close = mouse_pos.x >= close_x
            && mouse_pos.x <= close_x + btn_w
            && mouse_pos.y >= btn_y
            && mouse_pos.y <= btn_y + btn_h;
        draw_rectangle(
            close_x,
            btn_y,
            btn_w,
            btn_h,
            if mouse_over_close {
                Color::from_rgba(150, 50, 50, 255)
            } else {
                Color::from_rgba(100, 30, 30, 255)
            },
        );
        draw_text_cn("关闭", close_x + 20.0, btn_y + 7.0, 14.0, WHITE);

        if left_clicked && mouse_over_close {
            self.close();
        }

        mouse_over
    }
}
