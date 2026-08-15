/// 攻城/征服系统，对应 C# ConquestObject.cs + ConquestGuildInfo.cs
/// 公会争夺城堡领土的每周活动
use chrono::{Datelike, Timelike};

/// 世界 tick 数/天（100ms/tick × 86400s）
pub(crate) const TICKS_PER_DAY: u64 = 864_000;

/// 征服类型（C# Shared/Enums.cs ConquestType : byte）
pub const CONQUEST_TYPE_REQUEST: i32 = 0; // 宣战制：需 npc_schedule_conquest 注册攻方
pub const CONQUEST_TYPE_AUTO: i32 = 1; // 自动：到点即开战
pub const CONQUEST_TYPE_FORCED: i32 = 2; // 强制：脚本/GM 触发

/// 征服游戏模式
#[derive(Debug, Clone, PartialEq)]
pub enum ConquestGame {
    CapturePalace, // 占领皇宫
    KingOfHill,    // 占山为王
    Classic,       // 经典模式
    ControlPoints, // 控制点
}

/// 战争状态
#[derive(Debug, Clone, PartialEq)]
pub enum WarState {
    Idle,
    Declared,   // 已宣战
    InProgress, // 战斗中
    Ended,      // 已结束
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
    /// 每周开战布尔（Mon..Sun；DB monday..sunday 列序）
    pub days: [bool; 7],
    /// 开战小时（0-23；C# ConquestInfo.StartHour）
    pub start_hour: i32,
    /// 战争时长（分钟；C# ConquestInfo.WarLength）
    pub war_length: i32,
    /// 征服类型（C# ConquestType：0=Request 宣战制 / 1=Auto 自动 / 2=Forced 强制）
    pub conquest_type: i32,
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
    /// 领地守卫/箭塔落点（C# ConquestInfo.ConquestGuards；CONQUESTGUARD 按 id 落点）
    pub guards: Vec<ConquestGuardInfo>,
    /// 城门（C# ConquestInfo.ConquestGates；启动生成 SiegeStructure）
    pub gates: Vec<ConquestGuardInfo>,
    /// 城墙（C# ConquestInfo.ConquestWalls；启动生成 SiegeStructure）
    pub walls: Vec<ConquestGuardInfo>,
    /// 领地旗子 NPC 落点（C# ConquestInfo.ConquestFlags；登录/换图时生成 ObjectNpc）
    pub flags: Vec<ConquestFlagInfo>,
    /// 攻城金库（C# GuildInfo.GoldStorage，TAKECONQUESTGOLD 取走）
    pub gold_storage: u64,
    /// NPC 税率（C# GuildInfo.NPCRate，SETCONQUESTRATE 设置）
    pub tax_rate: u8,
    /// 行会领地是否挂售（GTSALE/GTCANCELSALE）
    pub for_sale: bool,
    /// 挂售价格（GTSALE <price>）
    pub sale_price: u64,
    /// 领地租期到期 tick（0=未拥有/已到期；C# GTRent > Now 语义）
    pub rent_expire_tick: u64,
    /// 动态态脏标记（C# GuildInfo.NeedSave：变更置位，持久化时清除）
    pub need_save: bool,
}

/// 领地守卫/箭塔落点（对应 C# ConquestArcherInfo / ConquestGuildArcherInfo）
#[derive(Debug, Clone)]
pub struct ConquestGuardInfo {
    pub index: i32,
    pub x: i32,
    pub y: i32,
    pub mob_index: i32,
    pub name: String,
    pub repair_cost: u32,
}

/// 领地旗子 NPC 落点（对应 C# ConquestFlagInfo：ConquestInfo.ConquestFlags）
#[derive(Debug, Clone)]
pub struct ConquestFlagInfo {
    pub index: i32,
    pub x: i32,
    pub y: i32,
    pub name: String,
    pub file_name: String,
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
/// 控制点判定半径（C# ScorePoints：Functions.InRange(flag, player, 3)）
pub const CONTROL_POINT_RADIUS: i32 = 3;
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
            war_duration_secs: 3600,           // 1 hour default
            days: [true; 7],                   // C# CheckDay 未知星期默认 true
            start_hour: 20,                    // 8 PM
            war_length: 60,                    // C# ConquestInfo.WarLength 默认 60 分钟
            conquest_type: CONQUEST_TYPE_AUTO, // 代码种子自动开战（C# 默认 Request，DB 行会覆盖）
            scores: std::collections::HashMap::new(),
            king_zone: None,
            control_points: Vec::new(),
            control_point_owners: Vec::new(),
            max_points: MAX_KING_POINTS,
            siege_structure_ids: Vec::new(),
            guards: Vec::new(),
            gates: Vec::new(),
            walls: Vec::new(),
            flags: Vec::new(),
            gold_storage: 0,
            tax_rate: 0,
            for_sale: false,
            sale_price: 0,
            rent_expire_tick: 0,
            need_save: false,
        }
    }

    /// C# GuildInfo.HasGT（GTRent > Now）：剩余租期天数（向上取整；0 = 无/已到期）
    pub fn gt_days_left(&self, now_tick: u64) -> u32 {
        if self.rent_expire_tick == 0 || now_tick >= self.rent_expire_tick {
            0
        } else {
            (self.rent_expire_tick - now_tick).div_ceil(TICKS_PER_DAY) as u32
        }
    }

    /// 附加守卫落点（DB 加载后设置；C# ConquestInfo.ConquestGuards）
    pub fn with_guards(mut self, guards: Vec<ConquestGuardInfo>) -> Self {
        self.guards = guards;
        self
    }

    /// 是否处于非战争状态（C# !WarIsOn；Rust 侧 Idle=未开战/已复位，Ended=战毕待再武装）
    pub fn is_peace(&self) -> bool {
        self.state == WarState::Idle || self.state == WarState::Ended
    }

    /// 检查是否到开战时间（C# ConquestObject.AutoSchedule + CheckDay：
    /// 星期布尔 + [StartHour, StartHour+WarLength) 分钟窗口内且未在战）
    pub fn should_start_war(&self, now: &chrono::NaiveDateTime) -> bool {
        if !self.is_peace() {
            return false;
        }
        // days 下标 Mon=0..Sun=6（与 DB monday..sunday 列序一致）
        let day = now.weekday().num_days_from_monday() as usize;
        if !self.days.get(day).copied().unwrap_or(true) {
            return false;
        }
        let now_min = now.hour() as i32 * 60 + now.minute() as i32;
        let start = self.start_hour * 60;
        let finish = start + self.war_length;
        start <= now_min && now_min < finish
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

    /// 结束战争（兼容入口：GM @STARTCONQUEST 停止等无玩家上下文的调用点，
    /// 按当前积分最高者收口；模式化战末判定走 end_war_with）
    pub fn end_war(&mut self) -> Option<String> {
        let winner = self
            .scores
            .iter()
            .filter(|(_, score)| **score > 0)
            .max_by_key(|(_, score)| **score)
            .map(|(g, _)| g.clone());
        self.end_war_with(winner)
    }

    /// 战末统一收口（C# EndWar + AtWarChanged(false) + TakeConquest）：
    /// 置 Ended；清宣战方；winner 经 take_conquest 易主（含 Request 门槛与前 owner 转攻方）
    pub fn end_war_with(&mut self, winner: Option<String>) -> Option<String> {
        self.state = WarState::Ended;
        self.attacker_guild = None; // C# AtWarChanged(false)：Request 型战末清 AttackerID
        if let Some(g) = winner {
            self.take_conquest(&g);
        }
        self.need_save = true;
        self.owner_guild.clone()
    }

    /// 易主（C# ConquestObject.TakeConquest 核心）：
    /// - Request 型仅宣战行会可夺（AttackerID 门槛）；
    /// - 前 owner 转为下一任宣战方（C# `GuildInfo.AttackerID = tmpPrevious.Guildindex`）；
    /// - 返回是否实际变更（夺方已持有/门槛不满足 → false）
    pub fn take_conquest(&mut self, guild: &str) -> bool {
        if self.conquest_type == CONQUEST_TYPE_REQUEST
            && self.attacker_guild.as_deref() != Some(guild)
        {
            return false;
        }
        if self.owner_guild.as_deref() == Some(guild) {
            return false;
        }
        let prev = self.owner_guild.take();
        self.owner_guild = Some(guild.to_string());
        self.attacker_guild = prev;
        self.need_save = true;
        true
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
        self.need_save = true;
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

    /// ControlPoints 单点每跳拉锯判定（C# ScorePoints ControlPoints 分支的简化）：
    /// guilds_here 为该点范围内行会去重列表——
    /// - 空：进度回落；
    /// - 单行会：进度 +1，满 MAX_CONTROL_POINTS(6) 完成占领并计 1 分；
    /// - 多行会：进度互相抵消回落。
    /// 返回本跳完成占领的行会（分数已在内部累加）。
    pub fn tick_control_point(&mut self, idx: usize, guilds_here: &[String]) -> Option<String> {
        let captured = {
            let cp = self.control_point_owners.get_mut(idx)?;
            if guilds_here.is_empty() {
                if cp.progress > 0 {
                    cp.progress -= 1;
                }
                cp.contesting_guild = None;
                return None;
            }
            if guilds_here.len() == 1 {
                let g = &guilds_here[0];
                cp.contesting_guild = Some(g.clone());
                if cp.owner_guild.as_deref() == Some(g.as_str()) {
                    return None; // 已是拥有者，维持
                }
                cp.progress += 1;
                if cp.progress >= MAX_CONTROL_POINTS {
                    cp.owner_guild = Some(g.clone());
                    cp.progress = 0;
                    Some(g.clone())
                } else {
                    None
                }
            } else {
                // 多行会争夺：进度互相抵消，无人增长
                cp.contesting_guild = None;
                if cp.progress > 0 {
                    cp.progress -= 1;
                }
                None
            }
        };
        if let Some(ref g) = captured {
            self.add_score(g, 1);
        }
        captured
    }

    /// KingOfHill 每跳计分（C# ScorePoints KingOfHill：圈内行会每跳 +1 封顶
    /// MAX_KING_POINTS(18)，圈外/被压制行会 -1；领先者 != 当前 owner → 即时易主）。
    /// 返回本跳易主的新 owner（战争继续，可反复易主）。
    pub fn tick_king_of_hill(&mut self, guilds_in_zone: &[String]) -> Option<String> {
        if guilds_in_zone.is_empty() {
            return None; // C#：圈内无人时不衰减
        }
        for g in guilds_in_zone {
            let e = self.scores.entry(g.clone()).or_insert(0);
            if *e < MAX_KING_POINTS {
                *e += 1;
            }
        }
        for (g, pts) in self.scores.iter_mut() {
            if !guilds_in_zone.iter().any(|z| z == g) && *pts > 0 {
                *pts -= 1;
            }
        }
        let leader = self
            .scores
            .iter()
            .filter(|(_, v)| **v > 0)
            .max_by_key(|(_, v)| **v)
            .map(|(g, _)| g.clone())?;
        if self.owner_guild.as_deref() == Some(leader.as_str()) {
            return None;
        }
        if self.take_conquest(&leader) {
            Some(leader)
        } else {
            None
        }
    }

    /// Classic 每跳判定（C# ScorePoints Classic：宫殿内全部玩家属于同一行会 → 即时易主）。
    /// 返回本跳易主的新 owner。
    pub fn tick_classic(&mut self, players_in_palace: &[Option<String>]) -> Option<String> {
        let guild = classic_unique_guild(players_in_palace)?;
        if self.owner_guild.as_deref() == Some(guild.as_str()) {
            return None;
        }
        if self.take_conquest(&guild) {
            Some(guild)
        } else {
            None
        }
    }

    /// CapturePalace 进殿即时易主（C# PlayerObject.CheckConquest(checkPalace=true) →
    /// TakeConquest(player)：玩家进宫殿图触发；CapturePalace 分支随后 EndWar）。
    /// 返回易主的新 owner（战争立即结束）。
    pub fn tick_capture_palace(&mut self, player_guilds_in_palace: &[String]) -> Option<String> {
        for g in player_guilds_in_palace {
            if self.take_conquest(g) {
                // C# TakeConquest CapturePalace 分支：拿下宫殿即战争结束
                self.state = WarState::Ended;
                self.attacker_guild = None;
                return Some(g.clone());
            }
        }
        None
    }
}

/// 宫殿内玩家的唯一行会判定（C# ScorePoints Classic 的 guildCounter 逻辑：
/// 同行会多玩家只算 1 个单位，无行会玩家各算 1 个单位；counter==1 才有效）
pub fn classic_unique_guild(players_in_palace: &[Option<String>]) -> Option<String> {
    let mut taking: Option<String> = None;
    let mut counter = 0usize;
    for g in players_in_palace {
        match g {
            Some(name) => {
                if taking.as_deref() != Some(name.as_str()) {
                    counter += 1;
                    taking = Some(name.clone());
                }
            }
            None => counter += 1,
        }
    }
    if counter == 1 {
        taking
    } else {
        None
    }
}

/// 旗子 NPC 外观（C# ConquestGuildFlagInfo.Spawn：无归属默认 Image=1000/Colour=Color.Empty(0)；
/// 有归属用行会 GuildInfo.FlagImage/FlagColour）
pub fn conquest_flag_appearance(owner_guild_flag: Option<(u16, i32)>) -> (u16, i32) {
    owner_guild_flag.unwrap_or((1000, 0))
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
    /// 结构索引（C# ConquestGateInfo/ConquestWallInfo.Index；OPENGATE 等脚本按此定位）
    pub index: i32,
    /// 修复费用（C# Info.RepairCost，GetRepairCost 公式消费）
    pub repair_cost: u32,
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
            index: 0,
            repair_cost: 0,
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
            index: 0,
            repair_cost: 0,
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
            index: 0,
            repair_cost: 0,
        }
    }

    /// 箭塔（守方自动射击已实现：#1513 tick_conquest 战争期间每 3s 攻击非守方玩家）
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
            index: 0,
            repair_cost: 0,
        }
    }

    /// 启动时按数据放置城门/城墙（#1523：坐标/索引/修复费来自 ConquestInfo.Gates/Walls）
    pub fn placed(
        mut self,
        index: i32,
        x: i32,
        y: i32,
        repair_cost: u32,
        conquest_id: i32,
    ) -> Self {
        self.index = index;
        self.x = x;
        self.y = y;
        self.repair_cost = repair_cost;
        self.conquest_id = conquest_id;
        self
    }

    /// 是否为阻挡类结构（#1431：C# CastleGate.Blocking => Closed && base.Blocking——
    /// 开门（is_open）的城门不阻挡；城墙始终阻挡，HP 归零后由调用方过滤）
    pub fn is_blocking(&self) -> bool {
        match self.structure_type {
            SiegeStructureType::Wall => true,
            SiegeStructureType::CastleGate => !self.is_open,
            _ => false,
        }
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
        if s.conquest_id != conquest_id {
            continue;
        }
        if !s.is_blocking() || s.is_destroyed() {
            continue;
        }
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
    #[test]
    fn test_repair_cost_formula() {
        // #1524：C# GetRepairCost——城门/城墙：RepairCost / (MaxHP / (MaxHP - HP))；箭塔：死亡全额
        // 满血 0
        let mut gate = SiegeStructure::gate(1);
        assert_eq!(gate.repair_cost(), 0);
        // 城门缺一半：100000 / (50000/25000=2) = 50000
        let mut gate2 = SiegeStructure::gate(2).placed(0, 0, 0, 100000, 1);
        gate2.hp = 25000;
        assert_eq!(gate2.repair_cost(), 50000);
        // 城墙缺 1/3：60000 / (30000/10000=3) = 20000
        let mut wall = SiegeStructure::wall(3).placed(0, 0, 0, 60000, 1);
        wall.hp = 20000;
        assert_eq!(wall.repair_cost(), 20000);
        // 箭塔：存活 0，死亡全额
        let mut tower = SiegeStructure::archer_tower(4).placed(0, 0, 0, 8000, 1);
        assert_eq!(tower.repair_cost(), 0);
        tower.hp = 0;
        assert_eq!(tower.repair_cost(), 8000);
    }

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
        w1.conquest_id = 1;
        w1.x = 5;
        w1.y = 5;
        let mut w2 = SiegeStructure::wall(11);
        w2.conquest_id = 1;
        w2.x = 20;
        w2.y = 20;
        map.insert(10, w1);
        map.insert(11, w2);
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
    #[test]
    fn test_gate_is_blocking_follows_is_open() {
        // #1431：C# CastleGate.Blocking => Closed && base.Blocking——开门不阻挡
        let mut gate = SiegeStructure::gate(1);
        assert!(gate.is_blocking(), "关门默认阻挡");
        gate.is_open = true;
        assert!(!gate.is_blocking(), "开门不阻挡");
        gate.is_open = false;
        gate.hp = 0;
        assert!(
            gate.is_blocking(),
            "is_blocking 只看开关门；已摧毁由调用方过滤 is_destroyed"
        );
        // 城墙始终阻挡（无开关门）
        let wall = SiegeStructure::wall(2);
        assert!(wall.is_blocking());
    }

    #[test]
    fn test_find_nearest_target_skips_open_gate() {
        // #1431：攻城器目标选择跳过已开门城门（is_blocking=false）
        let mut map = std::collections::HashMap::new();
        let mut w1 = SiegeStructure::wall(10);
        w1.conquest_id = 1;
        w1.x = 5;
        w1.y = 5;
        let mut g2 = SiegeStructure::gate(11);
        g2.conquest_id = 1;
        g2.x = 20;
        g2.y = 20;
        g2.is_open = true;
        map.insert(10, w1);
        map.insert(11, g2);
        let ids = vec![10u32, 11u32];
        assert_eq!(find_nearest_target(0, 0, 1, &map, &ids), Some(10));
        // 若只有开门城门 → 无目标
        map.remove(&10);
        assert_eq!(find_nearest_target(0, 0, 1, &map, &ids), None);
    }

    #[test]
    fn test_conquest_flag_appearance() {
        // C# ConquestGuildFlagInfo.Spawn：无归属 (1000, Color.Empty=0)；有归属用行会旗标
        assert_eq!(conquest_flag_appearance(None), (1000, 0));
        assert_eq!(
            conquest_flag_appearance(Some((1200, 0xFFFF0000u32 as i32))),
            (1200, 0xFFFF0000u32 as i32)
        );
    }

    #[test]
    fn test_should_start_war_schedule() {
        // #2568：C# AutoSchedule——星期布尔 + [StartHour, StartHour+WarLength) 分钟窗口
        let mut inst = ConquestInstance::new(1, 0, 0, ConquestGame::Classic);
        // 仅周一、20:00 开战、打 60 分钟
        inst.days = [true, false, false, false, false, false, false];
        inst.start_hour = 20;
        inst.war_length = 60;
        let at = |y: i32, m: u32, d: u32, h: u32, mi: u32| {
            chrono::NaiveDate::from_ymd_opt(y, m, d)
                .unwrap()
                .and_hms_opt(h, mi, 0)
                .unwrap()
        };
        // 2026-08-10 是周一；7 天 × 窗口内外
        assert!(
            inst.should_start_war(&at(2026, 8, 10, 20, 0)),
            "周一开战时刻"
        );
        assert!(inst.should_start_war(&at(2026, 8, 10, 20, 59)), "窗口内");
        assert!(
            !inst.should_start_war(&at(2026, 8, 10, 21, 0)),
            "窗口结束（左闭右开）"
        );
        assert!(
            !inst.should_start_war(&at(2026, 8, 10, 19, 59)),
            "窗口未开始"
        );
        for (d, name) in [
            (11, "周二"),
            (12, "周三"),
            (13, "周四"),
            (14, "周五"),
            (15, "周六"),
            (16, "周日"),
        ] {
            assert!(
                !inst.should_start_war(&at(2026, 8, d, 20, 0)),
                "{name} 不开战"
            );
        }
        // 战争进行中即使到点也不重复开战
        inst.start_war("攻方");
        assert!(!inst.should_start_war(&at(2026, 8, 10, 20, 30)));
        // 战毕（Ended）→ 同窗口可再开战（C# !WarIsOn；每周循环不断链）
        inst.end_war();
        assert_eq!(inst.state, WarState::Ended);
        assert!(inst.should_start_war(&at(2026, 8, 10, 20, 45)));
    }

    #[test]
    fn test_control_point_capture_and_tally() {
        // #2568：control_points 填充后的拉锯计分（C# ScorePoints ControlPoints）
        let mut inst = ConquestInstance::new(1, 0, 0, ConquestGame::ControlPoints);
        // 从 conquest_flags 坐标填充（world 启动同款：半径 3）
        inst.control_points = vec![(10, 10, 3), (50, 50, 3)];
        inst.control_point_owners
            .resize(inst.control_points.len(), ControlPointState::default());
        inst.start_war("攻方");
        // 攻方站 0 号点：6 跳完成占领
        let attackers = vec!["攻方".to_string()];
        let mut captured = None;
        for _ in 0..MAX_CONTROL_POINTS {
            captured = inst.tick_control_point(0, &attackers);
        }
        assert_eq!(captured.as_deref(), Some("攻方"), "满 6 跳占领");
        assert_eq!(inst.scores.get("攻方"), Some(&1), "占领计 1 分");
        assert_eq!(
            inst.control_point_owners[0].owner_guild.as_deref(),
            Some("攻方")
        );
        // 多行会争夺：进度回落，不产生占领
        let contested = vec!["攻方".to_string(), "守方".to_string()];
        inst.control_point_owners[1].progress = 3;
        assert!(inst.tick_control_point(1, &contested).is_none());
        assert_eq!(inst.control_point_owners[1].progress, 2, "争夺回落");
        // 战末 tally：0 号点归攻方
        let tally = inst.tally_control_points();
        assert_eq!(tally.get("攻方"), Some(&1));
        inst.end_war_with(Some("攻方".to_string()));
        assert_eq!(inst.owner_guild.as_deref(), Some("攻方"));
        assert_eq!(inst.state, WarState::Ended);
    }

    #[test]
    fn test_king_of_hill_flip() {
        // #2568：KingOfHill 王座圈计分（C# ScorePoints KingOfHill：18 分封顶、领先即易主）
        let mut inst = ConquestInstance::new(1, 0, 0, ConquestGame::KingOfHill);
        inst.king_zone = Some((40, 40, 60, 60));
        assert!(inst.is_in_king_zone(50, 50));
        assert!(!inst.is_in_king_zone(39, 50));
        inst.owner_guild = Some("守方".to_string());
        inst.start_war("攻方");
        // 攻方持续占圈：每跳 +1，分数压过守方后即时易主（易主后攻方即 owner，后续跳不再变更）
        let zone = vec!["攻方".to_string()];
        let mut flipped = None;
        for _ in 0..5 {
            if let Some(g) = inst.tick_king_of_hill(&zone) {
                flipped = Some(g);
                break;
            }
        }
        assert_eq!(flipped.as_deref(), Some("攻方"), "领先者即时易主");
        assert_eq!(inst.owner_guild.as_deref(), Some("攻方"));
        assert_eq!(
            inst.attacker_guild.as_deref(),
            Some("守方"),
            "前 owner 转宣战方"
        );
        // 攻方已 5 分（守方 0 起步），封顶 18
        assert!(inst.scores.get("攻方").copied().unwrap_or(0) <= MAX_KING_POINTS);
        // 圈内无人：不衰减、不变更
        assert!(inst.tick_king_of_hill(&[]).is_none());
    }

    #[test]
    fn test_classic_unique_guild_in_palace() {
        // #2568：Classic 宫殿唯一行会（C# ScorePoints Classic：guildCounter==1）
        let mut inst = ConquestInstance::new(1, 0, 2, ConquestGame::Classic);
        inst.owner_guild = Some("守方".to_string());
        inst.start_war("攻方");
        // 两名同行会玩家在宫殿 → 唯一行会 → 易主
        let players = vec![Some("攻方".to_string()), Some("攻方".to_string())];
        assert_eq!(inst.tick_classic(&players).as_deref(), Some("攻方"));
        assert_eq!(inst.owner_guild.as_deref(), Some("攻方"));
        // 两行会同在 → 不易主
        let mixed = vec![Some("甲".to_string()), Some("乙".to_string())];
        assert!(inst.tick_classic(&mixed).is_none());
        // 无行会玩家算独立单位 → 不易主
        let with_unguilded = vec![Some("攻方".to_string()), None];
        assert!(inst.tick_classic(&with_unguilded).is_none());
        // 空宫殿 → 不易主
        assert!(inst.tick_classic(&[]).is_none());
        // free fn：无行会玩家独占 → counter==1 但无行会可判
        assert_eq!(
            classic_unique_guild(&[None, None]),
            None,
            "两名无行会 counter=2"
        );
    }

    #[test]
    fn test_capture_palace_immediate_flip() {
        // #2568：CapturePalace 进殿即时易主（C# PlayerObject.TakeConquest）
        let mut inst = ConquestInstance::new(1, 0, 2, ConquestGame::CapturePalace);
        inst.owner_guild = Some("守方".to_string());
        inst.start_war("攻方");
        assert_eq!(inst.state, WarState::InProgress);
        // 攻方玩家进宫殿 → 立即易主并结束战争
        let in_palace = vec!["攻方".to_string()];
        assert_eq!(
            inst.tick_capture_palace(&in_palace).as_deref(),
            Some("攻方")
        );
        assert_eq!(inst.owner_guild.as_deref(), Some("攻方"));
        assert_eq!(inst.state, WarState::Ended, "CapturePalace 拿宫殿即战终");
        assert!(inst.attacker_guild.is_none());
        assert!(inst.need_save, "易主置脏标记");
    }

    #[test]
    fn test_request_type_attacker_gate() {
        // #2568：C# AutoSchedule Request 型门槛——非宣战行会不能夺城
        let mut inst = ConquestInstance::new(1, 0, 2, ConquestGame::CapturePalace);
        inst.conquest_type = CONQUEST_TYPE_REQUEST;
        inst.attacker_guild = Some("宣战方".to_string());
        inst.state = WarState::InProgress;
        // 未宣战行会进殿：不夺城
        assert!(inst.tick_capture_palace(&["路过的".to_string()]).is_none());
        assert_eq!(inst.owner_guild, None);
        // 宣战行会进殿：夺城
        assert_eq!(
            inst.tick_capture_palace(&["宣战方".to_string()]).as_deref(),
            Some("宣战方")
        );
        assert_eq!(inst.owner_guild.as_deref(), Some("宣战方"));
    }

    #[test]
    fn test_take_conquest_prev_owner_becomes_attacker() {
        // C# TakeConquest：GuildInfo.AttackerID = tmpPrevious.Guildindex（前 owner 转攻方）
        let mut inst = ConquestInstance::new(1, 0, 0, ConquestGame::ControlPoints);
        inst.owner_guild = Some("旧主".to_string());
        assert!(inst.take_conquest("新王"));
        assert_eq!(inst.owner_guild.as_deref(), Some("新王"));
        assert_eq!(inst.attacker_guild.as_deref(), Some("旧主"));
        assert!(inst.need_save);
        // 夺方已持有 → 无变更
        assert!(!inst.take_conquest("新王"));
    }
}

impl SiegeStructure {
    /// 修复费用（#1524：C# ConquestGuildInfo.GetRepairCost）
    /// 城门/城墙：cost = RepairCost / (MaxHP / (MaxHP - HP))（整数除法，满血 0）
    /// 箭塔：死亡 → 全额 RepairCost；存活 → 0
    pub fn repair_cost(&self) -> u64 {
        if self.structure_type == SiegeStructureType::ArcherTower {
            return if self.hp <= 0 {
                self.repair_cost as u64
            } else {
                0
            };
        }
        let missing = (self.max_hp - self.hp).max(0) as u64;
        if missing == 0 {
            return 0;
        }
        let divisor = (self.max_hp as u64 / missing).max(1);
        self.repair_cost as u64 / divisor
    }

    /// 满修复（HP 回满 + 损伤等级清零；区分已有 repair(amount)）
    pub fn repair_full(&mut self) {
        self.hp = self.max_hp;
        self.damage_level = 0;
    }
}
