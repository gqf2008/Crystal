// Mail system - 邮件数据结构
// 纯数据结构，由 WorldActor 调用

use mir2_shared::data::item::UserItem;

/// 邮件条目
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
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
    /// 邮件是否已锁定（无法删除/修改）
    pub locked: bool,
    /// 附件金币
    pub gold: u64,
    /// 附件物品（最多 5 个）
    pub items: Vec<UserItem>,
}

/// 玩家收件箱
#[derive(Debug, Clone, Default, serde::Serialize, serde::Deserialize)]
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
    /// C# CollectMail：collected=true 才可领取（false=仍在邮局，需先 [@COLLECTPARCEL] 取回）
    pub fn collect_attachment(&mut self, mail_id: u64) -> Option<(u64, Vec<UserItem>)> {
        let mail = self.get_mail_mut(mail_id)?;
        if !mail.collected {
            return None; // 未从邮局取回
        }
        let gold = mail.gold;
        let items = std::mem::take(&mut mail.items);
        mail.gold = 0;
        mail.collected = true;
        Some((gold, items))
    }

    /// C# NPCScript CollectParcelKey：把所有包裹从邮局取回（collected=true），不转移金币/物品。
    /// 返回被取回的邮件数。
    pub fn release_parcels(&mut self) -> usize {
        let mut released = 0;
        for m in &mut self.inbox {
            if !m.collected && (!m.items.is_empty() || m.gold > 0) {
                m.collected = true;
                released += 1;
            }
        }
        released
    }

    /// 未读邮件数量
    pub fn unread_count(&self) -> usize {
        self.inbox.iter().filter(|m| !m.read).count()
    }

    /// #2382：收件箱超容量时清理“已读+已收取+无附件”的最旧邮件（C# Envir :3545-3554）
    pub fn trim_to_capacity(&mut self, capacity: usize) -> usize {
        let mut removed = 0usize;
        while self.inbox.len() > capacity {
            match self
                .inbox
                .iter()
                .position(|m| m.read && m.collected && m.items.is_empty() && m.gold == 0)
            {
                Some(pos) => {
                    self.inbox.remove(pos);
                    removed += 1;
                }
                None => break,
            }
        }
        removed
    }
}

/// C# MailInfo.Send()：发信时初始 Collected 计算。
/// - 无附件（无金币无物品）→ 恒 true（无需邮局取回）
/// - 有金币有物品 → MailAutoSendGold && MailAutoSendItems
/// - 仅物品 → MailAutoSendItems
/// - 仅金币 → MailAutoSendGold
pub fn initial_collected(
    has_gold: bool,
    has_items: bool,
    auto_send_gold: bool,
    auto_send_items: bool,
) -> bool {
    if !has_gold && !has_items {
        return true;
    }
    if has_gold && has_items {
        return auto_send_gold && auto_send_items;
    }
    if has_items {
        return auto_send_items;
    }
    auto_send_gold
}

/// 全局邮件 ID 计数器（#73：服务器重启从 1 开始会与 DB 已有 mail_id 冲突 → UNIQUE 约束失败）
static NEXT_MAIL_ID: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(1);

/// 启动时把计数器初始化到 DB 最大 mail_id+1（避免重启后新邮件 id 冲突）
pub fn init_mail_id(max_id: u64) {
    NEXT_MAIL_ID.store(max_id.max(1), std::sync::atomic::Ordering::Relaxed);
}

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
            locked: false,
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

        // 未从邮局取回（collected=false）→ 邮箱不能领取
        assert!(mailbox.collect_attachment(1).is_none());

        // 邮局取回（collected=true）
        assert_eq!(mailbox.release_parcels(), 1);
        assert!(mailbox.get_mail(1).unwrap().collected);

        // 邮箱领取
        let (gold, items) = mailbox.collect_attachment(1).unwrap();
        assert_eq!(gold, 100);
        assert!(items.is_empty());
        assert_eq!(mailbox.get_mail(1).unwrap().gold, 0);
    }

    #[test]
    fn test_release_parcels_only_releases_uncollected_parcels() {
        let mut mailbox = Mailbox::new();
        let mut a = make_mail(); // collected=false, gold=100
        let mut b = make_mail();
        b.mail_id = 2;
        b.collected = true; // 已取回
        let mut c = make_mail();
        c.mail_id = 3;
        c.gold = 0; // 无附件（纯消息）
        mailbox.add_mail(a);
        mailbox.add_mail(b);
        mailbox.add_mail(c);

        assert_eq!(mailbox.release_parcels(), 1);
        assert!(mailbox.get_mail(1).unwrap().collected);
        assert!(mailbox.get_mail(2).unwrap().collected);
        assert!(!mailbox.get_mail(3).unwrap().collected);
    }

    #[test]
    fn test_initial_collected_matches_csharp_send() {
        // 无附件恒 true
        assert!(initial_collected(false, false, false, false));
        // 仅金币 → auto_send_gold
        assert!(!initial_collected(true, false, false, false));
        assert!(initial_collected(true, false, true, false));
        // 仅物品 → auto_send_items
        assert!(!initial_collected(false, true, false, false));
        assert!(initial_collected(false, true, false, true));
        // 金币+物品 → 两者皆真
        assert!(!initial_collected(true, true, true, false));
        assert!(!initial_collected(true, true, false, true));
        assert!(initial_collected(true, true, true, true));
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

    /// #2382：超容量清理已读+已收取+无附件的旧邮件
    #[test]
    fn test_trim_to_capacity_removes_collected_old_mail() {
        let mut mailbox = Mailbox::new();
        let mk = |id: u64| MailMessage {
            mail_id: id,
            sender_name: "s".into(),
            receiver_name: "r".into(),
            subject: String::new(),
            body: String::new(),
            timestamp: 0,
            read: true,
            collected: true,
            locked: false,
            gold: 0,
            items: vec![],
        };
        for id in 1..=5 {
            mailbox.add_mail(mk(id));
        }
        let removed = mailbox.trim_to_capacity(3);
        assert_eq!(removed, 2);
        assert_eq!(mailbox.inbox.len(), 3);
    }

    /// #2382：未读/未收取/带附件的邮件不被清理
    #[test]
    fn test_trim_keeps_unread_or_attachment_mail() {
        let mut mailbox = Mailbox::new();
        let mk = |id: u64, read: bool, collected: bool, gold: u64| MailMessage {
            mail_id: id,
            sender_name: "s".into(),
            receiver_name: "r".into(),
            subject: String::new(),
            body: String::new(),
            timestamp: 0,
            read,
            collected,
            locked: false,
            gold,
            items: vec![],
        };
        mailbox.add_mail(mk(1, false, true, 0)); // 未读
        mailbox.add_mail(mk(2, true, false, 0)); // 未收取
        mailbox.add_mail(mk(3, true, true, 50)); // 带金币
        let removed = mailbox.trim_to_capacity(1);
        assert_eq!(removed, 0);
        assert_eq!(mailbox.inbox.len(), 3);
    }
}
