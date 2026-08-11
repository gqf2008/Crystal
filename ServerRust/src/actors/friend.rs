// Friend system - 好友数据结构
// 纯数据结构，由 WorldActor 调用

/// 好友条目
#[derive(Debug, Clone)]
#[derive(serde::Serialize, serde::Deserialize)]
pub struct FriendEntry {
    /// 角色 ID（object_id）
    pub object_id: u32,
    /// 好友名称
    pub name: String,
    /// 备注
    pub memo: String,
}

/// 黑名单条目
#[derive(Debug, Clone)]
#[derive(serde::Serialize, serde::Deserialize)]
pub struct BlockedEntry {
    pub object_id: u32,
    pub name: String,
}

/// 玩家好友列表
#[derive(Debug, Clone, Default)]
#[derive(serde::Serialize, serde::Deserialize)]
pub struct FriendList {
    pub friends: Vec<FriendEntry>,
    pub blocked: Vec<BlockedEntry>,
}

impl FriendList {
    pub fn new() -> Self {
        Self::default()
    }

    /// 添加好友
    pub fn add_friend(&mut self, object_id: u32, name: String) {
        if !self.friends.iter().any(|f| f.object_id == object_id) {
            self.friends.push(FriendEntry { object_id, name, memo: String::new() });
        }
    }

    /// 移除好友（按 object_id）
    pub fn remove_friend(&mut self, object_id: u32) -> bool {
        if let Some(idx) = self.friends.iter().position(|f| f.object_id == object_id) {
            self.friends.remove(idx);
            true
        } else {
            false
        }
    }

    /// 设置备注
    pub fn set_memo(&mut self, object_id: u32, memo: String) -> bool {
        if let Some(f) = self.friends.iter_mut().find(|f| f.object_id == object_id) {
            f.memo = memo;
            true
        } else {
            false
        }
    }

    /// 添加黑名单
    pub fn add_blocked(&mut self, object_id: u32, name: String) {
        if !self.blocked.iter().any(|b| b.object_id == object_id) {
            self.blocked.push(BlockedEntry { object_id, name });
        }
    }

    /// 移除黑名单
    pub fn remove_blocked(&mut self, object_id: u32) -> bool {
        if let Some(idx) = self.blocked.iter().position(|b| b.object_id == object_id) {
            self.blocked.remove(idx);
            true
        } else {
            false
        }
    }

    /// 是否已好友
    pub fn is_friend(&self, object_id: u32) -> bool {
        self.friends.iter().any(|f| f.object_id == object_id)
    }

    /// 是否已好友（按名称，忽略大小写；离线添加的条目 object_id 为名字哈希，需按名查重）
    pub fn is_friend_name(&self, name: &str) -> bool {
        let n = name.to_lowercase();
        self.friends.iter().any(|f| f.name.to_lowercase() == n)
    }

    /// 是否已拉黑
    pub fn is_blocked(&self, object_id: u32) -> bool {
        self.blocked.iter().any(|b| b.object_id == object_id)
    }

    /// 是否已拉黑（按名称，忽略大小写；离线收件人 object_id 不可得时用）
    pub fn is_blocked_name(&self, name: &str) -> bool {
        let n = name.to_lowercase();
        self.blocked.iter().any(|b| b.name.to_lowercase() == n)
    }
}

/// 离线添加好友/黑名单的稳定 object_id（近似 C# CharacterInfo.Index：
/// 客户端用它做唯一标识/移除/备注；离线时运行时 object_id 不可得，用名字 FNV-1a 哈希，
/// 上线后由 SocialActor 校正为运行时 ID）
pub fn friend_id_from_name(name: &str) -> u32 {
    let mut hash: u32 = 0x811c9dc5;
    for b in name.to_lowercase().bytes() {
        hash ^= b as u32;
        hash = hash.wrapping_mul(0x0100_0193);
    }
    if hash == 0 { 1 } else { hash }
}

/// 好友在线判定：object_id 命中在线列表，或名字忽略大小写命中在线名字列表
/// （离线添加的好友 object_id 为名字哈希，上线后尚未校正时也能正确显示在线）
pub fn friend_is_online(
    object_id: u32,
    name: &str,
    online_object_ids: &[u32],
    online_names: &[String],
) -> bool {
    online_object_ids.contains(&object_id)
        || online_names.iter().any(|n| n.eq_ignore_ascii_case(name))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_add_and_remove_friend() {
        let mut list = FriendList::new();
        list.add_friend(1001, "Alice".into());
        list.add_friend(1002, "Bob".into());
        assert_eq!(list.friends.len(), 2);

        assert!(list.remove_friend(1001));
        assert_eq!(list.friends.len(), 1);
        assert!(!list.remove_friend(9999));
    }

    #[test]
    fn test_duplicate_friend_prevention() {
        let mut list = FriendList::new();
        list.add_friend(1001, "Alice".into());
        list.add_friend(1001, "Alice".into()); // duplicate
        assert_eq!(list.friends.len(), 1);
    }

    #[test]
    fn test_set_memo() {
        let mut list = FriendList::new();
        list.add_friend(1001, "Alice".into());
        assert!(list.set_memo(1001, "Best friend".into()));
        assert_eq!(list.friends[0].memo, "Best friend");
        assert!(!list.set_memo(9999, "No one".into()));
    }

    #[test]
    fn test_blocked_list() {
        let mut list = FriendList::new();
        list.add_blocked(2001, "Enemy".into());
        assert!(list.is_blocked(2001));
        assert!(list.remove_blocked(2001));
        assert!(!list.is_blocked(2001));
    }

    #[test]
    fn test_blocked_by_name_case_insensitive() {
        let mut list = FriendList::new();
        list.add_blocked(2001, "Enemy".into());
        assert!(list.is_blocked_name("Enemy"));
        assert!(list.is_blocked_name("enemy"));
        assert!(list.is_blocked_name("ENEMY"));
        assert!(!list.is_blocked_name("Friend"));
    }

    #[test]
    fn test_friend_id_from_name_stable_and_case_insensitive() {
        // 大小写不同 → 同一稳定 id；非零
        assert_eq!(super::friend_id_from_name("Alice"), super::friend_id_from_name("alice"));
        assert_eq!(super::friend_id_from_name("Alice"), super::friend_id_from_name("ALICE"));
        assert_ne!(super::friend_id_from_name("Alice"), 0);
        // 不同名字大概率不同
        assert_ne!(super::friend_id_from_name("Alice"), super::friend_id_from_name("Bob"));
    }

    #[test]
    fn test_is_friend_name_case_insensitive() {
        let mut list = FriendList::new();
        list.add_friend(1001, "Alice".into());
        assert!(list.is_friend_name("Alice"));
        assert!(list.is_friend_name("alice"));
        assert!(list.is_friend_name("ALICE"));
        assert!(!list.is_friend_name("Bob"));
    }

    #[test]
    fn test_friend_is_online_by_id_or_name() {
        let ids = vec![1001u32, 1002];
        let names = vec!["Online".to_string()];
        assert!(super::friend_is_online(1001, "X", &ids, &names));
        assert!(super::friend_is_online(0, "ONLINE", &ids, &names));
        assert!(!super::friend_is_online(0, "Offline", &ids, &names));
        assert!(!super::friend_is_online(9999, "X", &ids, &names));
    }
}
