/// 攻城/征服系统，对应 C# ConquestObject.cs + ConquestGuildInfo.cs
/// 公会争夺城堡领土的每周活动

use chrono::{Datelike, Timelike};

/// 征服游戏模式
#[derive(Debug, Clone, PartialEq)]
pub enum ConquestGame {
    CapturePalace,  // 占领皇宫
    KingOfHill,     // 占山为王
    Classic,        // 经典模式
    ControlPoints,  // 控制点
}

/// 战争状态
#[derive(Debug, Clone, PartialEq)]
pub enum WarState {
    Idle,
    Declared,       // 已宣战
    InProgress,     // 战斗中
    Ended,          // 已结束
}

/// 单个征服区域的信息
#[derive(Debug, Clone)]
pub struct ConquestInstance {
    /// 征服区域 ID
    pub id: i32,
    /// 所属地图
    pub map_index: i32,
    /// 王座地图
    pub palace_map: i32,
    /// 游戏模式
    pub game: ConquestGame,
    /// 当前状态
    pub state: WarState,
    /// 当前拥有者公会名
    pub owner_guild: Option<String>,
    /// 进攻方公会名
    pub attacker_guild: Option<String>,
    /// 战争开始的时间（Unix秒）
    pub war_start_time: i64,
    /// 战争持续时间（秒）
    pub war_duration_secs: i64,
    /// 每周几开战（0=Sun..6=Sat）
    pub war_day: u32,
    /// 开战小时（0-23）
    pub war_hour: u32,
    /// 各公会积分 (guild_name -> points)
    pub scores: std::collections::HashMap<String, i32>,
    /// 王座区域 (x1, y1, x2, y2) 用于 KingOfHill
    pub king_zone: Option<(i32, i32, i32, i32)>,
    /// 控制点列表 (x, y, radius)
    pub control_points: Vec<(i32, i32, i32)>,
    /// 最大分
    pub max_points: i32,
}

impl ConquestInstance {
    pub fn new(id: i32, map_index: i32, palace_map: i32, game: ConquestGame) -> Self {
        Self {
            id,
            map_index,
            palace_map,
            game,
            state: WarState::Idle,
            owner_guild: None,
            attacker_guild: None,
            war_start_time: 0,
            war_duration_secs: 3600, // 1 hour default
            war_day: 6, // Saturday
            war_hour: 20, // 8 PM
            scores: std::collections::HashMap::new(),
            king_zone: None,
            control_points: Vec::new(),
            max_points: 100,
        }
    }

    /// 检查是否到开战时间
    pub fn should_start_war(&self, now: &chrono::NaiveDateTime) -> bool {
        self.state == WarState::Idle
            && now.weekday().num_days_from_sunday() == self.war_day
            && now.hour() == self.war_hour as u32
            && now.minute() == 0
    }

    /// 开始战争
    pub fn start_war(&mut self, attacker: &str) {
        self.state = WarState::InProgress;
        self.attacker_guild = Some(attacker.to_string());
        self.war_start_time = chrono::Utc::now().timestamp();
        self.scores.clear();
    }

    /// 结束战争
    pub fn end_war(&mut self) -> Option<String> {
        self.state = WarState::Ended;
        // Find winner: highest score
        let winner = self.scores.iter()
            .max_by_key(|(_, score)| **score)
            .map(|(g, _)| g.clone());
        if let Some(ref guild) = winner {
            self.owner_guild = Some(guild.clone());
        }
        self.attacker_guild = None;
        winner
    }

    /// 给指定公会加积分
    pub fn add_score(&mut self, guild: &str, points: i32) {
        let entry = self.scores.entry(guild.to_string()).or_insert(0);
        *entry = (*entry + points).min(self.max_points);
    }

    /// KingOfHill: 检查玩家是否在王座区域内
    pub fn is_in_king_zone(&self, x: i32, y: i32) -> bool {
        if let Some((x1, y1, x2, y2)) = self.king_zone {
            x >= x1 && x <= x2 && y >= y1 && y <= y2
        } else {
            false
        }
    }

    /// ControlPoints: 检查玩家是否在控制点范围内
    pub fn on_control_point(&self, x: i32, y: i32) -> Option<usize> {
        self.control_points.iter().position(|(cx, cy, r)| {
            let dx = (x - cx).abs();
            let dy = (y - cy).abs();
            dx <= *r && dy <= *r
        })
    }
}

/// 城门/城墙状态（对应 C# Gate/Wall/CastleGate）
#[derive(Debug, Clone)]
pub struct SiegeStructure {
    pub object_id: u32,
    pub structure_type: SiegeStructureType,
    pub max_hp: i32,
    pub hp: i32,
    pub damage_level: u8, // 0-4, higher = more damaged appearance
    pub is_open: bool,
    pub owner_guild: Option<String>,
}

#[derive(Debug, Clone, PartialEq)]
pub enum SiegeStructureType {
    CastleGate,
    Wall,
    ArcherTower,
    Catapult,
}

impl SiegeStructure {
    pub fn gate(object_id: u32) -> Self {
        Self {
            object_id,
            structure_type: SiegeStructureType::CastleGate,
            max_hp: 50000,
            hp: 50000,
            damage_level: 0,
            is_open: false,
            owner_guild: None,
        }
    }

    pub fn wall(object_id: u32) -> Self {
        Self {
            object_id,
            structure_type: SiegeStructureType::Wall,
            max_hp: 30000,
            hp: 30000,
            damage_level: 0,
            is_open: false,
            owner_guild: None,
        }
    }

    /// 受到伤害
    pub fn take_damage(&mut self, damage: i32) -> bool {
        self.hp = self.hp.saturating_sub(damage);
        let pct = self.hp as f32 / self.max_hp as f32;
        self.damage_level = ((1.0 - pct) * 5.0).min(4.0) as u8;
        self.hp <= 0
    }

    /// 修理
    pub fn repair(&mut self, amount: i32) {
        self.hp = (self.hp + amount).min(self.max_hp);
        let pct = self.hp as f32 / self.max_hp as f32;
        self.damage_level = ((1.0 - pct) * 5.0).min(4.0) as u8;
    }
}
