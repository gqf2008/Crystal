//! 轻量 INI 解析器 —— 等价 C# `Settings` 读取 `Configs/*.ini`（Daneo1989 数据源）
//!
//! C# 服务端启动时从 `EnvirPath/Configs/*.ini` 读取行会/钓鱼/经验等配置
//! （`Settings.LoadGuildSettings` / `Envir.cs` 等）。本模块提供同等数据源：
//! - `FishingSystem.ini` → [`FishingConfig`]
//! - `GuildSettings.ini` 的 `[Buff-*]` → [`GuildBuffInfo`]
//! - 通用 `parse_ini` 供后续 `Mines.ini` / `OrbsExpList.ini` 等复用

use std::collections::HashMap;
use std::fs;
use std::path::Path;

/// 解析 INI 文本：section -> (key -> value)
/// - `[Section]` 行开始新 section；`;`/`#` 开头视为注释；空行跳过
/// - `key=value` 拆成 (key, value)，key 统一小写
pub fn parse_ini(content: &str) -> HashMap<String, HashMap<String, String>> {
    let mut result: HashMap<String, HashMap<String, String>> = HashMap::new();
    let mut current: String = String::new();
    for raw in content.lines() {
        let line = raw.trim();
        if line.is_empty() || line.starts_with(';') || line.starts_with('#') {
            continue;
        }
        if line.starts_with('[') && line.ends_with(']') {
            current = line[1..line.len() - 1].trim().to_lowercase();
            result.entry(current.clone()).or_default();
            continue;
        }
        if let Some(eq) = line.find('=') {
            let key = line[..eq].trim().to_lowercase();
            let value = line[eq + 1..].trim().to_string();
            result.entry(current.clone()).or_default().insert(key, value);
        }
    }
    result
}

/// 从解析结果取字符串（找不到返回 None）
pub fn ini_get<'a>(
    parsed: &'a HashMap<String, HashMap<String, String>>,
    section: &str,
    key: &str,
) -> Option<&'a str> {
    parsed
        .get(&section.to_lowercase())?
        .get(&key.to_lowercase())
        .map(|s| s.as_str())
}

/// 从解析结果取整数（找不到/解析失败返回默认值）
pub fn ini_get_i64(parsed: &HashMap<String, HashMap<String, String>>, section: &str, key: &str, default: i64) -> i64 {
    ini_get(parsed, section, key)
        .and_then(|v| v.trim().parse::<i64>().ok())
        .unwrap_or(default)
}

/// 钓鱼系统配置（C# `Settings.Fishing*`：`Configs/FishingSystem.ini`）
#[derive(Debug, Clone)]
pub struct FishingConfig {
    /// C# Settings.FishingAttempts = 30（FishingProgressMax）
    pub attempts: u32,
    /// C# Settings.FishingSuccessStart = 10
    pub success_start: i32,
    /// C# Settings.FishingSuccessMultiplier = 10（连续成功次数 × 加成）
    pub success_multiplier: i32,
    /// C# Settings.FishingDelay = 0（毫秒，UpdateFish 间隔）
    pub delay_ms: u32,
    /// C# Settings.FishingMobSpawnChance = 5（百分比，收获时刷怪概率）
    pub monster_spawn_chance: u32,
    /// C# Settings.FishingMonster = GiantKeratoid（收获时刷的怪）
    pub monster: String,
}

impl Default for FishingConfig {
    fn default() -> Self {
        Self {
            attempts: 30,
            success_start: 10,
            success_multiplier: 10,
            delay_ms: 0,
            monster_spawn_chance: 5,
            monster: "GiantKeratoid".to_string(),
        }
    }
}

/// 行会 Buff 定义（C# `GuildBuffInfo`：`Configs/GuildSettings.ini` `[Buff-*]`）
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GuildBuffInfo {
    /// C# GuildBuffInfo.Id（客户端用 Id 标识，与数组下标无关）
    pub id: u32,
    /// C# GuildBuffInfo.Icon
    pub icon: u32,
    /// C# GuildBuffInfo.Name
    pub name: String,
    /// C# GuildBuffInfo.LevelRequirement
    pub level_req: u32,
    /// C# GuildBuffInfo.PointsRequirement
    pub points_req: u32,
    /// C# GuildBuffInfo.TimeLimit（分钟）
    pub time_limit_minutes: u32,
    /// C# GuildBuffInfo.ActivationCost（金币）
    pub activation_cost: u64,
    pub buff_ac: i32,
    pub buff_mac: i32,
    pub buff_dc: i32,
    pub buff_mc: i32,
    pub buff_sc: i32,
    pub buff_max_hp: i32,
    pub buff_max_mp: i32,
    /// C# BuffMineRate
    pub buff_mine_rate: i32,
    /// C# BuffGemRate
    pub buff_gem_rate: i32,
    /// C# BuffFishRate（钓鱼成功率加成 %）
    pub buff_fish_rate: i32,
    /// C# BuffExpRate（经验加成 %）
    pub buff_exp_rate: i32,
    /// C# BuffCraftRate
    pub buff_craft_rate: i32,
    /// C# BuffSkillRate
    pub buff_skill_rate: i32,
    /// C# BuffHpRegen
    pub buff_hp_regen: i32,
    /// C# BuffMpRegen
    pub buff_mp_regen: i32,
    /// C# BuffAttack
    pub buff_attack: i32,
    /// C# BuffDropRate
    pub buff_drop_rate: i32,
    /// C# BuffGoldRate
    pub buff_gold_rate: i32,
}

/// 从 `Configs/FishingSystem.ini` 加载钓鱼配置；文件缺失时返回默认值
pub fn load_fishing_config(configs_dir: &Path) -> FishingConfig {
    let path = configs_dir.join("FishingSystem.ini");
    let Ok(content) = fs::read_to_string(&path) else {
        return FishingConfig::default();
    };
    let parsed = parse_ini(&content);
    let mut cfg = FishingConfig::default();
    cfg.attempts = ini_get_i64(&parsed, "Rates", "Attempts", cfg.attempts as i64).max(1) as u32;
    cfg.success_start = ini_get_i64(&parsed, "Rates", "SuccessStart", cfg.success_start as i64) as i32;
    cfg.success_multiplier = ini_get_i64(&parsed, "Rates", "SuccessMultiplier", cfg.success_multiplier as i64) as i32;
    cfg.delay_ms = ini_get_i64(&parsed, "Rates", "Delay", cfg.delay_ms as i64).max(0) as u32;
    cfg.monster_spawn_chance =
        ini_get_i64(&parsed, "Rates", "MonsterSpawnChance", cfg.monster_spawn_chance as i64).clamp(0, 100) as u32;
    if let Some(m) = ini_get(&parsed, "Game", "Monster") {
        if !m.is_empty() {
            cfg.monster = m.to_string();
        }
    }
    cfg
}

fn get_i32(parsed: &HashMap<String, HashMap<String, String>>, section: &str, key: &str, default: i32) -> i32 {
    ini_get_i64(parsed, section, key, default as i64) as i32
}

/// 从 `Configs/GuildSettings.ini` 加载行会 Buff 定义（`[Buff-0]`..`[Buff-15]`，TotalBuffs=16）
pub fn load_guild_buff_infos(configs_dir: &Path) -> Vec<GuildBuffInfo> {
    let path = configs_dir.join("GuildSettings.ini");
    let Ok(content) = fs::read_to_string(&path) else {
        return Vec::new();
    };
    let parsed = parse_ini(&content);
    let total = ini_get_i64(&parsed, "Guilds", "TotalBuffs", 0).max(0) as usize;
    let mut out = Vec::with_capacity(total.min(64));
    for i in 0..total.min(64) {
        let section = format!("Buff-{}", i);
        let Some(id) = ini_get(&parsed, &section, "Id").and_then(|v| v.parse::<u32>().ok()) else {
            continue;
        };
        out.push(GuildBuffInfo {
            id,
            icon: ini_get_i64(&parsed, &section, "Icon", 0).max(0) as u32,
            name: ini_get(&parsed, &section, "Name").unwrap_or("").to_string(),
            level_req: ini_get_i64(&parsed, &section, "LevelReq", 0).max(0) as u32,
            points_req: ini_get_i64(&parsed, &section, "PointsReq", 0).max(0) as u32,
            time_limit_minutes: ini_get_i64(&parsed, &section, "TimeLimit", 0).max(0) as u32,
            activation_cost: ini_get_i64(&parsed, &section, "ActivationCost", 0).max(0) as u64,
            buff_ac: get_i32(&parsed, &section, "BuffAc", 0),
            buff_mac: get_i32(&parsed, &section, "BuffMAC", 0),
            buff_dc: get_i32(&parsed, &section, "BuffDc", 0),
            buff_mc: get_i32(&parsed, &section, "BuffMc", 0),
            buff_sc: get_i32(&parsed, &section, "BuffSc", 0),
            buff_max_hp: get_i32(&parsed, &section, "BuffMaxHp", 0),
            buff_max_mp: get_i32(&parsed, &section, "BuffMaxMp", 0),
            buff_mine_rate: get_i32(&parsed, &section, "BuffMineRate", 0),
            buff_gem_rate: get_i32(&parsed, &section, "BuffGemRate", 0),
            buff_fish_rate: get_i32(&parsed, &section, "BuffFishRate", 0),
            buff_exp_rate: get_i32(&parsed, &section, "BuffExpRate", 0),
            buff_craft_rate: get_i32(&parsed, &section, "BuffCraftRate", 0),
            buff_skill_rate: get_i32(&parsed, &section, "BuffSkillRate", 0),
            buff_hp_regen: get_i32(&parsed, &section, "BuffHpRegen", 0),
            buff_mp_regen: get_i32(&parsed, &section, "BuffMpRegen", 0),
            buff_attack: get_i32(&parsed, &section, "BuffAttack", 0),
            buff_drop_rate: get_i32(&parsed, &section, "BuffDropRate", 0),
            buff_gold_rate: get_i32(&parsed, &section, "BuffGoldRate", 0),
        });
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_ini_basic() {
        let content = "\
; comment
[Guilds]
MinimumLevel=22
ExpRate=0.01
TotalBuffs=16

[Buff-0]
Id=1
Icon=24
Name=Reputation
BuffExpRate=0
";
        let parsed = parse_ini(content);
        assert_eq!(ini_get(&parsed, "Guilds", "MinimumLevel"), Some("22"));
        assert_eq!(ini_get(&parsed, "guilds", "TotalBuffs"), Some("16"));
        assert_eq!(ini_get(&parsed, "Buff-0", "Name"), Some("Reputation"));
        assert_eq!(ini_get(&parsed, "Buff-0", "BuffExpRate"), Some("0"));
        assert_eq!(ini_get(&parsed, "Missing", "x"), None);
    }

    #[test]
    fn test_fishing_config_default_on_missing() {
        let cfg = load_fishing_config(Path::new("C:/definitely/not/exists"));
        assert_eq!(cfg.attempts, 30);
        assert_eq!(cfg.success_start, 10);
        assert_eq!(cfg.success_multiplier, 10);
        assert_eq!(cfg.monster, "GiantKeratoid");
    }

    #[test]
    fn test_fishing_config_parsing() {
        let dir = std::env::temp_dir().join("crystal_ini_test_fish");
        std::fs::create_dir_all(&dir).unwrap();
        std::fs::write(
            dir.join("FishingSystem.ini"),
            "[Rates]\nAttempts=30\nSuccessStart=10\nSuccessMultiplier=10\nDelay=0\nMonsterSpawnChance=5\n\n[Game]\nMonster=GiantKeratoid\n",
        )
        .unwrap();
        let cfg = load_fishing_config(&dir);
        assert_eq!(cfg.attempts, 30);
        assert_eq!(cfg.success_start, 10);
        assert_eq!(cfg.monster_spawn_chance, 5);
        assert_eq!(cfg.monster, "GiantKeratoid");
        std::fs::remove_dir_all(&dir).ok();
    }

    #[test]
    fn test_guild_buff_infos_parsing() {
        let dir = std::env::temp_dir().join("crystal_ini_test_buff");
        std::fs::create_dir_all(&dir).unwrap();
        std::fs::write(
            dir.join("GuildSettings.ini"),
            "[Guilds]\nTotalBuffs=2\n\n[Buff-0]\nId=1\nIcon=24\nName=Reputation\nLevelReq=0\nPointsReq=1\nTimeLimit=60\nActivationCost=0\nBuffAc=0\nBuffMAC=0\nBuffDc=0\nBuffMc=0\nBuffSc=0\nBuffMaxHp=0\nBuffMaxMp=0\nBuffMineRate=0\nBuffGemRate=0\nBuffFishRate=0\nBuffExpRate=0\nBuffCraftRate=0\nBuffSkillRate=0\nBuffHpRegen=0\nBuffMpRegen=0\nBuffAttack=0\nBuffDropRate=0\nBuffGoldRate=0\n\n[Buff-1]\nId=2\nIcon=27\nName=Specialized Crafter\nLevelReq=0\nPointsReq=1\nTimeLimit=60\nActivationCost=0\nBuffAc=0\nBuffMAC=0\nBuffDc=0\nBuffMc=0\nBuffSc=0\nBuffMaxHp=0\nBuffMaxMp=0\nBuffMineRate=0\nBuffGemRate=0\nBuffFishRate=0\nBuffExpRate=0\nBuffCraftRate=1\nBuffSkillRate=0\nBuffHpRegen=0\nBuffMpRegen=0\nBuffAttack=0\nBuffDropRate=0\nBuffGoldRate=0\n",
        )
        .unwrap();
        let infos = load_guild_buff_infos(&dir);
        assert_eq!(infos.len(), 2);
        assert_eq!(infos[0].id, 1);
        assert_eq!(infos[0].name, "Reputation");
        assert_eq!(infos[1].id, 2);
        assert_eq!(infos[1].buff_craft_rate, 1);
        std::fs::remove_dir_all(&dir).ok();
    }
}

/// #1749：C# Settings.RandomItemStatsList（Configs/RandomItemStats.ini）——掉落随机附加属性配置
/// 解析 Item0..ItemN 节，字段名与 C# 一致（ini key 统一小写）
pub fn load_random_item_stats(configs_dir: &Path) -> Vec<mir2_shared::data::item::RandomItemStat> {
    let path = configs_dir.join("RandomItemStats.ini");
    let content = match fs::read_to_string(&path) {
        Ok(c) => c,
        Err(_) => return Vec::new(),
    };
    let parsed = parse_ini(&content);
    let mut out = Vec::new();
    for i in 0..256 {
        let section = format!("item{}", i);
        if !parsed.contains_key(&section) {
            break;
        }
        let mut s = mir2_shared::data::item::RandomItemStat::default();
        macro_rules! rd {
            ($key:literal, $field:ident) => {
                s.$field = ini_get_i64(&parsed, &section, $key, 0).clamp(0, 255) as u8;
            };
        }
        rd!("maxdurachanCe", max_dura_chance);
        rd!("maxdurastatchance", max_dura_stat_chance);
        rd!("maxduramaxstat", max_dura_max_stat);
        rd!("maxacchance", max_ac_chance);
        rd!("maxacstatchance", max_ac_stat_chance);
        rd!("maxacmaxstat", max_ac_max_stat);
        rd!("maxmacchance", max_mac_chance);
        rd!("maxmacstatchance", max_mac_stat_chance);
        rd!("maxmacmaxstat", max_mac_max_stat);
        rd!("maxdcchance", max_dc_chance);
        rd!("maxdcstatchance", max_dc_stat_chance);
        rd!("maxdcmaxstat", max_dc_max_stat);
        rd!("maxmcchance", max_mc_chance);
        rd!("maxmcstatchance", max_mc_stat_chance);
        rd!("maxmcmaxstat", max_mc_max_stat);
        rd!("maxscchance", max_sc_chance);
        rd!("maxscstatchance", max_sc_stat_chance);
        rd!("maxscmaxstat", max_sc_max_stat);
        rd!("accuracychance", accuracy_chance);
        rd!("accuracystatchance", accuracy_stat_chance);
        rd!("accuracymaxstat", accuracy_max_stat);
        rd!("agilitychance", agility_chance);
        rd!("agilitystatchance", agility_stat_chance);
        rd!("agilitymaxstat", agility_max_stat);
        rd!("hpchance", hp_chance);
        rd!("hpstatchance", hp_stat_chance);
        rd!("hpmaxstat", hp_max_stat);
        rd!("mpchance", mp_chance);
        rd!("mpstatchance", mp_stat_chance);
        rd!("mpmaxstat", mp_max_stat);
        rd!("strongchance", strong_chance);
        rd!("strongstatchance", strong_stat_chance);
        rd!("strongmaxstat", strong_max_stat);
        rd!("magicresistchance", magic_resist_chance);
        rd!("magicresiststatchance", magic_resist_stat_chance);
        rd!("magicresistmaxstat", magic_resist_max_stat);
        rd!("poisonresistchance", poison_resist_chance);
        rd!("poisonresiststatchance", poison_resist_stat_chance);
        rd!("poisonresistmaxstat", poison_resist_max_stat);
        rd!("hprecovchance", hp_recovery_chance);
        rd!("hprecovstatchance", hp_recovery_stat_chance);
        rd!("hprecovmaxstat", hp_recovery_max_stat);
        rd!("mprecovchance", mp_recovery_chance);
        rd!("mprecovstatchance", mp_recovery_stat_chance);
        rd!("mprecovmaxstat", mp_recovery_max_stat);
        rd!("poisonrecovchance", poison_recovery_chance);
        rd!("poisonrecovstatchance", poison_recovery_stat_chance);
        rd!("poisonrecovmaxstat", poison_recovery_max_stat);
        rd!("criticalratechance", critical_rate_chance);
        rd!("criticalratestatchance", critical_rate_stat_chance);
        rd!("criticalratemaxstat", critical_rate_max_stat);
        rd!("criticaldamagechance", critical_damage_chance);
        rd!("criticaldamagestatchance", critical_damage_stat_chance);
        rd!("criticaldamagemaxstat", critical_damage_max_stat);
        rd!("freezechance", freeze_chance);
        rd!("freezestatchance", freeze_stat_chance);
        rd!("freezemaxstat", freeze_max_stat);
        rd!("poisonattackchance", poison_attack_chance);
        rd!("poisonattackstatchance", poison_attack_stat_chance);
        rd!("poisonattackmaxstat", poison_attack_max_stat);
        rd!("attackspeedchance", attack_speed_chance);
        rd!("attackspeedstatchance", attack_speed_stat_chance);
        rd!("attackspeedmaxstat", attack_speed_max_stat);
        rd!("luckchance", luck_chance);
        rd!("luckstatchance", luck_stat_chance);
        rd!("luckmaxstat", luck_max_stat);
        rd!("cursechance", curse_chance);
        rd!("slotchance", slot_chance);
        rd!("slotstatchance", slot_stat_chance);
        rd!("slotmaxstat", slot_max_stat);
        out.push(s);
    }
    out
}

