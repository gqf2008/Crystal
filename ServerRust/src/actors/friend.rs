// Friend system - 好友数据结构
// 纯数据结构，由 WorldActor 调用

/// 好友条目
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct FriendEntry {
    /// 角色 ID（object_id）
    pub object_id: u32,
    /// 好友名称
    pub name: String,
    /// 备注
    pub memo: String,
}

/// 黑名单条目
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct BlockedEntry {
    pub object_id: u32,
    pub name: String,
}

/// 玩家好友列表
#[derive(Debug, Clone, Default, serde::Serialize, serde::Deserialize)]
pub struct FriendList {
    pub friends: Vec<FriendEntry>,
    pub blocked: Vec<BlockedEntry>,
}

impl FriendList {
    pub fn new() -> Self {
        Self::default()
    }

    /// 添加好友。
    ///
    /// 身份 = 角色名（忽略大小写）；C# 基准 `Server/MirDatabase/CharacterInfo.cs:712-743`
    /// `FriendInfo.Index` 是**持久角色 Index**（重启/重登不变），运行时 ObjectID 从不作身份键。
    /// 本端 `object_id` 是会话内运行时 id（每次登录都变），若按它去重就会同名累积——
    /// 参见 #2879：测试库 `friends` 表同名 6 条不同 id；上线校正把多条同名改成同一 id 后，
    /// 写库撞 `PRIMARY KEY(character_name, friend_object_id)`，令**整个角色存档事务回滚**。
    pub fn add_friend(&mut self, object_id: u32, name: String) {
        match self
            .friends
            .iter()
            .position(|f| f.name.eq_ignore_ascii_case(&name))
        {
            Some(idx) => {
                // 已有该好友（含离线添加的占位条目）→ 只刷新运行时 id，不新增条目
                self.friends[idx].object_id = object_id;
                self.friends[idx].name = name;
            }
            None => self.friends.push(FriendEntry {
                object_id,
                name,
                memo: String::new(),
            }),
        }
        self.normalize();
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

    /// 添加黑名单（同 [`Self::add_friend`]：身份 = 名字，运行时 id 只作缓存）
    pub fn add_blocked(&mut self, object_id: u32, name: String) {
        match self
            .blocked
            .iter()
            .position(|b| b.name.eq_ignore_ascii_case(&name))
        {
            Some(idx) => {
                self.blocked[idx].object_id = object_id;
                self.blocked[idx].name = name;
            }
            None => self.blocked.push(BlockedEntry { object_id, name }),
        }
        self.normalize();
    }

    /// 归一化：按名字（忽略大小写）去重，再兜底按运行时 id 去重。返回移除条目数。
    ///
    /// 用途：① 旧库/旧内存里已有同名多条（#2879 遗留）时自愈；② 上线校正把同名条目
    /// 统一刷成同一运行时 id 后收敛为一条；③ 写库前兜底，保证存档不会因好友主键冲突回滚。
    pub fn normalize(&mut self) -> usize {
        let before = self.friends.len() + self.blocked.len();

        let mut friends: Vec<FriendEntry> = Vec::with_capacity(self.friends.len());
        for f in self.friends.drain(..) {
            match friends
                .iter_mut()
                .find(|p| p.name.eq_ignore_ascii_case(&f.name))
            {
                Some(prev) => {
                    // 保留最后一条的运行时 id（最新会话），备注沿用非空者
                    if prev.memo.is_empty() {
                        prev.memo = f.memo;
                    }
                    prev.object_id = f.object_id;
                    if prev.name != f.name {
                        prev.name = f.name;
                    }
                }
                None => friends.push(f),
            }
        }
        let mut deduped: Vec<FriendEntry> = Vec::with_capacity(friends.len());
        for f in friends {
            if !deduped.iter().any(|p| p.object_id == f.object_id) {
                deduped.push(f);
            }
        }
        self.friends = deduped;

        let mut blocked: Vec<BlockedEntry> = Vec::with_capacity(self.blocked.len());
        for b in self.blocked.drain(..) {
            match blocked
                .iter_mut()
                .find(|p| p.name.eq_ignore_ascii_case(&b.name))
            {
                Some(prev) => {
                    prev.object_id = b.object_id;
                    if prev.name != b.name {
                        prev.name = b.name;
                    }
                }
                None => blocked.push(b),
            }
        }
        let mut deduped_blocked: Vec<BlockedEntry> = Vec::with_capacity(blocked.len());
        for b in blocked {
            if !deduped_blocked.iter().any(|p| p.object_id == b.object_id) {
                deduped_blocked.push(b);
            }
        }
        self.blocked = deduped_blocked;

        before - (self.friends.len() + self.blocked.len())
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
    if hash == 0 {
        1
    } else {
        hash
    }
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

    /// #2879：同名好友换运行时 id（重登/对方重新上线）应原地刷新，而不是新增条目。
    /// 反向添加链路（social.rs `AddFriendRequest` 在线双向添加）每次上线都会走这里。
    #[test]
    fn test_add_friend_same_name_new_runtime_id_updates_in_place() {
        let mut list = FriendList::new();
        list.add_friend(1000, "bevychar".into());
        list.add_friend(32296, "BEVYCHAR".into()); // 大小写不同 + 新运行时 id
        assert_eq!(
            list.friends.len(),
            1,
            "同名（忽略大小写）应按名字去重，不能按运行时 id 累积"
        );
        assert_eq!(list.friends[0].object_id, 32296, "应刷新为最新运行时 id");
    }

    /// #2879 数据现场复现：测试库里同名 6 条不同运行时 id（旧版遗留）→ 归一化后 1 条。
    #[test]
    fn test_normalize_collapses_legacy_duplicate_friend_rows() {
        let mut list = FriendList::new();
        for (oid, memo) in [
            (1000u32, ""),
            (1402, "老备注"),
            (18604, ""),
            (20560, ""),
            (30340, ""),
            (32296, ""),
        ] {
            list.friends.push(FriendEntry {
                object_id: oid,
                name: "bevychar".into(),
                memo: memo.to_string(),
            });
        }
        assert_eq!(list.normalize(), 5);
        assert_eq!(list.friends.len(), 1);
        assert_eq!(list.friends[0].object_id, 32296);
        assert_eq!(list.friends[0].memo, "老备注", "非空备注应保留");
        assert_eq!(list.normalize(), 0, "二次归一应为幂等");
    }

    /// 兜底：不同名但运行时 id 撞车（旧库可能存过运行时 id）同样会撞写库主键，须去重。
    #[test]
    fn test_normalize_dedupes_object_id_across_names() {
        let mut list = FriendList::new();
        list.friends.push(FriendEntry {
            object_id: 7,
            name: "A".into(),
            memo: String::new(),
        });
        list.friends.push(FriendEntry {
            object_id: 7,
            name: "B".into(),
            memo: String::new(),
        });
        assert_eq!(list.normalize(), 1);
        assert_eq!(list.friends.len(), 1);
    }

    /// 黑名单同一处理（add_blocked 同源缺陷）。
    #[test]
    fn test_add_blocked_same_name_new_runtime_id_updates_in_place() {
        let mut list = FriendList::new();
        list.add_blocked(1000, "Enemy".into());
        list.add_blocked(2000, "enemy".into());
        assert_eq!(list.blocked.len(), 1);
        assert_eq!(list.blocked[0].object_id, 2000);
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
        assert_eq!(
            super::friend_id_from_name("Alice"),
            super::friend_id_from_name("alice")
        );
        assert_eq!(
            super::friend_id_from_name("Alice"),
            super::friend_id_from_name("ALICE")
        );
        assert_ne!(super::friend_id_from_name("Alice"), 0);
        // 不同名字大概率不同
        assert_ne!(
            super::friend_id_from_name("Alice"),
            super::friend_id_from_name("Bob")
        );
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
