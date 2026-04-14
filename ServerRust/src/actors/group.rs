// Group/Party system - 组队数据结构
// 纯数据结构，由 WorldActor 调用

/// 组队最大成员数
pub const MAX_GROUP_SIZE: usize = 5;

/// 组队模式（掉落分配）
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum GroupMode {
    /// 各自拾取
    All,
    /// 队长分配
    Leader,
    /// 按队伍分配
    Hunter,
}

impl Default for GroupMode {
    fn default() -> Self {
        Self::All
    }
}

/// 组队成员
#[derive(Debug, Clone)]
pub struct GroupMember {
    /// Session ID
    pub session_id: u64,
    /// 玩家名称
    pub name: String,
    /// 是否队长
    pub is_leader: bool,
    /// 是否在线
    pub online: bool,
}

/// 组队
#[derive(Debug, Clone)]
pub struct Group {
    /// 组队唯一 ID
    pub id: u64,
    /// 成员列表
    pub members: Vec<GroupMember>,
    /// 掉落模式
    pub mode: GroupMode,
}

impl Group {
    pub fn new(id: u64, leader: GroupMember) -> Self {
        Self {
            id,
            members: vec![leader],
            mode: GroupMode::default(),
        }
    }

    /// 获取队长 session ID
    pub fn leader_session(&self) -> Option<u64> {
        self.members
            .iter()
            .find(|m| m.is_leader && m.online)
            .map(|m| m.session_id)
    }

    /// 检查玩家是否在组队中
    pub fn has_member(&self, session_id: u64) -> bool {
        self.members.iter().any(|m| m.session_id == session_id)
    }

    /// 添加成员（返回 false 如果已满或已存在）
    pub fn add_member(&mut self, member: GroupMember) -> bool {
        if self.has_member(member.session_id) || self.members.len() >= MAX_GROUP_SIZE {
            return false;
        }
        self.members.push(member);
        true
    }

    /// 移除成员
    pub fn remove_member(&mut self, session_id: u64) -> Option<GroupMember> {
        if let Some(idx) = self.members.iter().position(|m| m.session_id == session_id) {
            let member = self.members.remove(idx);
            // 如果踢出的是队长，转移给下一个在线成员（优先在线，否则第一个）
            if member.is_leader {
                if let Some(next) = self.members.iter_mut().find(|m| m.online) {
                    next.is_leader = true;
                } else if !self.members.is_empty() {
                    self.members[0].is_leader = true;
                }
            }
            Some(member)
        } else {
            None
        }
    }

    /// 更新成员在线状态
    pub fn set_online(&mut self, session_id: u64, online: bool) {
        if let Some(m) = self.members.iter_mut().find(|m| m.session_id == session_id) {
            m.online = online;
        }
    }

    /// 获取所有在线成员（排除指定 session）
    pub fn other_online_sessions(&self, exclude_session: u64) -> Vec<u64> {
        self.members
            .iter()
            .filter(|m| m.session_id != exclude_session && m.online)
            .map(|m| m.session_id)
            .collect()
    }

    /// 获取成员数量
    pub fn member_count(&self) -> usize {
        self.members.len()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_member(session_id: u64, is_leader: bool) -> GroupMember {
        GroupMember {
            session_id,
            name: format!("Player{}", session_id),
            is_leader,
            online: true,
        }
    }

    #[test]
    fn test_create_group_with_leader() {
        let leader = make_member(1, true);
        let group = Group::new(100, leader);
        assert_eq!(group.id, 100);
        assert_eq!(group.member_count(), 1);
        assert_eq!(group.leader_session(), Some(1));
    }

    #[test]
    fn test_add_and_remove_member() {
        let leader = make_member(1, true);
        let mut group = Group::new(100, leader);
        assert!(group.add_member(make_member(2, false)));
        assert!(group.add_member(make_member(3, false)));
        assert_eq!(group.member_count(), 3);

        let removed = group.remove_member(2);
        assert!(removed.is_some());
        assert_eq!(group.member_count(), 2);
        assert!(!group.has_member(2));
    }

    #[test]
    fn test_leader_transfer_on_kick() {
        let leader = make_member(1, true);
        let mut group = Group::new(100, leader);
        group.add_member(make_member(2, false));
        group.add_member(make_member(3, false));

        // 踢出队长，队长应该转移到下一个成员
        group.remove_member(1);
        assert!(group.members[0].is_leader);
        assert_eq!(group.members[0].session_id, 2);
        assert_eq!(group.leader_session(), Some(2));
    }

    #[test]
    fn test_online_status() {
        let leader = make_member(1, true);
        let mut group = Group::new(100, leader);
        group.add_member(make_member(2, false));

        group.set_online(2, false);
        let other = group.other_online_sessions(1);
        assert!(other.is_empty());

        group.set_online(2, true);
        let other = group.other_online_sessions(1);
        assert_eq!(other, vec![2]);
    }

    #[test]
    fn test_duplicate_member_prevention() {
        let leader = make_member(1, true);
        let mut group = Group::new(100, leader);
        assert!(group.add_member(make_member(2, false)));
        assert!(!group.add_member(make_member(2, false))); // duplicate
        assert_eq!(group.member_count(), 2);
    }

    #[test]
    fn test_max_group_size() {
        let leader = make_member(1, true);
        let mut group = Group::new(100, leader);
        for i in 2..=5 {
            assert!(group.add_member(make_member(i, false)));
        }
        assert_eq!(group.member_count(), 5);
        // 第 6 个应该失败
        assert!(!group.add_member(make_member(6, false)));
        assert_eq!(group.member_count(), 5);
    }

    #[test]
    fn test_leader_transfer_prefers_online() {
        let leader = make_member(1, true);
        let mut group = Group::new(100, leader);
        group.add_member(make_member(2, false));
        group.add_member(make_member(3, false));
        // 让成员 2 离线
        group.set_online(2, false);

        // 踢出队长，应该转移给在线的成员 3
        group.remove_member(1);
        assert!(group.members.iter().find(|m| m.session_id == 3).unwrap().is_leader);
    }
}
