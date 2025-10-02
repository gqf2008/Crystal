// MailDialog - Mail system for sending and receiving messages
// Rust implementation of Client/MirScenes/Dialogs/MailDialogs.cs

use crate::game::scenes::dialogs::Dialog;

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

    pub fn is_unread(&self) -> bool {
        self.status == MailStatus::Unread
    }

    pub fn mark_read(&mut self) {
        if self.status == MailStatus::Unread {
            self.status = MailStatus::Read;
        }
    }

    pub fn is_locked(&self) -> bool {
        self.status == MailStatus::Locked
    }

    pub fn toggle_lock(&mut self) {
        self.status = match self.status {
            MailStatus::Locked => MailStatus::Read,
            _ => MailStatus::Locked,
        };
    }

    pub fn has_attachments(&self) -> bool {
        self.gold > 0 || self.item_count > 0
    }

    pub fn get_type_icon(&self) -> u16 {
        match self.mail_type {
            MailType::Normal => 0,
            MailType::Gold => 1,
            MailType::Item => 2,
            MailType::System => 3,
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
}

/// Mail Compose Dialog - Write new mail
pub struct MailComposeDialog {
    visible: bool,
    pub recipient: String,
    pub subject: String,
    pub message: String,
    pub gold: u32,
    pub max_message_length: usize, // 1000 in C#
}

impl MailComposeDialog {
    pub fn new() -> Self {
        Self {
            visible: false,
            recipient: String::new(),
            subject: String::new(),
            message: String::new(),
            gold: 0,
            max_message_length: 1000,
        }
    }

    pub fn compose_mail(&mut self, recipient: String) {
        self.recipient = recipient;
        self.subject.clear();
        self.message.clear();
        self.gold = 0;
        self.visible = true;
    }

    pub fn set_recipient(&mut self, name: String) {
        self.recipient = name;
    }

    pub fn set_subject(&mut self, subject: String) {
        self.subject = subject;
    }

    pub fn set_message(&mut self, message: String) {
        if message.len() <= self.max_message_length {
            self.message = message;
        } else {
            self.message = message[..self.max_message_length].to_string();
        }
    }

    pub fn set_gold(&mut self, gold: u32) {
        self.gold = gold;
    }

    pub fn can_send(&self) -> bool {
        !self.recipient.is_empty() 
            && !self.subject.is_empty() 
            && !self.message.is_empty()
    }

    pub fn reset(&mut self) {
        self.recipient.clear();
        self.subject.clear();
        self.message.clear();
        self.gold = 0;
    }

    pub fn get_remaining_chars(&self) -> usize {
        self.max_message_length.saturating_sub(self.message.len())
    }
}

impl Dialog for MailComposeDialog {
    fn show(&mut self) {
        self.visible = true;
    }

    fn hide(&mut self) {
        self.visible = false;
        self.reset();
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
}

#[cfg(test)]
mod tests {
    use super::*;

    fn create_test_mail(id: u64, sender: &str, subject: &str) -> ClientMail {
        ClientMail::new(id, sender.to_string(), subject.to_string())
    }

    #[test]
    fn test_client_mail_creation() {
        let mail = create_test_mail(1, "Alice", "Hello");
        assert_eq!(mail.mail_id, 1);
        assert_eq!(mail.sender_name, "Alice");
        assert!(mail.is_unread());
    }

    #[test]
    fn test_mail_mark_read() {
        let mut mail = create_test_mail(1, "Bob", "Test");
        assert!(mail.is_unread());
        
        mail.mark_read();
        assert!(!mail.is_unread());
        assert_eq!(mail.status, MailStatus::Read);
    }

    #[test]
    fn test_mail_lock() {
        let mut mail = create_test_mail(1, "Charlie", "Important");
        assert!(!mail.is_locked());
        
        mail.toggle_lock();
        assert!(mail.is_locked());
        
        mail.toggle_lock();
        assert!(!mail.is_locked());
    }

    #[test]
    fn test_mail_attachments() {
        let mut mail = create_test_mail(1, "Dave", "Gift");
        assert!(!mail.has_attachments());
        
        mail.gold = 100;
        assert!(mail.has_attachments());
    }

    #[test]
    fn test_mail_list_dialog_creation() {
        let dialog = MailListDialog::new();
        assert!(!dialog.is_visible());
        assert_eq!(dialog.total_mail_count(), 0);
    }

    #[test]
    fn test_add_remove_mail() {
        let mut dialog = MailListDialog::new();
        let mail = create_test_mail(1, "Eve", "Test");
        
        dialog.add_mail(mail);
        assert_eq!(dialog.total_mail_count(), 1);
        
        assert!(dialog.remove_mail(1));
        assert_eq!(dialog.total_mail_count(), 0);
    }

    #[test]
    fn test_select_mail() {
        let mut dialog = MailListDialog::new();
        dialog.add_mail(create_test_mail(1, "Alice", "Mail 1"));
        dialog.add_mail(create_test_mail(2, "Bob", "Mail 2"));
        
        assert!(dialog.select_mail(0));
        assert!(dialog.get_selected_mail().is_some());
        assert_eq!(dialog.get_selected_mail().unwrap().mail_id, 1);
        
        // Should be marked as read
        assert!(!dialog.get_selected_mail().unwrap().is_unread());
    }

    #[test]
    fn test_delete_selected() {
        let mut dialog = MailListDialog::new();
        dialog.add_mail(create_test_mail(1, "Alice", "Test"));
        dialog.select_mail(0);
        
        let deleted_id = dialog.delete_selected();
        assert_eq!(deleted_id, Some(1));
        assert_eq!(dialog.total_mail_count(), 0);
    }

    #[test]
    fn test_mail_pagination() {
        let mut dialog = MailListDialog::new();
        for i in 0..25 {
            dialog.add_mail(create_test_mail(i, "Sender", &format!("Mail {}", i)));
        }
        
        assert_eq!(dialog.current_page, 1);
        assert_eq!(dialog.total_pages(), 3);
        
        assert!(dialog.next_page());
        assert_eq!(dialog.current_page, 2);
        
        assert!(dialog.previous_page());
        assert_eq!(dialog.current_page, 1);
    }

    #[test]
    fn test_unread_count() {
        let mut dialog = MailListDialog::new();
        dialog.add_mail(create_test_mail(1, "A", "Mail 1"));
        dialog.add_mail(create_test_mail(2, "B", "Mail 2"));
        
        assert_eq!(dialog.unread_count(), 2);
        
        dialog.select_mail(0); // Marks as read
        assert_eq!(dialog.unread_count(), 1);
    }

    #[test]
    fn test_visible_mails() {
        let mut dialog = MailListDialog::new();
        for i in 0..15 {
            dialog.add_mail(create_test_mail(i, "Sender", &format!("Mail {}", i)));
        }
        
        let visible = dialog.get_visible_mails();
        assert_eq!(visible.len(), 10); // MAX_ROWS
    }

    #[test]
    fn test_can_reply() {
        let mut dialog = MailListDialog::new();
        let mut mail = create_test_mail(1, "Alice", "Test");
        mail.mail_type = MailType::Normal;
        dialog.add_mail(mail);
        
        dialog.select_mail(0);
        assert!(dialog.can_reply());
        
        // System mail cannot be replied to
        let mut system_mail = create_test_mail(2, "System", "Notice");
        system_mail.mail_type = MailType::System;
        dialog.add_mail(system_mail);
        dialog.select_mail(1);
        assert!(!dialog.can_reply());
    }

    #[test]
    fn test_mail_compose_dialog() {
        let dialog = MailComposeDialog::new();
        assert!(!dialog.is_visible());
        assert!(!dialog.can_send());
    }

    #[test]
    fn test_compose_mail() {
        let mut dialog = MailComposeDialog::new();
        dialog.compose_mail("Alice".to_string());
        
        assert!(dialog.is_visible());
        assert_eq!(dialog.recipient, "Alice");
        assert!(dialog.subject.is_empty());
    }

    #[test]
    fn test_set_message() {
        let mut dialog = MailComposeDialog::new();
        let short_msg = "Hello!";
        dialog.set_message(short_msg.to_string());
        assert_eq!(dialog.message, "Hello!");
        
        // Test max length
        let long_msg = "A".repeat(2000);
        dialog.set_message(long_msg);
        assert_eq!(dialog.message.len(), 1000);
    }

    #[test]
    fn test_can_send() {
        let mut dialog = MailComposeDialog::new();
        assert!(!dialog.can_send());
        
        dialog.set_recipient("Bob".to_string());
        assert!(!dialog.can_send());
        
        dialog.set_subject("Test".to_string());
        assert!(!dialog.can_send());
        
        dialog.set_message("Message body".to_string());
        assert!(dialog.can_send());
    }

    #[test]
    fn test_remaining_chars() {
        let mut dialog = MailComposeDialog::new();
        assert_eq!(dialog.get_remaining_chars(), 1000);
        
        dialog.set_message("Hello".to_string());
        assert_eq!(dialog.get_remaining_chars(), 995);
    }

    #[test]
    fn test_reset_compose() {
        let mut dialog = MailComposeDialog::new();
        dialog.set_recipient("Alice".to_string());
        dialog.set_subject("Test".to_string());
        dialog.set_message("Body".to_string());
        dialog.set_gold(100);
        
        dialog.reset();
        assert!(dialog.recipient.is_empty());
        assert!(dialog.subject.is_empty());
        assert!(dialog.message.is_empty());
        assert_eq!(dialog.gold, 0);
    }

    #[test]
    fn test_find_mail() {
        let mut dialog = MailListDialog::new();
        dialog.add_mail(create_test_mail(10, "Alice", "Test"));
        
        assert!(dialog.find_mail(10).is_some());
        assert!(dialog.find_mail(99).is_none());
    }
}
