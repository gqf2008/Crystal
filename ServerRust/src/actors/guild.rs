// Guild system - 行会数据结构
// 纯数据结构，由 WorldActor 调用

/// 行会成员 rank
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum GuildRank {
    Leader = 0,
    Officer = 1,
    Member = 2,
}

impl GuildRank {
    pub fn from_u8(val: u8) -> Self {
        match val {
            0 => Self::Leader,
            1 => Self::Officer,
            _ => Self::Member,
        }
    }

    /// 默认行会权限（C# GuildRankOptions 位标志；简化：Leader 全权限，Officer 部分，Member 无）
    pub fn default_options(&self) -> u8 {
        const CAN_CHANGE_RANK: u8 = 1;
        const CAN_RECRUIT: u8 = 2;
        const CAN_KICK: u8 = 4;
        const CAN_STORE_ITEM: u8 = 8;
        const CAN_RETRIEVE_ITEM: u8 = 16;
        const CAN_ALTER_ALLIANCE: u8 = 32;
        const CAN_CHANGE_NOTICE: u8 = 64;
        const CAN_ACTIVATE_BUFF: u8 = 128;
        match self {
            Self::Leader => CAN_CHANGE_RANK | CAN_RECRUIT | CAN_KICK | CAN_STORE_ITEM
                | CAN_RETRIEVE_ITEM | CAN_ALTER_ALLIANCE | CAN_CHANGE_NOTICE | CAN_ACTIVATE_BUFF,
            Self::Officer => CAN_RECRUIT | CAN_KICK | CAN_STORE_ITEM | CAN_RETRIEVE_ITEM | CAN_CHANGE_NOTICE,
            Self::Member => 0,
        }
    }
}

/// 行会成员
#[derive(Debug, Clone)]
pub struct GuildMember {
    /// 玩家名称
    pub name: String,
    /// Session ID（None = 离线）
    pub session_id: Option<u64>,
    /// 成员 rank
    pub rank: GuildRank,
}

/// 行会
#[derive(Debug, Clone)]
pub struct Guild {
    /// 行会名称（唯一标识）
    pub name: String,
    /// 公告（多行）
    pub notice: Vec<String>,
    /// 成员列表
    pub members: Vec<GuildMember>,
    /// 行会金币（仓库）
    pub gold: u64,
    /// 行会仓库物品（最多 100 格）
    pub storage_items: Vec<Option<(mir2_shared::data::item::UserItem, u32)>>,
    /// 已激活的行会 Buff id 列表（C# GuildObject.BuffList）
    pub buffs: Vec<u32>,
}

impl Guild {
    pub fn new(name: String, leader_name: String, leader_session: u64) -> Self {
        Self {
            name,
            notice: vec![
                "Welcome to our guild!".to_string(),
                "".to_string(),
                "".to_string(),
                "".to_string(),
                "".to_string(),
            ],
            members: vec![GuildMember {
                name: leader_name,
                session_id: Some(leader_session),
                rank: GuildRank::Leader,
            }],
            gold: 0,
            storage_items: vec![None; 100],
            buffs: Vec::new(),
        }
    }

    /// 成员数量
    pub fn member_count(&self) -> usize {
        self.members.len()
    }

    /// 是否已有该成员
    pub fn has_member(&self, name: &str) -> bool {
        self.members.iter().any(|m| m.name == name)
    }

    /// 添加成员
    pub fn add_member(&mut self, name: String, session_id: Option<u64>) {
        if !self.has_member(&name) {
            self.members.push(GuildMember {
                name,
                session_id,
                rank: GuildRank::Member,
            });
        }
    }

    /// 移除成员
    pub fn remove_member(&mut self, name: &str) -> bool {
        // 不能踢会长
        if let Some(idx) = self.members.iter().position(|m| m.name == name) {
            if self.members[idx].rank == GuildRank::Leader {
                return false;
            }
            self.members.remove(idx);
            true
        } else {
            false
        }
    }

    /// 设置成员 rank
    pub fn set_rank(&mut self, name: &str, rank: GuildRank) -> bool {
        if let Some(m) = self.members.iter_mut().find(|m| m.name == name) {
            m.rank = rank;
            true
        } else {
            false
        }
    }

    /// 设置成员在线状态
    pub fn set_online(&mut self, name: &str, session_id: u64) -> bool {
        if let Some(m) = self.members.iter_mut().find(|m| m.name == name) {
            m.session_id = Some(session_id);
            true
        } else {
            false
        }
    }

    /// 设置成员离线
    pub fn set_offline(&mut self, name: &str) {
        if let Some(m) = self.members.iter_mut().find(|m| m.name == name) {
            m.session_id = None;
        }
    }

    /// 获取会长名称
    pub fn leader_name(&self) -> &str {
        self.members.iter()
            .find(|m| m.rank == GuildRank::Leader)
            .map(|m| m.name.as_str())
            .unwrap_or("Unknown")
    }

    /// 在线成员 session 列表
    pub fn online_sessions(&self, exclude: u64) -> Vec<u64> {
        self.members.iter()
            .filter_map(|m| m.session_id.filter(|s| *s != exclude))
            .collect()
    }

    /// 存入物品到行会仓库
    /// 返回 (物品, 仓库格子) 或 None
    pub fn deposit_item(&mut self, item: mir2_shared::data::item::UserItem, quantity: u32) -> Option<usize> {
        let slot = self.storage_items.iter_mut().position(|s| s.is_none())?;
        self.storage_items[slot] = Some((item, quantity));
        Some(slot)
    }

    /// 从行会仓库取出物品
    /// 返回 Some((物品, 数量, 格子)) 或 None
    pub fn withdraw_item(&mut self, storage_grid: u8) -> Option<(mir2_shared::data::item::UserItem, u32, u8)> {
        let idx = storage_grid as usize;
        if idx >= self.storage_items.len() {
            return None;
        }
        self.storage_items[idx].take().map(|(item, qty)| (item, qty, storage_grid))
    }

    /// 仓库是否有空位
    pub fn storage_has_space(&self) -> bool {
        self.storage_items.iter().any(|s| s.is_none())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_guild() -> Guild {
        Guild::new("DragonSlayers".into(), "Alice".into(), 1)
    }

    #[test]
    fn test_create_guild() {
        let g = make_guild();
        assert_eq!(g.name, "DragonSlayers");
        assert_eq!(g.member_count(), 1);
        assert_eq!(g.leader_name(), "Alice");
    }

    #[test]
    fn test_add_and_remove_member() {
        let mut g = make_guild();
        g.add_member("Bob".into(), Some(2));
        assert_eq!(g.member_count(), 2);
        assert!(g.has_member("Bob"));

        assert!(g.remove_member("Bob"));
        assert_eq!(g.member_count(), 1);
        assert!(!g.has_member("Bob"));
    }

    #[test]
    fn test_cannot_kick_leader() {
        let mut g = make_guild();
        assert!(!g.remove_member("Alice"));
        assert_eq!(g.member_count(), 1);
    }

    #[test]
    fn test_set_rank() {
        let mut g = make_guild();
        g.add_member("Bob".into(), Some(2));
        assert!(g.set_rank("Bob", GuildRank::Officer));
        assert_eq!(g.members[1].rank, GuildRank::Officer);
        assert!(!g.set_rank("Nobody", GuildRank::Officer));
    }

    #[test]
    fn test_online_status() {
        let mut g = make_guild();
        g.add_member("Bob".into(), Some(2));
        g.set_offline("Bob");
        assert!(g.members[1].session_id.is_none());

        g.set_online("Bob", 3);
        assert_eq!(g.members[1].session_id, Some(3));
    }

    #[test]
    fn test_online_sessions() {
        let mut g = make_guild();
        g.add_member("Bob".into(), Some(2));
        g.add_member("Carol".into(), None); // offline
        let sessions = g.online_sessions(1);
        assert_eq!(sessions, vec![2]);
    }

    #[test]
    fn test_duplicate_member_prevention() {
        let mut g = make_guild();
        g.add_member("Bob".into(), Some(2));
        g.add_member("Bob".into(), Some(3));
        assert_eq!(g.member_count(), 2);
    }
}
