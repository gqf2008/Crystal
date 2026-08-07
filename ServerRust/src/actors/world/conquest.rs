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
    /// 每个控制点的占领进度 (index -> (owner_guild, progress 0..MAX_CONTROL_POINTS))
    pub control_point_owners: Vec<ControlPointState>,
    /// 最大分
    pub max_points: i32,
    /// 属于本区域的攻城结构 object_id 列表（城墙/城门/攻城器）
    pub siege_structure_ids: Vec<u32>,
    /// 攻城金库（C# GuildInfo.GoldStorage，TAKECONQUESTGOLD 取走）
    pub gold_storage: u64,
    /// NPC 税率（C# GuildInfo.NPCRate，SETCONQUESTRATE 设置）
    pub tax_rate: u8,
    /// 行会领地是否挂售（GTSALE/GTCANCELSALE）
    pub for_sale: bool,
    /// 挂售价格（GTSALE <price>）
    pub sale_price: u64,
    /// 领地剩余租期（天，EXTENDGT 延长）
    pub rent_days: u32,
}

/// 控制点占领状态（对应 C# ControlPoints dict 的 entry）
#[derive(Debug, Clone, Default)]
pub struct ControlPointState {
    /// 当前占领该点的公会名
    pub owner_guild: Option<String>,
    /// 占领进度（0..MAX_CONTROL_POINTS，满值即占领）
    pub progress: i32,
    /// 当前争夺中的公会名（最近站上去的）
    pub contesting_guild: Option<String>,
}

/// 控制点占领阈值（对应 C# MAX_CONTROL_POINTS = 6）
pub const MAX_CONTROL_POINTS: i32 = 6;
/// KingOfHill 胜利阈值（对应 C# MAX_KING_POINTS = 18）
pub const MAX_KING_POINTS: i32 = 18;

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
            control_point_owners: Vec::new(),
            max_points: MAX_KING_POINTS,
            siege_structure_ids: Vec::new(),
            gold_storage: 0,
            tax_rate: 0,
            for_sale: false,
            sale_price: 0,
            rent_days: 30,
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
        // 重置控制点占领状态
        for cp in &mut self.control_point_owners {
            cp.owner_guild = None;
            cp.progress = 0;
            cp.contesting_guild = None;
        }
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

    /// 重置领地（C# ConquestObject.Reset：清空占领/攻击方/积分/控制点，恢复 Idle）
    pub fn reset(&mut self) {
        self.state = WarState::Idle;
        self.owner_guild = None;
        self.attacker_guild = None;
        self.war_start_time = 0;
        self.scores.clear();
        for cp in &mut self.control_point_owners {
            cp.owner_guild = None;
            cp.progress = 0;
            cp.contesting_guild = None;
        }
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

    /// 判断本区域是否已"破城"（所有城墙/城门均已破损）。
    /// 用于决定进攻方能否进入内城。
    pub fn is_breached(&self, structures: &std::collections::HashMap<u32, SiegeStructure>) -> bool {
        let mut blocking_alive = 0u32;
        for oid in &self.siege_structure_ids {
            if let Some(s) = structures.get(oid) {
                if (s.structure_type == SiegeStructureType::Wall
                    || s.structure_type == SiegeStructureType::CastleGate)
                    && s.hp > 0
                {
                    blocking_alive += 1;
                }
            }
        }
        // 如果本区域原本就没有城墙/城门，视为已破城
        self.siege_structure_ids.is_empty() || blocking_alive == 0
    }

    /// 统计本区域的控制点占领数，返回各公会控制的点数。
    pub fn tally_control_points(&self) -> std::collections::HashMap<String, i32> {
        let mut out: std::collections::HashMap<String, i32> = std::collections::HashMap::new();
        for cp in &self.control_point_owners {
            if let Some(ref g) = cp.owner_guild {
                *out.entry(g.clone()).or_insert(0) += 1;
            }
        }
        out
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
    /// 所在坐标（攻城器选择最近城墙用）
    pub x: i32,
    pub y: i32,
    /// 所属征服区域 ID（多个城堡同时开战时区分）
    pub conquest_id: i32,
}

#[derive(Debug, Clone, PartialEq)]
pub enum SiegeStructureType {
    CastleGate,
    Wall,
    ArcherTower,
    Catapult, // 攻城器（投石车），攻方用来打墙
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
            x: 0,
            y: 0,
            conquest_id: 0,
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
            x: 0,
            y: 0,
            conquest_id: 0,
        }
    }

    /// 攻城器（投石车），攻方武器，本身有 HP
    pub fn catapult(object_id: u32) -> Self {
        Self {
            object_id,
            structure_type: SiegeStructureType::Catapult,
            max_hp: 5000,
            hp: 5000,
            damage_level: 0,
            is_open: false,
            owner_guild: None,
            x: 0,
            y: 0,
            conquest_id: 0,
        }
    }

    /// 箭塔（守方自动射击，本简化版仅作为可被摧毁的目标）
    pub fn archer_tower(object_id: u32) -> Self {
        Self {
            object_id,
            structure_type: SiegeStructureType::ArcherTower,
            max_hp: 10000,
            hp: 10000,
            damage_level: 0,
            is_open: false,
            owner_guild: None,
            x: 0,
            y: 0,
            conquest_id: 0,
        }
    }

    /// 是否为阻挡类结构（城墙/城门，HP 归零后通过）
    pub fn is_blocking(&self) -> bool {
        matches!(self.structure_type, SiegeStructureType::Wall | SiegeStructureType::CastleGate)
    }

    /// 是否已被摧毁（HP 归零，破损）
    pub fn is_destroyed(&self) -> bool {
        self.hp <= 0
    }

    /// 受到伤害，返回是否在本次打击中被摧毁
    pub fn take_damage(&mut self, damage: i32) -> bool {
        let was_alive = self.hp > 0;
        self.hp = self.hp.saturating_sub(damage);
        let pct = self.hp as f32 / self.max_hp as f32;
        self.damage_level = ((1.0 - pct) * 5.0).min(4.0) as u8;
        was_alive && self.hp <= 0
    }

    /// 修理
    pub fn repair(&mut self, amount: i32) {
        self.hp = (self.hp + amount).min(self.max_hp);
        let pct = self.hp as f32 / self.max_hp as f32;
        self.damage_level = ((1.0 - pct) * 5.0).min(4.0) as u8;
    }
}

/// 攻城器每次攻击对城墙造成的固定伤害（简化值，对齐 C# Siege 的 Strike 伤害量级）
pub const CATAPULT_DAMAGE_PER_HIT: i32 = 800;
/// 攻城器攻击间隔（ticks，10 ticks ≈ 1 秒）
pub const CATAPULT_ATTACK_INTERVAL: u64 = 10;

/// 选择离攻城器 (x,y) 最近的、未摧毁的城墙/城门 object_id。
/// 攻城器只打本 conquest_id 区域内的目标。
pub fn find_nearest_target(
    catapult_x: i32,
    catapult_y: i32,
    conquest_id: i32,
    structures: &std::collections::HashMap<u32, SiegeStructure>,
    candidate_ids: &[u32],
) -> Option<u32> {
    let mut best: Option<(u32, i32)> = None;
    for oid in candidate_ids {
        let s = structures.get(oid)?;
        if s.conquest_id != conquest_id { continue; }
        if !s.is_blocking() || s.is_destroyed() { continue; }
        let dist = (s.x - catapult_x).abs() + (s.y - catapult_y).abs();
        if best.is_none_or(|(_, d)| dist < d) {
            best = Some((*oid, dist));
        }
    }
    best.map(|(oid, _)| oid)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_take_damage_and_destroy() {
        let mut wall = SiegeStructure::wall(1);
        // 100 dmg on 30000 hp → barely scratched, damage_level still 0
        assert!(!wall.take_damage(100));
        assert_eq!(wall.damage_level, 0);
        // Heavy damage (~50%) → damage_level should be ~2
        assert!(!wall.take_damage(14000));
        assert!(wall.damage_level >= 2);
        let destroyed = wall.take_damage(wall.max_hp);
        assert!(destroyed);
        assert!(wall.is_destroyed());
    }

    #[test]
    fn test_find_nearest_target() {
        let mut map = std::collections::HashMap::new();
        let mut w1 = SiegeStructure::wall(10);
        w1.conquest_id = 1; w1.x = 5; w1.y = 5;
        let mut w2 = SiegeStructure::wall(11);
        w2.conquest_id = 1; w2.x = 20; w2.y = 20;
        map.insert(10, w1); map.insert(11, w2);
        let ids = vec![10u32, 11u32];
        let target = find_nearest_target(0, 0, 1, &map, &ids);
        assert_eq!(target, Some(10));
    }

    #[test]
    fn test_is_breached() {
        let mut inst = ConquestInstance::new(1, 0, 0, ConquestGame::Classic);
        let mut structures = std::collections::HashMap::new();
        let mut w = SiegeStructure::wall(1);
        w.conquest_id = 1;
        structures.insert(1, w);
        inst.siege_structure_ids = vec![1];
        assert!(!inst.is_breached(&structures));
        structures.get_mut(&1).unwrap().hp = 0;
        assert!(inst.is_breached(&structures));
    }

    #[test]
    fn test_conquest_war_lifecycle() {
        // #928：@STARTCONQUEST / @RESETCONQUEST 依赖的状态流转
        let mut inst = ConquestInstance::new(1, 3, 2, ConquestGame::CapturePalace);
        assert_eq!(inst.state, WarState::Idle);
        inst.start_war("攻击行会");
        assert_eq!(inst.state, WarState::InProgress);
        assert_eq!(inst.attacker_guild.as_deref(), Some("攻击行会"));
        inst.add_score("攻击行会", 5);
        inst.add_score("守方行会", 8);
        let winner = inst.end_war();
        assert_eq!(inst.state, WarState::Ended);
        assert_eq!(winner.as_deref(), Some("守方行会"));
        assert_eq!(inst.owner_guild.as_deref(), Some("守方行会"));
        inst.reset();
        assert_eq!(inst.state, WarState::Idle);
        assert!(inst.owner_guild.is_none());
        assert!(inst.attacker_guild.is_none());
        assert!(inst.scores.is_empty());
        inst.start_war("新攻击方");
        assert_eq!(inst.state, WarState::InProgress);
        assert_eq!(inst.attacker_guild.as_deref(), Some("新攻击方"));
    }
}

impl SiegeStructure {
    /// 修复费用（简化：按缺失 HP 比例，对齐 C# GetRepairCost 概念；每 100 点缺失收 5 金币）
    pub fn repair_cost(&self) -> u64 {
        let missing = (self.max_hp - self.hp).max(0) as u64;
        missing / 100 * 5
    }

    /// 满修复（HP 回满 + 损伤等级清零；区分已有 repair(amount)）
    pub fn repair_full(&mut self) {
        self.hp = self.max_hp;
        self.damage_level = 0;
    }
}