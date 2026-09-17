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

impl MailMessage {
    /// 是否还有未收取的附件（金币或物品）——删除前必须检查，防丢件
    /// （C# 客户端 MailDialogs.cs:239-248 删除带附件邮件需玩家确认；服务端以此兜底）
    pub fn has_uncollected_parcel(&self) -> bool {
        self.gold > 0 || !self.items.is_empty()
    }
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
    /// 拒绝删除：带未收取附件（防丢件）或已锁定（C# 客户端 DeleteButton：Locked 直接 return）
    pub fn delete_mail(&mut self, mail_id: u64) -> bool {
        if let Some(idx) = self.inbox.iter().position(|m| m.mail_id == mail_id) {
            let mail = &self.inbox[idx];
            if mail.has_uncollected_parcel() || mail.locked {
                return false;
            }
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
        // 收取附件后自动解锁：锁定是为防误删带附件邮件，附件既已取出，锁定即失去保护对象；
        // 迁移库中 locked=1 的带附件邮件因此可在收取后删除，不再永久占格。
        // （取舍：无附件的纯消息锁定邮件仍需玩家用客户端 LockButton 自行解锁，与 C# 一致）
        mail.locked = false;
        Some((gold, items))
    }

    /// 收取失败回写：把未能入包的金币/物品写回邮件附件（防丢件/防金币蒸发）。
    /// 配合收取方「入包/入金失败回滚」使用；邮件已被并发删除时返回 false（调用方告警）。
    pub fn restore_attachment(&mut self, mail_id: u64, gold: u64, items: Vec<UserItem>) -> bool {
        let Some(mail) = self.get_mail_mut(mail_id) else {
            return false;
        };
        mail.gold = mail.gold.saturating_add(gold);
        mail.items.extend(items);
        true
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

    fn make_item(uid: u64) -> UserItem {
        UserItem {
            unique_id: uid,
            item_index: 100,
            ..Default::default()
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
        let mut m1 = make_mail();
        m1.gold = 0; // 无附件才可删除
        let mut m2 = make_mail();
        m2.mail_id = 2;
        m2.gold = 0;
        mailbox.add_mail(m1);
        mailbox.add_mail(m2);
        assert_eq!(mailbox.inbox.len(), 2);

        assert!(mailbox.delete_mail(1));
        assert_eq!(mailbox.inbox.len(), 1);
        assert!(!mailbox.delete_mail(999));
    }

    /// 严重15-2：带未收取附件（金币/物品）的邮件拒绝删除，附件不丢
    #[test]
    fn test_delete_mail_refuses_uncollected_parcel() {
        let mut mailbox = Mailbox::new();
        let gold_mail = make_mail(); // gold=100 未收取
        mailbox.add_mail(gold_mail);

        assert!(!mailbox.delete_mail(1));
        assert_eq!(mailbox.inbox.len(), 1);
        assert_eq!(mailbox.get_mail(1).unwrap().gold, 100);

        // 收取附件后即可删除
        assert_eq!(mailbox.release_parcels(), 1);
        let _ = mailbox.collect_attachment(1).unwrap();
        assert!(mailbox.delete_mail(1));
        assert!(mailbox.inbox.is_empty());
    }

    /// 严重15-2：已锁定邮件拒绝删除（C# 客户端 Locked 直接 return）
    #[test]
    fn test_delete_mail_refuses_locked() {
        let mut mailbox = Mailbox::new();
        let mut m = make_mail();
        m.gold = 0;
        m.locked = true;
        mailbox.add_mail(m);

        assert!(!mailbox.delete_mail(1));
        assert_eq!(mailbox.inbox.len(), 1);
    }

    /// 收取并发安全①（回归）：入包/入金失败后 restore_attachment 把金币与物品原样写回邮件，
    /// 附件不丢、邮件回到「有未收附件」状态（删除仍被拒）。
    #[test]
    fn test_restore_attachment_writes_back_after_failed_collect() {
        let mut mailbox = Mailbox::new();
        let mut m = make_mail(); // gold=100
        m.items = vec![make_item(7001), make_item(7002)];
        mailbox.add_mail(m);
        assert_eq!(mailbox.release_parcels(), 1);

        // 收取方已 mem::take 清空附件（此时背包并发填满 → 入包失败）
        let (gold, items) = mailbox.collect_attachment(1).unwrap();
        assert_eq!(gold, 100);
        assert_eq!(items.len(), 2);
        assert!(!mailbox.get_mail(1).unwrap().has_uncollected_parcel());

        // 入包/入金失败 → 写回邮件
        assert!(mailbox.restore_attachment(1, gold, items));
        let mail = mailbox.get_mail(1).unwrap();
        assert_eq!(mail.gold, 100);
        assert_eq!(mail.items.len(), 2);
        assert_eq!(mail.items[0].unique_id, 7001);
        assert!(mail.has_uncollected_parcel());
        assert!(!mailbox.delete_mail(1), "写回后带附件仍拒绝删除");

        // 邮件已被并发删除 → 写回失败（调用方告警），不 panic
        assert!(!mailbox.restore_attachment(999, 1, vec![make_item(1)]));
    }

    /// 收取并发安全③（回归）：收取附件后自动解锁——迁移库 locked=1 的带附件邮件
    /// 收取后即可删除，不再永久占格。
    #[test]
    fn test_collect_attachment_auto_unlocks_migrated_locked_mail() {
        let mut mailbox = Mailbox::new();
        let mut m = make_mail(); // gold=100 未收取
        m.locked = true; // 迁移库带入 locked=1
        mailbox.add_mail(m);
        assert_eq!(mailbox.release_parcels(), 1);

        let (gold, _) = mailbox.collect_attachment(1).unwrap();
        assert_eq!(gold, 100);
        assert!(!mailbox.get_mail(1).unwrap().locked, "收取后应自动解锁");
        assert!(mailbox.delete_mail(1), "无附件且已解锁 → 可删除");
    }

    /// has_uncollected_parcel：金币或物品任一未收取即为真
    #[test]
    fn test_has_uncollected_parcel() {
        let mut m = make_mail();
        assert!(m.has_uncollected_parcel()); // gold=100
        m.gold = 0;
        assert!(!m.has_uncollected_parcel());
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
