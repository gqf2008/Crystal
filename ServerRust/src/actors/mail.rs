// Mail system - 邮件数据结构
// 纯数据结构，由 WorldActor 调用

use mir2_shared::data::item::UserItem;

/// 邮件条目
#[derive(Debug, Clone)]
pub struct MailMessage {
    /// 全局唯一邮件 ID
    pub mail_id: u64,
    /// 发件人名称
    pub sender_name: String,
    /// 收件人名称
    pub receiver_name: String,
    /// 主题
    pub subject: String,
    /// 正文
    pub body: String,
    /// 发送时间戳（Unix 秒）
    pub timestamp: i64,
    /// 是否已读
    pub read: bool,
    /// 附件是否已收取
    pub collected: bool,
    /// 附件金币
    pub gold: u64,
    /// 附件物品（最多 5 个）
    pub items: Vec<UserItem>,
}

/// 玩家收件箱
#[derive(Debug, Clone, Default)]
pub struct Mailbox {
    pub inbox: Vec<MailMessage>,
}

impl Mailbox {
    pub fn new() -> Self {
        Self::default()
    }

    /// 添加邮件到收件箱
    pub fn add_mail(&mut self, mail: MailMessage) {
        self.inbox.push(mail);
    }

    /// 查找邮件（不可变）
    pub fn get_mail(&self, mail_id: u64) -> Option<&MailMessage> {
        self.inbox.iter().find(|m| m.mail_id == mail_id)
    }

    /// 查找邮件（可变）
    pub fn get_mail_mut(&mut self, mail_id: u64) -> Option<&mut MailMessage> {
        self.inbox.iter_mut().find(|m| m.mail_id == mail_id)
    }

    /// 标记已读
    pub fn mark_read(&mut self, mail_id: u64) -> bool {
        if let Some(m) = self.get_mail_mut(mail_id) {
            m.read = true;
            true
        } else {
            false
        }
    }

    /// 标记附件已收取
    pub fn mark_collected(&mut self, mail_id: u64) -> bool {
        if let Some(m) = self.get_mail_mut(mail_id) {
            m.collected = true;
            true
        } else {
            false
        }
    }

    /// 删除邮件
    pub fn delete_mail(&mut self, mail_id: u64) -> bool {
        if let Some(idx) = self.inbox.iter().position(|m| m.mail_id == mail_id) {
            self.inbox.remove(idx);
            true
        } else {
            false
        }
    }

    /// 收取附件（返回金币和物品）
    pub fn collect_attachment(&mut self, mail_id: u64) -> Option<(u64, Vec<UserItem>)> {
        let mail = self.get_mail_mut(mail_id)?;
        if mail.collected {
            return None; // 已收取
        }
        let gold = mail.gold;
        let items = std::mem::take(&mut mail.items);
        mail.collected = true;
        Some((gold, items))
    }

    /// 未读邮件数量
    pub fn unread_count(&self) -> usize {
        self.inbox.iter().filter(|m| !m.read).count()
    }
}

/// 全局邮件 ID 计数器
static NEXT_MAIL_ID: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(1);

pub fn generate_mail_id() -> u64 {
    NEXT_MAIL_ID.fetch_add(1, std::sync::atomic::Ordering::Relaxed)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_mail() -> MailMessage {
        MailMessage {
            mail_id: 1,
            sender_name: "Alice".into(),
            receiver_name: "Bob".into(),
            subject: "Hello".into(),
            body: "Hi there!".into(),
            timestamp: 1000,
            read: false,
            collected: false,
            gold: 100,
            items: vec![],
        }
    }

    #[test]
    fn test_add_and_get_mail() {
        let mut mailbox = Mailbox::new();
        let mail = make_mail();
        mailbox.add_mail(mail);
        assert_eq!(mailbox.inbox.len(), 1);

        let got = mailbox.get_mail(1).unwrap();
        assert_eq!(got.sender_name, "Alice");
        assert!(mailbox.get_mail(999).is_none());
    }

    #[test]
    fn test_mark_read() {
        let mut mailbox = Mailbox::new();
        mailbox.add_mail(make_mail());
        assert!(mailbox.mark_read(1));
        assert!(mailbox.get_mail(1).unwrap().read);
        assert!(!mailbox.mark_read(999));
    }

    #[test]
    fn test_collect_attachments() {
        let mut mailbox = Mailbox::new();
        mailbox.add_mail(make_mail());

        let (gold, items) = mailbox.collect_attachment(1).unwrap();
        assert_eq!(gold, 100);
        assert!(items.is_empty());

        // 第二次收取应失败（已收取）
        assert!(mailbox.collect_attachment(1).is_none());
    }

    #[test]
    fn test_delete_mail() {
        let mut mailbox = Mailbox::new();
        mailbox.add_mail(make_mail());
        mailbox.add_mail(make_mail());
        assert_eq!(mailbox.inbox.len(), 2);

        assert!(mailbox.delete_mail(1));
        assert_eq!(mailbox.inbox.len(), 1);
        assert!(!mailbox.delete_mail(999));
    }

    #[test]
    fn test_unread_count() {
        let mut mailbox = Mailbox::new();
        let mail1 = make_mail();
        let mut mail2 = make_mail();
        mail2.mail_id = 2;
        mail2.read = true;
        mailbox.add_mail(mail1);
        mailbox.add_mail(mail2);
        assert_eq!(mailbox.unread_count(), 1);
    }
}
