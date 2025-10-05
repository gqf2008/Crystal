// Mail Compose Letter Dialog - 邮件撰写信件对话框
// 对应C#的MailComposeLetterDialog类

use crate::scenes::dialogs::Dialog;

/// Mail Compose Dialog - Compose and send mail
pub struct MailComposeLetterDialog {
    visible: bool,
    pub recipient: String,
    pub subject: String,
    pub message: String,
    pub gold: u32,
    pub max_message_length: usize, // 1000 in C#
}

impl MailComposeLetterDialog {
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

impl Dialog for MailComposeLetterDialog {
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

    fn toggle(&mut self) {
        self.visible = !self.visible;
    }

    fn name(&self) -> &str { "MailComposeLetterDialog" }
    fn contains_point(&self, x: i32, y: i32) -> bool { x >= 0 && x < 400 && y >= 0 && y < 350 }
    fn position(&self) -> (i32, i32) { (0, 0) }
    fn size(&self) -> (i32, i32) { (400, 350) }
}

impl Default for MailComposeLetterDialog {
    fn default() -> Self {
        Self::new()
    }
}