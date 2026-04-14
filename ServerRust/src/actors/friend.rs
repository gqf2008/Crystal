// Friend system - 好友数据结构
// 纯数据结构，由 WorldActor 调用

/// 好友条目
#[derive(Debug, Clone)]
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
pub struct BlockedEntry {
    pub object_id: u32,
    pub name: String,
}

/// 玩家好友列表
#[derive(Debug, Clone, Default)]
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

    /// 是否已拉黑
    pub fn is_blocked(&self, object_id: u32) -> bool {
        self.blocked.iter().any(|b| b.object_id == object_id)
    }
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
}
