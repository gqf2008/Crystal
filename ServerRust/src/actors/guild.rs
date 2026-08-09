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
    /// C# GuildRankOptions 位标志（GuildInfo.cs）：CanChangeRank=1/CanRecruit=2/CanKick=4/CanStoreItem=8/
    /// CanRetrieveItem=16/CanAlterAlliance=32/CanChangeNotice=64/CanActivateBuff=128
    pub const CAN_CHANGE_RANK: u8 = 1;
    pub const CAN_RECRUIT: u8 = 2;
    pub const CAN_KICK: u8 = 4;
    pub const CAN_STORE_ITEM: u8 = 8;
    pub const CAN_RETRIEVE_ITEM: u8 = 16;
    pub const CAN_ALTER_ALLIANCE: u8 = 32;
    pub const CAN_CHANGE_NOTICE: u8 = 64;
    pub const CAN_ACTIVATE_BUFF: u8 = 128;

    pub fn from_u8(val: u8) -> Self {
        match val {
            0 => Self::Leader,
            1 => Self::Officer,
            _ => Self::Member,
        }
    }

    /// 默认行会权限（C# GuildRankOptions：Leader 全权限，Officer 部分，Member 无）
    pub fn default_options(&self) -> u8 {
        match self {
            Self::Leader => Self::CAN_CHANGE_RANK | Self::CAN_RECRUIT | Self::CAN_KICK | Self::CAN_STORE_ITEM
                | Self::CAN_RETRIEVE_ITEM | Self::CAN_ALTER_ALLIANCE | Self::CAN_CHANGE_NOTICE | Self::CAN_ACTIVATE_BUFF,
            Self::Officer => Self::CAN_RECRUIT | Self::CAN_KICK | Self::CAN_STORE_ITEM | Self::CAN_RETRIEVE_ITEM | Self::CAN_CHANGE_NOTICE,
            Self::Member => 0,
        }
    }
}

/// 行会职务定义（C# GuildObject.Ranks；任意数量，name + options 权限位）
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct GuildRankDef {
    pub index: u8,
    pub name: String,
    /// C# GuildRankOptions 位标志
    pub options: u8,
}

/// 默认 3 档职务（0=会长 1=副会长 2=成员，权限取自 GuildRank::default_options）
pub fn default_rank_defs() -> Vec<GuildRankDef> {
    vec![
        GuildRankDef { index: 0, name: "会长".to_string(), options: GuildRank::Leader.default_options() },
        GuildRankDef { index: 1, name: "副会长".to_string(), options: GuildRank::Officer.default_options() },
        GuildRankDef { index: 2, name: "成员".to_string(), options: GuildRank::Member.default_options() },
    ]
}

/// 行会成员
#[derive(Debug, Clone)]
pub struct GuildMember {
    /// 玩家名称
    pub name: String,
    /// Session ID（None = 离线）
    pub session_id: Option<u64>,
    /// 成员 rank（逻辑档：Leader/Officer/Member，权限判定用）
    pub rank: GuildRank,
    /// 职务索引（rank_defs 下标，显示/改名/分组用；默认与档位一致）
    pub rank_index: u8,
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
    /// 职务定义列表（C# GuildObject.Ranks：任意数量，name + options）
    pub rank_defs: Vec<GuildRankDef>,
    /// 行会金币（仓库）
    pub gold: u64,
    /// 行会仓库物品（最多 100 格）
    pub storage_items: Vec<Option<(mir2_shared::data::item::UserItem, u32)>>,
    /// 已激活的行会 Buff id 列表（C# GuildObject.BuffList）
    pub buffs: Vec<u32>,
    /// 行会经验（C# GuildInfo.Experience）
    pub experience: i64,
    /// 行会等级（C# GuildInfo.Level）
    pub level: u8,
    /// 本级所需经验（C# GuildInfo.MaxExperience；0=不升级）
    pub max_experience: i64,
    /// 未分配点数（C# GuildInfo.SparePoints）
    pub spare_points: u8,
    /// 成员上限（C# GuildInfo.MemberCap）
    pub member_cap: i32,
    /// #1344：下次 GuildExpGain 广播时间（unix ms，运行时；C# NextExpUpdate=Envir.Time+10000）
    pub next_exp_update: i64,
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
            rank_defs: default_rank_defs(),
            members: vec![GuildMember {
                name: leader_name,
                session_id: Some(leader_session),
                rank: GuildRank::Leader,
                rank_index: 0,
            }],
            gold: 0,
            storage_items: vec![None; 100],
            buffs: Vec::new(),
            experience: 0,
            level: 1,
            max_experience: 0,
            spare_points: 0,
            member_cap: 50,
            next_exp_update: 0,
        }
    }

    /// C# GuildObject.GainExp（644-690）：行会经验积累 + 升级
    /// 返回是否升级；升级时 SparePoints += PointPerLevel，MaxExperience/MemberCap 查配置列表
    pub fn apply_gain_exp(
        &mut self,
        amount: i64,
        exp_rate: f64,
        point_per_level: u8,
        experience_list: &[i64],
        membercap_list: &[i32],
    ) -> bool {
        if self.max_experience <= 0 {
            return false;
        }
        let exp_amount = (amount as f64 * exp_rate) as u64;
        if exp_amount == 0 {
            return false;
        }
        self.experience += exp_amount as i64;
        let mut leveled = false;
        while self.experience > self.max_experience {
            leveled = true;
            self.level = (self.level as u16 + 1).min(255) as u8;
            self.spare_points = (self.spare_points as u16 + point_per_level as u16).min(255) as u8;
            self.experience -= self.max_experience;
            let li = self.level as usize;
            self.max_experience = if li < experience_list.len() { experience_list[li] } else { 0 };
            if self.max_experience == 0 || self.level == 255 {
                break;
            }
        }
        if leveled {
            let li = self.level as usize;
            if li < membercap_list.len() {
                self.member_cap = membercap_list[li];
            }
        }
        leveled
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
                rank_index: 2,
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
            m.rank_index = rank as u8;
            true
        } else {
            false
        }
    }

    /// #1395：职务显示名（rank_defs 下标，越界回退档位默认名）
    pub fn rank_name(&self, rank_index: u8) -> String {
        self.rank_defs
            .iter()
            .find(|d| d.index == rank_index)
            .map(|d| d.name.clone())
            .unwrap_or_else(|| "成员".to_string())
    }

    /// #1395：职务权限（rank_defs.options；越界回退档位默认）
    pub fn rank_options(&self, rank_index: u8) -> u8 {
        self.rank_defs
            .iter()
            .find(|d| d.index == rank_index)
            .map(|d| d.options)
            .unwrap_or(0)
    }

    /// #1461：成员权限位（按成员 rank_index 查 rank_defs.options；C# MyGuildRank.Options）
    pub fn member_options(&self, name: &str) -> u8 {
        self.members
            .iter()
            .find(|m| m.name == name)
            .map(|m| self.rank_options(m.rank_index))
            .unwrap_or(0)
    }

    /// #1395：添加职务（C# EditGuildMember ChangeType=4），返回新 index
    pub fn add_rank(&mut self, name: &str) -> u8 {
        let next = self.rank_defs.iter().map(|d| d.index).max().unwrap_or(0).saturating_add(1);
        self.rank_defs.push(GuildRankDef { index: next, name: name.to_string(), options: 0 });
        next
    }

    /// #1395：设置职务权限（C# EditGuildMember ChangeType=5 切位）
    pub fn set_rank_options(&mut self, rank_index: u8, options: u8) -> bool {
        if let Some(d) = self.rank_defs.iter_mut().find(|d| d.index == rank_index) {
            d.options = options;
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



    /// #1161：C# GuildObject.GainExp——经验按 ExpRate 积累、升级、点/上限列表
    #[test]
    fn test_apply_gain_exp_levels_up() {
        let mut g = Guild::new("G".into(), "Leader".into(), 1);
        g.max_experience = 1000;
        // C#：MaxExperience = Guild_ExperienceList[Level]（索引=等级；0 位占位）
        let exp_list = [0i64, 1000, 2000, 4000];
        let cap_list = [0i32, 50, 60, 70];
        // 10000 经验 × 0.01 = 100 → 不升级
        assert!(!g.apply_gain_exp(10000, 0.01, 1, &exp_list, &cap_list));
        assert_eq!(g.experience, 100);
        assert_eq!(g.level, 1);
        // 200000 经验 × 0.01 = 2000 → 连升（1000 满 → 剩 1000 = 2000 的 0 → 升到 2 级余 1000）
        // 100(已有)+2000=2100 > 1000 → 升 1 级：剩 1100，max=2000；不满足 >2000 → 停
        assert!(g.apply_gain_exp(200000, 0.01, 1, &exp_list, &cap_list));
        assert_eq!(g.level, 2);
        assert_eq!(g.experience, 1100);
        assert_eq!(g.spare_points, 1);
        assert_eq!(g.max_experience, 2000);
        assert_eq!(g.member_cap, 60);
        // max_experience=0 → 不再积累
        g.max_experience = 0;
        assert!(!g.apply_gain_exp(999999, 0.01, 1, &exp_list, &cap_list));
    }

    /// #1161：ExpRate=0 时经验为 0 → 不处理
    #[test]
    fn test_apply_gain_exp_zero_rate() {
        let mut g = Guild::new("G".into(), "L".into(), 1);
        g.max_experience = 1000;
        assert!(!g.apply_gain_exp(10000, 0.0, 0, &[], &[]));
        assert_eq!(g.experience, 0);
    }

    #[test]
    fn member_options_use_rank_defs_bits() {
        // #1461：C# MyGuildRank.Options——按 rank_index 查 rank_defs.options
        let mut g = Guild::new("G".into(), "L".into(), 1);
        g.add_member("A".into(), None);
        // 成员（index=2）默认无权限
        assert_eq!(g.member_options("A"), 0);
        // 授予 CanRecruit 后（模拟自定义职务/权限调整）
        if let Some(m) = g.members.iter_mut().find(|m| m.name == "A") {
            m.rank_index = 2;
        }
        g.set_rank_options(2, crate::actors::guild::GuildRank::CAN_RECRUIT);
        assert_eq!(g.member_options("A"), crate::actors::guild::GuildRank::CAN_RECRUIT);
        // 未找到成员 → 0
        assert_eq!(g.member_options("Nobody"), 0);
    }
}
