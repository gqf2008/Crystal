// ============================================================================
// 全局游戏资源 (Resources) - 不是组件，而是全局唯一的数据
// ============================================================================
//
// 这些资源存储在 hecs::World 之外，作为独立的全局状态管理
// 使用 Arc<RwLock<T>> 包装以支持多线程访问

use mir2_shared::{ClientQuestProgress, ClientFriend, GuildMember};

/// 当前地图信息
#[derive(Debug, Clone)]
pub struct CurrentMap {
    pub map_index: i32,
    pub file_name: String,
    pub title: String,
}

impl CurrentMap {
    pub fn new(map_index: i32, file_name: String, title: String) -> Self {
        Self {
            map_index,
            file_name,
            title,
        }
    }
}

impl Default for CurrentMap {
    fn default() -> Self {
        Self {
            map_index: 0,
            file_name: String::new(),
            title: String::new(),
        }
    }
}

/// 组队成员信息
#[derive(Debug, Clone)]
pub struct GroupMember {
    pub name: String,
    pub level: u16,
    pub health_percent: u8,  // 血量百分比 (0-100)
    pub mana_percent: u8,    // 魔法百分比 (0-100)
}

/// 组队数据
#[derive(Debug, Clone, Default)]
pub struct GroupData {
    /// 队伍成员列表
    pub members: Vec<GroupMember>,
    /// 队长名称
    pub leader: Option<String>,
    /// 是否在队伍中
    pub in_group: bool,
}

impl GroupData {
    pub fn new() -> Self {
        Self {
            members: Vec::new(),
            leader: None,
            in_group: false,
        }
    }
    
    /// 是否是队长
    pub fn is_leader(&self, name: &str) -> bool {
        self.leader.as_ref().map_or(false, |leader| leader == name)
    }
    
    /// 获取队伍人数
    pub fn member_count(&self) -> usize {
        self.members.len()
    }
    
    /// 清空队伍
    pub fn clear(&mut self) {
        self.members.clear();
        self.leader = None;
        self.in_group = false;
    }
}

/// 公会数据
#[derive(Debug, Clone, Default)]
pub struct GuildData {
    /// 公会名称
    pub guild_name: Option<String>,
    /// 公会成员列表
    pub members: Vec<GuildMember>,
    /// 是否在公会中
    pub in_guild: bool,
    /// 公会等级
    pub guild_level: u8,
    /// 公会经验
    pub guild_experience: i64,
}

impl GuildData {
    pub fn new() -> Self {
        Self {
            guild_name: None,
            members: Vec::new(),
            in_guild: false,
            guild_level: 1,
            guild_experience: 0,
        }
    }
    
    /// 获取公会成员数量
    pub fn member_count(&self) -> usize {
        self.members.len()
    }
    
    /// 清空公会数据
    pub fn clear(&mut self) {
        self.guild_name = None;
        self.members.clear();
        self.in_guild = false;
        self.guild_level = 1;
        self.guild_experience = 0;
    }
}

/// 好友列表
#[derive(Debug, Clone, Default)]
pub struct FriendList {
    /// 好友列表
    pub friends: Vec<ClientFriend>,
}

impl FriendList {
    pub fn new() -> Self {
        Self {
            friends: Vec::new(),
        }
    }
    
    /// 添加好友
    pub fn add_friend(&mut self, friend: ClientFriend) {
        if !self.friends.iter().any(|f| f.index == friend.index) {
            self.friends.push(friend);
        }
    }
    
    /// 移除好友
    pub fn remove_friend(&mut self, index: i32) {
        self.friends.retain(|f| f.index != index);
    }
    
    /// 查找好友
    pub fn find_friend(&self, name: &str) -> Option<&ClientFriend> {
        self.friends.iter().find(|f| f.name == name)
    }
    
    /// 获取好友数量
    pub fn friend_count(&self) -> usize {
        self.friends.len()
    }
}

/// 活动任务列表
#[derive(Debug, Clone, Default)]
pub struct ActiveQuests {
    /// 进行中的任务列表
    pub quests: Vec<ClientQuestProgress>,
}

impl ActiveQuests {
    pub fn new() -> Self {
        Self {
            quests: Vec::new(),
        }
    }
    
    /// 添加任务
    pub fn add_quest(&mut self, quest: ClientQuestProgress) {
        // 检查是否已存在
        if let Some(existing) = self.quests.iter_mut().find(|q| q.index == quest.index) {
            *existing = quest; // 更新现有任务
        } else {
            self.quests.push(quest); // 添加新任务
        }
    }
    
    /// 移除任务
    pub fn remove_quest(&mut self, index: i32) {
        self.quests.retain(|q| q.index != index);
    }
    
    /// 查找任务
    pub fn find_quest(&self, index: i32) -> Option<&ClientQuestProgress> {
        self.quests.iter().find(|q| q.index == index)
    }
    
    /// 获取任务数量
    pub fn quest_count(&self) -> usize {
        self.quests.len()
    }
}

/// 交易状态（也可以作为组件）
#[derive(Debug, Clone)]
pub struct TradingState {
    /// 正在交易的玩家名称
    pub trading_with: Option<String>,
    /// 是否已确认
    pub confirmed: bool,
    /// 交易中的物品列表
    pub my_items: Vec<Option<mir2_shared::data::item::UserItem>>,
    /// 对方的物品列表
    pub their_items: Vec<Option<mir2_shared::data::item::UserItem>>,
    /// 我的金币
    pub my_gold: u32,
    /// 对方的金币
    pub their_gold: u32,
}

impl TradingState {
    pub fn new() -> Self {
        Self {
            trading_with: None,
            confirmed: false,
            my_items: vec![None; 10], // 交易窗口10格
            their_items: vec![None; 10],
            my_gold: 0,
            their_gold: 0,
        }
    }
    
    /// 是否正在交易
    pub fn is_trading(&self) -> bool {
        self.trading_with.is_some()
    }
    
    /// 开始交易
    pub fn start_trade(&mut self, player_name: String) {
        self.trading_with = Some(player_name);
        self.confirmed = false;
        self.clear_items();
    }
    
    /// 结束交易
    pub fn end_trade(&mut self) {
        self.trading_with = None;
        self.confirmed = false;
        self.clear_items();
    }
    
    /// 清空交易物品
    fn clear_items(&mut self) {
        self.my_items = vec![None; 10];
        self.their_items = vec![None; 10];
        self.my_gold = 0;
        self.their_gold = 0;
    }
}

impl Default for TradingState {
    fn default() -> Self {
        Self::new()
    }
}

/// 英雄数据
#[derive(Debug, Clone)]
pub struct HeroData {
    pub object_id: u32,
    pub name: String,
    pub level: u16,
    pub class: mir2_shared::enums::MirClass,
    pub gender: mir2_shared::enums::MirGender,
}

impl HeroData {
    pub fn new(object_id: u32, name: String, level: u16, class: mir2_shared::enums::MirClass, gender: mir2_shared::enums::MirGender) -> Self {
        Self {
            object_id,
            name,
            level,
            class,
            gender,
        }
    }
}
