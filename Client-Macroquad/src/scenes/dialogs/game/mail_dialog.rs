// ============================================================================
// MailDialogHybrid - 邮件系统对话框（对齐 C# MailDialogs.cs）
// ============================================================================
//
// C# 参考：Client/MirScenes/Dialogs/MailDialogs.cs
// - MailListDialog: 邮件列表（发件人/主题/日期/已读未读），分页，打开/删除/写信
// - MailComposeDialog: 收件人/主题/正文，金币/物品附件，发送/取消
// - MailReadDialog: 显示发件人/主题/正文，领取金币/物品附件，回复/删除
//
// ============================================================================

use macroquad::prelude::*;
use super::native_ui_utils::*;

// ============================================================================
// 常量
// ============================================================================

const MAIL_LIST_ROWS: usize = 10;
const MAIL_LIST_WIDTH: f32 = 310.0;
const MAIL_LIST_HEIGHT: f32 = 340.0;
const MAIL_COMPOSE_WIDTH: f32 = 280.0;
const MAIL_COMPOSE_HEIGHT: f32 = 320.0;
const MAIL_READ_WIDTH: f32 = 280.0;
const MAIL_READ_HEIGHT: f32 = 320.0;
const ROW_HEIGHT: f32 = 24.0;

// ============================================================================
// 类型定义
// ============================================================================

/// 邮件列表条目
#[derive(Debug, Clone)]
pub struct MailEntry {
    pub id: u64,
    pub sender: String,
    pub subject: String,
    pub date_str: String,
    pub is_read: bool,
    pub has_gold: bool,
    pub has_item: bool,
}

impl MailEntry {
    pub fn new(id: u64, sender: &str, subject: &str, date_str: &str) -> Self {
        Self {
            id,
            sender: sender.to_string(),
            subject: subject.to_string(),
            date_str: date_str.to_string(),
            is_read: false,
            has_gold: false,
            has_item: false,
        }
    }
}

/// 邮件操作事件
#[derive(Debug, Clone, PartialEq)]
pub enum MailAction {
    /// 打开邮件
    Open(u64),
    /// 删除邮件
    Delete(u64),
    /// 撰写新邮件
    Compose,
    /// 发送邮件
    Send {
        to: String,
        subject: String,
        body: String,
        gold: u64,
    },
    /// 回复邮件
    Reply(u64),
    /// 领取金币
    TakeGold(u64),
    /// 领取物品
    TakeItem(u64),
    /// 关闭
    Close,
}

// ============================================================================
// MailListDialogHybrid
// ============================================================================

/// 邮件列表对话框
pub struct MailListDialogHybrid {
    pub visible: bool,
    pub mails: Vec<MailEntry>,
    pub selected_index: Option<usize>,
    page: usize,
    position: Vec2,
    drag_helper: DragHelper,
}

impl MailListDialogHybrid {
    pub fn new() -> Self {
        Self {
            visible: false,
            mails: Vec::new(),
            selected_index: None,
            page: 0,
            position: Vec2::new(200.0, 100.0),
            drag_helper: DragHelper::new(),
        }
    }

    /// 当前页邮件数量
    pub fn page_count(&self) -> usize {
        if self.mails.is_empty() {
            0
        } else {
            (self.mails.len() - 1) / MAIL_LIST_ROWS + 1
        }
    }

    /// 当前页的邮件切片
    pub fn current_page_mails(&self) -> &[MailEntry] {
        let start = self.page * MAIL_LIST_ROWS;
        let end = (start + MAIL_LIST_ROWS).min(self.mails.len());
        if start >= self.mails.len() {
            &[]
        } else {
            &self.mails[start..end]
        }
    }

    /// 未读邮件数
    pub fn unread_count(&self) -> usize {
        self.mails.iter().filter(|m| !m.is_read).count()
    }

    /// 绘制并处理输入
    pub fn draw(&mut self) -> Option<MailAction> {
        if !self.visible {
            return None;
        }

        let mouse = mouse_pos();
        let mut action = None;

        // 拖动
        let title_rect = Rect::new(self.position.x, self.position.y, MAIL_LIST_WIDTH, 22.0);
        self.position = self.drag_helper.update(title_rect, self.position, mouse);

        let x = self.position.x;
        let y = self.position.y;

        // 背景
        draw_rectangle(x, y, MAIL_LIST_WIDTH, MAIL_LIST_HEIGHT, Color::new(0.0, 0.0, 0.0, 0.85));
        draw_rectangle_lines(x, y, MAIL_LIST_WIDTH, MAIL_LIST_HEIGHT, 1.0, DARKGRAY);

        // 标题
        draw_text(&format!("邮件 ({})", self.mails.len()), x + 10.0, y + 16.0, 14.0, GOLD);

        // 邮件列表
        let page_start = self.page * MAIL_LIST_ROWS;
        let page_end = (page_start + MAIL_LIST_ROWS).min(self.mails.len());
        let page_range = if page_start < self.mails.len() { page_start..page_end } else { 0..0 };
        for (i, idx) in page_range.enumerate() {
            let mail_id = self.mails[idx].id;
            let is_read = self.mails[idx].is_read;
            let has_attachment = self.mails[idx].has_gold || self.mails[idx].has_item;
            let sender = self.mails[idx].sender.clone();
            let subject = self.mails[idx].subject.clone();
            let date_str = self.mails[idx].date_str.clone();

            let row_y = y + 30.0 + i as f32 * ROW_HEIGHT;
            let row_rect = Rect::new(x + 4.0, row_y, MAIL_LIST_WIDTH - 8.0, ROW_HEIGHT);

            let bg_color = if self.selected_index == Some(idx) {
                Color::new(0.3, 0.3, 0.5, 0.6)
            } else if row_rect.contains(mouse) {
                Color::new(0.2, 0.2, 0.3, 0.4)
            } else {
                Color::new(0.0, 0.0, 0.0, 0.0)
            };
            draw_rectangle(row_rect.x, row_rect.y, row_rect.w, row_rect.h, bg_color);

            let text_color = if is_read { GRAY } else { WHITE };
            let indicator = if has_attachment { "* " } else { "" };
            draw_text(
                &format!("{}{} - {}", indicator, sender, subject),
                x + 8.0,
                row_y + 16.0,
                11.0,
                text_color,
            );
            draw_text(&date_str, x + MAIL_LIST_WIDTH - 80.0, row_y + 16.0, 10.0, DARKGRAY);

            if is_mouse_over(row_rect) && is_mouse_button_pressed(MouseButton::Left) {
                self.selected_index = Some(idx);
                action = Some(MailAction::Open(mail_id));
            }
        }

        // 分页按钮
        let btn_y = y + MAIL_LIST_HEIGHT - 30.0;
        let prev_rect = Rect::new(x + 10.0, btn_y, 50.0, 20.0);
        draw_rectangle_lines(prev_rect.x, prev_rect.y, prev_rect.w, prev_rect.h, 1.0, GRAY);
        draw_text("上一页", prev_rect.x + 6.0, prev_rect.y + 14.0, 11.0, GRAY);
        if is_mouse_over(prev_rect) && is_mouse_button_pressed(MouseButton::Left) && self.page > 0 {
            self.page -= 1;
        }

        let next_rect = Rect::new(x + 70.0, btn_y, 50.0, 20.0);
        draw_rectangle_lines(next_rect.x, next_rect.y, next_rect.w, next_rect.h, 1.0, GRAY);
        draw_text("下一页", next_rect.x + 6.0, next_rect.y + 14.0, 11.0, GRAY);
        if is_mouse_over(next_rect) && is_mouse_button_pressed(MouseButton::Left) && self.page + 1 < self.page_count() {
            self.page += 1;
        }

        // 删除按钮
        let del_rect = Rect::new(x + 140.0, btn_y, 50.0, 20.0);
        draw_rectangle_lines(del_rect.x, del_rect.y, del_rect.w, del_rect.h, 1.0, GRAY);
        draw_text("删除", del_rect.x + 12.0, del_rect.y + 14.0, 11.0, GRAY);
        if is_mouse_over(del_rect) && is_mouse_button_pressed(MouseButton::Left) {
            if let Some(idx) = self.selected_index {
                if let Some(mail) = self.mails.get(idx) {
                    action = Some(MailAction::Delete(mail.id));
                }
            }
        }

        // 写信按钮
        let compose_rect = Rect::new(x + 200.0, btn_y, 50.0, 20.0);
        draw_rectangle_lines(compose_rect.x, compose_rect.y, compose_rect.w, compose_rect.h, 1.0, GRAY);
        draw_text("写信", compose_rect.x + 12.0, compose_rect.y + 14.0, 11.0, GRAY);
        if is_mouse_over(compose_rect) && is_mouse_button_pressed(MouseButton::Left) {
            action = Some(MailAction::Compose);
        }

        // 关闭按钮
        let close_rect = Rect::new(x + MAIL_LIST_WIDTH - 20.0, y + 2.0, 16.0, 16.0);
        draw_text("X", close_rect.x + 3.0, close_rect.y + 12.0, 12.0, GRAY);
        if is_mouse_over(close_rect) && is_mouse_button_pressed(MouseButton::Left) {
            self.visible = false;
            action = Some(MailAction::Close);
        }

        action
    }
}

// ============================================================================
// MailComposeDialogHybrid
// ============================================================================

/// 邮件撰写对话框
pub struct MailComposeDialogHybrid {
    pub visible: bool,
    pub to_field: String,
    pub subject_field: String,
    pub body_field: String,
    pub gold_amount: u64,
    pub has_item_attachment: bool,
    position: Vec2,
    drag_helper: DragHelper,
}

impl MailComposeDialogHybrid {
    pub fn new() -> Self {
        Self {
            visible: false,
            to_field: String::new(),
            subject_field: String::new(),
            body_field: String::new(),
            gold_amount: 0,
            has_item_attachment: false,
            position: Vec2::new(250.0, 120.0),
            drag_helper: DragHelper::new(),
        }
    }

    /// 重置所有字段
    pub fn reset(&mut self) {
        self.to_field.clear();
        self.subject_field.clear();
        self.body_field.clear();
        self.gold_amount = 0;
        self.has_item_attachment = false;
    }

    /// 绘制并处理输入
    pub fn draw(&mut self) -> Option<MailAction> {
        if !self.visible {
            return None;
        }

        let mouse = mouse_pos();
        let mut action = None;

        let title_rect = Rect::new(self.position.x, self.position.y, MAIL_COMPOSE_WIDTH, 22.0);
        self.position = self.drag_helper.update(title_rect, self.position, mouse);

        let x = self.position.x;
        let y = self.position.y;

        // 背景
        draw_rectangle(x, y, MAIL_COMPOSE_WIDTH, MAIL_COMPOSE_HEIGHT, Color::new(0.0, 0.0, 0.0, 0.85));
        draw_rectangle_lines(x, y, MAIL_COMPOSE_WIDTH, MAIL_COMPOSE_HEIGHT, 1.0, DARKGRAY);

        // 标题
        draw_text("撰写邮件", x + 10.0, y + 16.0, 14.0, GOLD);

        // 字段标签
        draw_text("收件人:", x + 10.0, y + 46.0, 12.0, WHITE);
        draw_rectangle(x + 70.0, y + 34.0, 190.0, 18.0, Color::new(0.15, 0.15, 0.15, 1.0));
        draw_text(&self.to_field, x + 74.0, y + 46.0, 11.0, WHITE);

        draw_text("主  题:", x + 10.0, y + 72.0, 12.0, WHITE);
        draw_rectangle(x + 70.0, y + 60.0, 190.0, 18.0, Color::new(0.15, 0.15, 0.15, 1.0));
        draw_text(&self.subject_field, x + 74.0, y + 72.0, 11.0, WHITE);

        draw_text("正  文:", x + 10.0, y + 98.0, 12.0, WHITE);
        draw_rectangle(x + 10.0, y + 106.0, 260.0, 120.0, Color::new(0.15, 0.15, 0.15, 1.0));
        draw_text(&self.body_field, x + 14.0, y + 120.0, 11.0, WHITE);

        // 附件区域
        draw_text(&format!("金币: {}", self.gold_amount), x + 10.0, y + 248.0, 11.0, GOLD);

        let item_label = if self.has_item_attachment { "物品: [已附加]" } else { "物品: [空]" };
        draw_text(item_label, x + 140.0, y + 248.0, 11.0, GRAY);

        // 发送按钮
        let btn_y = y + MAIL_COMPOSE_HEIGHT - 30.0;
        let send_rect = Rect::new(x + 60.0, btn_y, 60.0, 20.0);
        draw_rectangle_lines(send_rect.x, send_rect.y, send_rect.w, send_rect.h, 1.0, GRAY);
        draw_text("发送", send_rect.x + 16.0, send_rect.y + 14.0, 11.0, GRAY);
        if is_mouse_over(send_rect) && is_mouse_button_pressed(MouseButton::Left) {
            action = Some(MailAction::Send {
                to: self.to_field.clone(),
                subject: self.subject_field.clone(),
                body: self.body_field.clone(),
                gold: self.gold_amount,
            });
        }

        // 取消按钮
        let cancel_rect = Rect::new(x + 150.0, btn_y, 60.0, 20.0);
        draw_rectangle_lines(cancel_rect.x, cancel_rect.y, cancel_rect.w, cancel_rect.h, 1.0, GRAY);
        draw_text("取消", cancel_rect.x + 16.0, cancel_rect.y + 14.0, 11.0, GRAY);
        if is_mouse_over(cancel_rect) && is_mouse_button_pressed(MouseButton::Left) {
            self.visible = false;
            action = Some(MailAction::Close);
        }

        action
    }
}

// ============================================================================
// MailReadDialogHybrid
// ============================================================================

/// 邮件阅读对话框
pub struct MailReadDialogHybrid {
    pub visible: bool,
    pub mail_id: u64,
    pub sender: String,
    pub subject: String,
    pub body: String,
    pub gold_amount: u64,
    pub has_item: bool,
    position: Vec2,
    drag_helper: DragHelper,
}

impl MailReadDialogHybrid {
    pub fn new() -> Self {
        Self {
            visible: false,
            mail_id: 0,
            sender: String::new(),
            subject: String::new(),
            body: String::new(),
            gold_amount: 0,
            has_item: false,
            position: Vec2::new(260.0, 110.0),
            drag_helper: DragHelper::new(),
        }
    }

    /// 加载邮件内容
    pub fn load_mail(&mut self, id: u64, sender: &str, subject: &str, body: &str, gold: u64, has_item: bool) {
        self.mail_id = id;
        self.sender = sender.to_string();
        self.subject = subject.to_string();
        self.body = body.to_string();
        self.gold_amount = gold;
        self.has_item = has_item;
    }

    /// 绘制并处理输入
    pub fn draw(&mut self) -> Option<MailAction> {
        if !self.visible {
            return None;
        }

        let mouse = mouse_pos();
        let mut action = None;

        let title_rect = Rect::new(self.position.x, self.position.y, MAIL_READ_WIDTH, 22.0);
        self.position = self.drag_helper.update(title_rect, self.position, mouse);

        let x = self.position.x;
        let y = self.position.y;

        // 背景
        draw_rectangle(x, y, MAIL_READ_WIDTH, MAIL_READ_HEIGHT, Color::new(0.0, 0.0, 0.0, 0.85));
        draw_rectangle_lines(x, y, MAIL_READ_WIDTH, MAIL_READ_HEIGHT, 1.0, DARKGRAY);

        // 标题
        draw_text("阅读邮件", x + 10.0, y + 16.0, 14.0, GOLD);

        // 发件人 & 主题
        draw_text(&format!("发件人: {}", self.sender), x + 10.0, y + 40.0, 12.0, WHITE);
        draw_text(&format!("主  题: {}", self.subject), x + 10.0, y + 58.0, 12.0, WHITE);

        // 正文
        draw_rectangle(x + 10.0, y + 70.0, 260.0, 140.0, Color::new(0.15, 0.15, 0.15, 1.0));
        draw_text(&self.body, x + 14.0, y + 86.0, 11.0, WHITE);

        // 附件
        if self.gold_amount > 0 {
            let gold_rect = Rect::new(x + 10.0, y + 220.0, 120.0, 20.0);
            draw_text(&format!("金币: {}", self.gold_amount), gold_rect.x, gold_rect.y + 14.0, 11.0, GOLD);
            let take_gold_rect = Rect::new(x + 130.0, y + 220.0, 50.0, 18.0);
            draw_rectangle_lines(take_gold_rect.x, take_gold_rect.y, take_gold_rect.w, take_gold_rect.h, 1.0, GOLD);
            draw_text("领取", take_gold_rect.x + 10.0, take_gold_rect.y + 13.0, 10.0, GOLD);
            if is_mouse_over(take_gold_rect) && is_mouse_button_pressed(MouseButton::Left) {
                action = Some(MailAction::TakeGold(self.mail_id));
            }
        }

        if self.has_item {
            let take_item_rect = Rect::new(x + 200.0, y + 220.0, 60.0, 18.0);
            draw_rectangle_lines(take_item_rect.x, take_item_rect.y, take_item_rect.w, take_item_rect.h, 1.0, LIME);
            draw_text("领取物品", take_item_rect.x + 4.0, take_item_rect.y + 13.0, 10.0, LIME);
            if is_mouse_over(take_item_rect) && is_mouse_button_pressed(MouseButton::Left) {
                action = Some(MailAction::TakeItem(self.mail_id));
            }
        }

        // 操作按钮
        let btn_y = y + MAIL_READ_HEIGHT - 30.0;
        let reply_rect = Rect::new(x + 30.0, btn_y, 60.0, 20.0);
        draw_rectangle_lines(reply_rect.x, reply_rect.y, reply_rect.w, reply_rect.h, 1.0, GRAY);
        draw_text("回复", reply_rect.x + 16.0, reply_rect.y + 14.0, 11.0, GRAY);
        if is_mouse_over(reply_rect) && is_mouse_button_pressed(MouseButton::Left) {
            action = Some(MailAction::Reply(self.mail_id));
        }

        let del_rect = Rect::new(x + 110.0, btn_y, 60.0, 20.0);
        draw_rectangle_lines(del_rect.x, del_rect.y, del_rect.w, del_rect.h, 1.0, GRAY);
        draw_text("删除", del_rect.x + 16.0, del_rect.y + 14.0, 11.0, GRAY);
        if is_mouse_over(del_rect) && is_mouse_button_pressed(MouseButton::Left) {
            action = Some(MailAction::Delete(self.mail_id));
        }

        let close_rect = Rect::new(x + 190.0, btn_y, 60.0, 20.0);
        draw_rectangle_lines(close_rect.x, close_rect.y, close_rect.w, close_rect.h, 1.0, GRAY);
        draw_text("关闭", close_rect.x + 16.0, close_rect.y + 14.0, 11.0, GRAY);
        if is_mouse_over(close_rect) && is_mouse_button_pressed(MouseButton::Left) {
            self.visible = false;
            action = Some(MailAction::Close);
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
    fn test_mail_entry_creation() {
        let entry = MailEntry::new(1, "Player1", "Hello", "2024-01-01");
        assert_eq!(entry.id, 1);
        assert_eq!(entry.sender, "Player1");
        assert!(!entry.is_read);
        assert!(!entry.has_gold);
        assert!(!entry.has_item);
    }

    #[test]
    fn test_mail_list_pagination() {
        let mut dialog = MailListDialogHybrid::new();
        assert_eq!(dialog.page_count(), 0);

        for i in 0..25 {
            dialog.mails.push(MailEntry::new(i, "Sender", &format!("Mail {}", i), "2024-01-01"));
        }
        assert_eq!(dialog.page_count(), 3); // 25 mails / 10 per page = 3 pages

        let page0 = dialog.current_page_mails();
        assert_eq!(page0.len(), 10);
    }

    #[test]
    fn test_mail_unread_count() {
        let mut dialog = MailListDialogHybrid::new();
        dialog.mails.push(MailEntry::new(1, "A", "S1", "2024-01-01"));
        dialog.mails.push(MailEntry::new(2, "B", "S2", "2024-01-01"));
        dialog.mails[0].is_read = true;
        assert_eq!(dialog.unread_count(), 1);
    }

    #[test]
    fn test_mail_compose_reset() {
        let mut dialog = MailComposeDialogHybrid::new();
        dialog.to_field = "Player1".to_string();
        dialog.subject_field = "Test".to_string();
        dialog.gold_amount = 100;
        dialog.reset();
        assert!(dialog.to_field.is_empty());
        assert!(dialog.subject_field.is_empty());
        assert_eq!(dialog.gold_amount, 0);
    }
}
