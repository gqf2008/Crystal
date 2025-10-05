// Mail List Dialog - 邮件列表对话框
// 对应C#的MailListDialog类

use crate::scenes::dialogs::Dialog;

/// Mail type
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MailType {
    Normal,  // Normal mail
    Gold,    // Contains gold
    Item,    // Contains items
    System,  // System message
}

/// Mail status
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MailStatus {
    Unread,
    Read,
    Locked, // User locked it
}

/// Client mail item
#[derive(Debug, Clone)]
pub struct ClientMail {
    pub mail_id: u64,
    pub mail_type: MailType,
    pub sender_name: String,
    pub subject: String,
    pub message: String,
    pub gold: u32,
    pub item_count: u8,
    pub status: MailStatus,
    pub sent_date: i64,
    pub expiry_date: i64,
}

impl ClientMail {
    pub fn new(mail_id: u64, sender: String, subject: String) -> Self {
        Self {
            mail_id,
            mail_type: MailType::Normal,
            sender_name: sender,
            subject,
            message: String::new(),
            gold: 0,
            item_count: 0,
            status: MailStatus::Unread,
            sent_date: 0,
            expiry_date: 0,
        }
    }

    pub fn mark_read(&mut self) {
        if self.status == MailStatus::Unread {
            self.status = MailStatus::Read;
        }
    }

    pub fn is_unread(&self) -> bool {
        self.status == MailStatus::Unread
    }

    pub fn is_locked(&self) -> bool {
        self.status == MailStatus::Locked
    }

    pub fn lock(&mut self) {
        self.status = MailStatus::Locked;
    }

    pub fn unlock(&mut self) {
        if self.status == MailStatus::Locked {
            self.status = MailStatus::Read;
        }
    }
}

/// Mail List Dialog - View inbox
pub struct MailListDialog {
    visible: bool,
    pub mails: Vec<ClientMail>,
    pub selected_mail: Option<usize>,
    pub current_page: usize,
    pub rows_per_page: usize, // 10 in C#
    pub start_index: usize,
}

impl MailListDialog {
    const MAX_ROWS: usize = 10;

    pub fn new() -> Self {
        Self {
            visible: false,
            mails: Vec::new(),
            selected_mail: None,
            current_page: 1,
            rows_per_page: Self::MAX_ROWS,
            start_index: 0,
        }
    }

    pub fn add_mail(&mut self, mail: ClientMail) {
        self.mails.push(mail);
    }

    pub fn remove_mail(&mut self, mail_id: u64) -> bool {
        if let Some(pos) = self.mails.iter().position(|m| m.mail_id == mail_id) {
            self.mails.remove(pos);
            if self.selected_mail == Some(pos) {
                self.selected_mail = None;
            }
            true
        } else {
            false
        }
    }

    pub fn find_mail(&self, mail_id: u64) -> Option<&ClientMail> {
        self.mails.iter().find(|m| m.mail_id == mail_id)
    }

    pub fn find_mail_mut(&mut self, mail_id: u64) -> Option<&mut ClientMail> {
        self.mails.iter_mut().find(|m| m.mail_id == mail_id)
    }

    pub fn select_mail(&mut self, index: usize) -> bool {
        if index < self.mails.len() {
            self.selected_mail = Some(index);
            // Mark as read
            if let Some(mail) = self.mails.get_mut(index) {
                mail.mark_read();
            }
            true
        } else {
            false
        }
    }

    pub fn get_selected_mail(&self) -> Option<&ClientMail> {
        self.selected_mail.and_then(|idx| self.mails.get(idx))
    }

    pub fn get_selected_mail_mut(&mut self) -> Option<&mut ClientMail> {
        self.selected_mail.and_then(|idx| self.mails.get_mut(idx))
    }

    pub fn delete_selected(&mut self) -> Option<u64> {
        if let Some(idx) = self.selected_mail {
            if idx < self.mails.len() {
                let mail = self.mails.remove(idx);
                self.selected_mail = None;
                return Some(mail.mail_id);
            }
        }
        None
    }

    pub fn get_visible_mails(&self) -> Vec<&ClientMail> {
        self.mails
            .iter()
            .skip(self.start_index)
            .take(self.rows_per_page)
            .collect()
    }

    pub fn next_page(&mut self) -> bool {
        let total_pages = self.total_pages();
        if self.current_page < total_pages {
            self.current_page += 1;
            self.start_index += self.rows_per_page;
            self.selected_mail = None;
            true
        } else {
            false
        }
    }

    pub fn previous_page(&mut self) -> bool {
        if self.current_page > 1 {
            self.current_page -= 1;
            self.start_index = self.start_index.saturating_sub(self.rows_per_page);
            self.selected_mail = None;
            true
        } else {
            false
        }
    }

    pub fn total_pages(&self) -> usize {
        (self.mails.len() + self.rows_per_page - 1) / self.rows_per_page
    }

    pub fn unread_count(&self) -> usize {
        self.mails.iter().filter(|m| m.is_unread()).count()
    }

    pub fn total_mail_count(&self) -> usize {
        self.mails.len()
    }

    pub fn clear_all(&mut self) {
        self.mails.clear();
        self.selected_mail = None;
        self.current_page = 1;
        self.start_index = 0;
    }

    pub fn can_reply(&self) -> bool {
        if let Some(mail) = self.get_selected_mail() {
            mail.mail_type != MailType::System
        } else {
            false
        }
    }
}

impl Dialog for MailListDialog {
    fn show(&mut self) {
        self.visible = true;
    }

    fn hide(&mut self) {
        self.visible = false;
    }

    fn update(&mut self, _delta_time: f32) {
        // Update logic
    }

    fn draw(&self) {
        if !self.visible {
            return;
        }
        // Draw logic
    }

    fn is_visible(&self) -> bool {
        self.visible
    }

    fn toggle(&mut self) {
        self.visible = !self.visible;
    }

    fn name(&self) -> &str {
        "MailListDialog"
    }

    fn contains_point(&self, x: i32, y: i32) -> bool {
        x >= 0 && x < 312 && y >= 0 && y < 444
    }

    fn position(&self) -> (i32, i32) {
        (0, 0)
    }

    fn size(&self) -> (i32, i32) {
        (312, 444)
    }
}

impl Default for MailListDialog {
    fn default() -> Self {
        Self::new()
    }
}