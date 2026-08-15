use crate::gate::actor::GateActor;
/// 龙系统，对应 C# MirEnvir/Dragon.cs
/// 龙身多部件Boss，经验累积→升级→掉落→定时降级
use kameo::actor::ActorRef;

/// 龙等级掉落条目（C# DragonInfo.DropInfo：DragonItem.txt）
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DragonDropEntry {
    /// 对应龙等级（1-12）
    pub level: u8,
    /// 1/chance 概率（C# DropInfo.Chance；rate = max(1, Chance/DropRate)）
    pub chance: i32,
    /// 物品名（None=金币条目；C# DropInfo.Item）
    pub item_name: Option<String>,
    /// 金币金额（C# DropInfo.Gold；>0 时物品为 None）
    pub gold: u64,
}

/// C# DragonInfo.DropInfo.FromLine：`1/3 Gold 30000 1` / `1/10 ItemName 3`
pub fn parse_dragon_drop_line(line: &str) -> Option<DragonDropEntry> {
    let line = line.trim();
    if line.is_empty() || line.starts_with(';') {
        return None;
    }
    let parts: Vec<&str> = line.split_whitespace().collect();
    if parts.len() < 3 {
        return None;
    }
    // 机会格式 "1/3" → 3（C# parts[0].Substring(2)）
    let chance: i32 = parts[0].strip_prefix("1/")?.parse().ok()?;
    if chance <= 0 {
        return None;
    }
    if parts[1].eq_ignore_ascii_case("gold") {
        if parts.len() < 4 {
            return None;
        }
        let gold: u64 = parts[2].parse().ok()?;
        let level: u8 = parts[3].parse().ok()?;
        Some(DragonDropEntry {
            level,
            chance,
            item_name: None,
            gold,
        })
    } else {
        let level: u8 = parts[2].parse().ok()?;
        Some(DragonDropEntry {
            level,
            chance,
            item_name: Some(parts[1].to_string()),
            gold: 0,
        })
    }
}

/// 龙的状态数据
#[derive(Debug, Clone)]
pub struct DragonState {
    /// 龙身主体怪物 object_id
    pub body_object_id: u32,
    /// 当前等级 (1-12)
    pub level: u8,
    /// 当前经验
    pub experience: u64,
    /// 升至满级的时间戳（Unix秒，满级后6小时自动降级回1级）
    pub max_level_time: i64,
    /// 上次降级检查时间（tick count）
    pub last_delevel_check: u64,
    /// 上次 spawn 检查时间（tick count，用于 EvilMir 生成节流）
    pub last_spawn_check: u64,
    /// 活跃状态
    pub active: bool,
    /// 当前 EvilMir 怪物 object_id（None = 未生成/已被击杀）
    pub evil_mir_oid: Option<u32>,
    /// 上次记录的 EvilMir HP（用于受击给龙加经验，C# EvilMir.ChangeHP → DragonSystem.GainExp）
    pub last_evil_mir_hp: i32,
    /// #2571：升级所需经验表（C# DragonInfo.Exps；空 → C# 线性默认 (i+1)*10000）
    pub exps: Vec<u64>,
    /// #2571：上次运行态落库 tick（经验变化节流用；运行时，不持久化）
    pub last_persist_tick: u64,
}

impl DragonState {
    pub fn new(body_object_id: u32) -> Self {
        Self {
            body_object_id,
            level: 1,
            experience: 0,
            max_level_time: 0,
            last_delevel_check: 0,
            last_spawn_check: 0,
            active: true,
            evil_mir_oid: None,
            last_evil_mir_hp: 0,
            exps: Vec::new(),
            last_persist_tick: 0,
        }
    }

    /// 缺省经验表常量（default_exps/xp_for_level 回退用）
    const DEFAULT_EXPS: [u64; 11] = [
        10_000, 20_000, 30_000, 40_000, 50_000, 60_000, 70_000, 80_000, 90_000, 100_000, 110_000,
    ];

    /// C# 线性默认经验表（DragonInfo.cs:38-42：Exps[i] = (i+1)*10000）
    pub fn default_exps() -> Vec<u64> {
        Self::DEFAULT_EXPS.to_vec()
    }

    /// #2571：注入 DB 经验表（dragon_info.exps_json；缺省/空值回退线性默认）
    pub fn set_exps_from_db(&mut self, db_exps: &[i64]) {
        let table: Vec<u64> = db_exps
            .iter()
            .copied()
            .filter(|e| *e > 0)
            .map(|e| e as u64)
            .collect();
        if !table.is_empty() {
            self.exps = table;
        }
    }

    /// 升级所需经验表（C# Dragon.GainExp 读 Info.Exps[min(11, Level-1)]）：
    /// level → level+1 所需经验；0 = 已满级不升级
    pub fn xp_for_level(&self, level: u8) -> u64 {
        if level >= 12 {
            return 0;
        }
        let table: &[u64] = if self.exps.is_empty() {
            &Self::DEFAULT_EXPS
        } else {
            &self.exps
        };
        table.get((level - 1) as usize).copied().unwrap_or(0)
    }

    /// 加点经验，返回升级的次数（可能连升多级）。对应 C# Dragon.GainExp。
    pub fn gain_exp(&mut self, amount: u64) -> u32 {
        let mut levelled = 0u32;
        if self.level >= 12 {
            return 0;
        }
        self.experience += amount;
        loop {
            if self.level >= 12 {
                break;
            }
            let needed = self.xp_for_level(self.level);
            if needed == 0 || self.experience < needed {
                break;
            }
            self.experience -= needed;
            self.level += 1;
            levelled += 1;
            if self.level >= 12 {
                self.experience = 0;
                self.max_level_time = chrono::Utc::now().timestamp();
            }
        }
        levelled
    }

    /// 生成龙身 24 个部件的位置偏移（C# BodyLocations）
    pub fn body_part_offsets() -> Vec<(i32, i32)> {
        vec![
            (0, -2),
            (1, -1),
            (2, 0),
            (1, 1),
            (0, 2),
            (-1, 1),
            (-2, 0),
            (-1, -1), // 外圈8个
            (0, -1),
            (1, 0),
            (0, 1),
            (-1, 0), // 内圈4个
            (0, -3),
            (2, -2),
            (3, 0),
            (2, 2),
            (0, 3),
            (-2, 2),
            (-3, 0),
            (-2, -2), // 更外圈8个
            (1, -2),
            (2, -1),
            (2, 1),
            (1, 2),
            (-1, 2),
            (-2, 1),
            (-2, -1),
            (-1, -2), // 交错8个
        ]
        .into_iter()
        .take(24)
        .collect()
    }
}

/// 处理龙降级逻辑（C# Dragon.Process 的降级分支）+ spawn 检查。
///
/// 返回 Some(SpawnEvilMirRequest) 当需要生成新 EvilMir（level 提升且当前无活跃 EvilMir）。
pub async fn tick_dragon_delevel(
    dragon: &mut DragonState,
    tick_count: u64,
    _gate_ref: &ActorRef<GateActor>,
) {
    if !dragon.active {
        return;
    }

    // C# Dragon.Process：满级保持 6*DeLevelDelay（6 小时）后重置 1 级
    if dragon.level >= 12 && dragon.max_level_time != 0 {
        let now = chrono::Utc::now().timestamp();
        // 6 hours = 21600 seconds（C# DeLevelDelay = 60*60*1000 ms = 1 小时；6 * DeLevelDelay = 6 小时）
        if now - dragon.max_level_time >= 21600 {
            dragon.level = 1;
            dragon.experience = 0;
            dragon.max_level_time = 0;
            dragon.last_delevel_check = tick_count;
            tracing::info!("Dragon reset to level 1 after max-level hold");
            return;
        }
    }

    // C# Dragon.Process：level>1 时每 DeLevelDelay（1 小时 = 36000 ticks）降一级
    const DELEVEL_INTERVAL_TICKS: u64 = 36_000;
    if dragon.level > 1 {
        if dragon.last_delevel_check == 0 {
            dragon.last_delevel_check = tick_count;
        } else if tick_count.saturating_sub(dragon.last_delevel_check) >= DELEVEL_INTERVAL_TICKS {
            dragon.level -= 1;
            dragon.experience = 0;
            dragon.last_delevel_check = tick_count;
            tracing::info!("Dragon deleveled to {}", dragon.level);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// #2344：C# DragonInfo.DropInfo.FromLine 解析
    #[test]
    fn test_parse_dragon_drop_line() {
        assert_eq!(
            parse_dragon_drop_line("1/3 Gold 30000 1"),
            Some(DragonDropEntry {
                level: 1,
                chance: 3,
                item_name: None,
                gold: 30000
            })
        );
        assert_eq!(
            parse_dragon_drop_line("1/10 BlackStone 3"),
            Some(DragonDropEntry {
                level: 3,
                chance: 10,
                item_name: Some("BlackStone".to_string()),
                gold: 0
            })
        );
        assert_eq!(parse_dragon_drop_line(";comment"), None);
        assert_eq!(parse_dragon_drop_line(""), None);
        assert_eq!(parse_dragon_drop_line("Gold 30000 1"), None);
        assert_eq!(parse_dragon_drop_line("1/3 Gold 30000"), None);
    }

    #[test]
    fn body_part_offsets_count_24() {
        // C# Dragon.BodyLocations 24 个身体部件偏移
        assert_eq!(super::DragonState::body_part_offsets().len(), 24);
    }

    #[test]
    fn test_gain_exp_level_up() {
        let mut d = DragonState::new(1);
        // #2571：C# 线性默认表——Level 1 -> 2 needs 10000 xp（(1+1)*10000）
        let n = d.gain_exp(10000);
        assert_eq!(n, 1);
        assert_eq!(d.level, 2);
        assert_eq!(d.experience, 0);
    }

    #[test]
    fn test_gain_exp_multi_level() {
        let mut d = DragonState::new(1);
        // 线性默认：1->2 需 10000，2->3 需 20000 → 30000 连升两级
        let n = d.gain_exp(30000);
        assert_eq!(n, 2);
        assert_eq!(d.level, 3);
        assert_eq!(d.experience, 0);
    }

    #[test]
    fn test_gain_exp_partial_progress_kept() {
        let mut d = DragonState::new(1);
        let n = d.gain_exp(15000);
        assert_eq!(n, 1);
        assert_eq!(d.level, 2);
        assert_eq!(
            d.experience, 5000,
            "余量应保留（C# Experience -= Exps[Level-1]）"
        );
    }

    #[test]
    fn test_max_level_delevel_trigger() {
        let mut d = DragonState::new(1);
        d.level = 12;
        d.max_level_time = 0;
        d.gain_exp(0); // no-op at max
        assert_eq!(d.level, 12);
    }

    /// #2571：经验表改读 DB dragon_info.exps——注入表优先生效
    #[test]
    fn test_exps_from_db_override() {
        let mut d = DragonState::new(1);
        // DB 表（仿 C# MirDB 存档值）
        d.set_exps_from_db(&[50_000, 60_000, 0, -1, 70_000]);
        assert_eq!(d.xp_for_level(1), 50_000, "非正值条目应被过滤");
        assert_eq!(d.xp_for_level(2), 60_000);
        assert_eq!(
            d.xp_for_level(3),
            70_000,
            "过滤后的表按序对齐等级（3→4 取第 3 条）"
        );
        assert_eq!(d.xp_for_level(4), 0, "表外条目 = 不升级");
        // 1->2 需 50000
        assert_eq!(d.gain_exp(50_000), 1);
        assert_eq!(d.level, 2);
    }

    /// #2571：DB 经验表空/全非正 → 回退 C# 线性默认 (i+1)*10000
    #[test]
    fn test_exps_empty_falls_back_to_csharp_linear() {
        let mut d = DragonState::new(1);
        d.set_exps_from_db(&[]);
        assert_eq!(d.xp_for_level(1), 10_000);
        d.set_exps_from_db(&[0, -5]);
        assert_eq!(d.xp_for_level(1), 10_000, "全非正值不注入");
        assert_eq!(d.xp_for_level(11), 110_000);
        assert_eq!(d.xp_for_level(12), 0, "满级不升级");
        assert_eq!(DragonState::default_exps().len(), 11);
    }
}
