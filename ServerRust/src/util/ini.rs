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

/// 从解析结果取浮点数（找不到/解析失败返回默认值）
pub fn ini_get_f32(parsed: &HashMap<String, HashMap<String, String>>, section: &str, key: &str, default: f32) -> f32 {
    ini_get(parsed, section, key)
        .and_then(|v| v.trim().parse::<f32>().ok())
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
#[derive(Debug, Clone, Default, PartialEq, Eq)]
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

/// 从 `Configs/ExpList.ini` 加载玩家升级经验曲线（C# Settings.ExperienceList：`[Exp] Level1..LevelN`）
/// 文件缺失/无数据时返回空 Vec（调用方回退 ×1.5）
pub fn load_exp_list(configs_dir: &Path) -> Vec<i64> {
    let path = configs_dir.join("ExpList.ini");
    let Ok(content) = fs::read_to_string(&path) else {
        return Vec::new();
    };
    let parsed = parse_ini(&content);
    let mut out = Vec::new();
    for i in 1..=1000 {
        let v = ini_get_i64(&parsed, "Exp", &format!("Level{}", i), -1);
        if v < 0 {
            break;
        }
        out.push(v);
    }
    out
}

/// 行会配置（C# Settings.LoadGuildSettings：Configs/GuildSettings.ini 覆盖默认值）
#[derive(Debug, Clone)]
pub struct GuildIniSettings {
    /// C# Settings.Guild_RequiredLevel（[Guilds] MinimumLevel）
    pub required_level: u16,
    /// C# Settings.Guild_ExpRate（[Guilds] ExpRate）
    pub exp_rate: f64,
    /// C# Settings.Guild_PointPerLevel（[Guilds] PointPerLevel）
    pub point_per_level: u8,
    /// C# Settings.Guild_WarTime（[Guilds] WarTime）
    pub war_time: i64,
    /// C# Settings.Guild_WarCost（[Guilds] WarCost）
    pub war_cost: u32,
    /// C# Settings.NewbieGuildBuffEnabled（[Guilds] NewbieGuildBuffEnabled）
    pub newbie_guild_buff_enabled: bool,
    /// C# Settings.NewbieGuildExpBuff（[Guilds] NewbieGuildExpBuff）
    pub newbie_guild_exp_buff: i32,
    /// C# Settings.Guild_ExperienceList（[Exp] Level-i）
    pub experience_list: Vec<i64>,
    /// C# Settings.Guild_MembercapList（[Cap] Level-i）
    pub membercap_list: Vec<i32>,
    /// C# Settings.Guild_CreationCostList（[Required-i]：金币/物品混合消耗）
    pub creation_costs: Vec<GuildCreationCost>,
}

impl Default for GuildIniSettings {
    fn default() -> Self {
        Self {
            required_level: 22,
            exp_rate: 0.01,
            point_per_level: 0,
            war_time: 180,
            war_cost: 3000,
            newbie_guild_buff_enabled: true,
            newbie_guild_exp_buff: 5,
            experience_list: Vec::new(),
            membercap_list: Vec::new(),
            creation_costs: Vec::new(),
        }
    }
}

/// 行会创建消耗条目（C# GuildItemVolume：GuildSettings.ini [Required-i]）
#[derive(Debug, Clone)]
pub struct GuildCreationCost {
    /// 物品名（空 = 金币消耗）
    pub item_name: Option<String>,
    /// 金币金额 / 物品数量
    pub amount: u32,
}

/// 从 `Configs/GuildSettings.ini` 加载行会配置（C# Settings.LoadGuildSettings；文件缺失返回 C# 默认）
pub fn load_guild_settings(configs_dir: &Path) -> GuildIniSettings {
    let path = configs_dir.join("GuildSettings.ini");
    let Ok(content) = fs::read_to_string(&path) else {
        return GuildIniSettings::default();
    };
    let parsed = parse_ini(&content);
    let mut s = GuildIniSettings::default();
    s.required_level = ini_get_i64(&parsed, "Guilds", "MinimumLevel", s.required_level as i64).clamp(1, 255) as u16;
    if let Some(v) = ini_get(&parsed, "Guilds", "ExpRate") {
        if let Ok(f) = v.parse::<f64>() {
            s.exp_rate = f;
        }
    }
    s.point_per_level = ini_get_i64(&parsed, "Guilds", "PointPerLevel", s.point_per_level as i64).clamp(0, 255) as u8;
    s.war_time = ini_get_i64(&parsed, "Guilds", "WarTime", s.war_time);
    s.war_cost = ini_get_i64(&parsed, "Guilds", "WarCost", s.war_cost as i64).max(0) as u32;
    if let Some(v) = ini_get(&parsed, "Guilds", "NewbieGuildBuffEnabled") {
        s.newbie_guild_buff_enabled = v == "1" || v.eq_ignore_ascii_case("true");
    }
    s.newbie_guild_exp_buff = ini_get_i64(&parsed, "Guilds", "NewbieGuildExpBuff", s.newbie_guild_exp_buff as i64) as i32;
    for i in 0..1000 {
        let v = ini_get_i64(&parsed, "Exp", &format!("Level-{}", i), -1);
        if v < 0 {
            break;
        }
        s.experience_list.push(v);
    }
    for i in 0..1000 {
        let v = ini_get_i64(&parsed, "Cap", &format!("Level-{}", i), -1);
        if v < 0 {
            break;
        }
        s.membercap_list.push(v as i32);
    }
    // C# LoadGuildSettings：while (Required-i Amount != 0) 读取消耗列表（ItemName 空 = 金币）
    for i in 0..100 {
        let sec = format!("Required-{}", i);
        let amount = ini_get_i64(&parsed, &sec, "Amount", 0);
        if amount <= 0 {
            break;
        }
        let item_name = ini_get(&parsed, &sec, "ItemName").map(|v| v.to_string());
        s.creation_costs.push(GuildCreationCost {
            item_name: if item_name.as_deref().map(|v| v.trim().is_empty()).unwrap_or(true) { None } else { item_name },
            amount: amount as u32,
        });
    }
    s
}

/// 商店/回购配置（C# Settings.LoadGoods：Configs/GoodsSystem.ini [Goods]）
#[derive(Debug, Clone)]
pub struct GoodsIniSettings {
    /// C# Settings.GoodsOn
    pub on: bool,
    /// C# Settings.GoodsMaxStored
    pub max_stored: u32,
    /// C# Settings.GoodsBuyBackTime（分钟）
    pub buy_back_time_minutes: u32,
    /// C# Settings.GoodsBuyBackMaxStored
    pub buy_back_max_stored: u32,
    /// C# Settings.GoodsHideAddedStats
    pub hide_added_stats: bool,
}

impl Default for GoodsIniSettings {
    fn default() -> Self {
        Self {
            on: true,
            max_stored: 15,
            buy_back_time_minutes: 60,
            buy_back_max_stored: 20,
            hide_added_stats: true,
        }
    }
}

/// 从 `Configs/GoodsSystem.ini` 加载商店配置（文件缺失返回 C# 默认）
pub fn load_goods_settings(configs_dir: &Path) -> GoodsIniSettings {
    let path = configs_dir.join("GoodsSystem.ini");
    let Ok(content) = fs::read_to_string(&path) else {
        return GoodsIniSettings::default();
    };
    let parsed = parse_ini(&content);
    let mut s = GoodsIniSettings::default();
    if let Some(v) = ini_get(&parsed, "Goods", "On") {
        s.on = v == "1" || v.eq_ignore_ascii_case("true");
    }
    s.max_stored = ini_get_i64(&parsed, "Goods", "MaxStored", s.max_stored as i64).max(0) as u32;
    s.buy_back_time_minutes = ini_get_i64(&parsed, "Goods", "BuyBackTime", s.buy_back_time_minutes as i64).max(0) as u32;
    s.buy_back_max_stored = ini_get_i64(&parsed, "Goods", "BuyBackMaxStored", s.buy_back_max_stored as i64).max(0) as u32;
    if let Some(v) = ini_get(&parsed, "Goods", "HideAddedStats") {
        s.hide_added_stats = v == "1" || v.eq_ignore_ascii_case("true");
    }
    s
}

/// 邮件配置（C# Settings.LoadMail：Configs/MailSystem.ini）
#[derive(Debug, Clone)]
pub struct MailIniSettings {
    /// C# Settings.MailAutoSendGold（Rust 未接入，仅记录）
    pub auto_send_gold: bool,
    /// C# Settings.MailAutoSendItems（仅记录）
    pub auto_send_items: bool,
    /// C# Settings.MailFreeWithStamp
    pub free_with_stamp: bool,
    /// C# Settings.MailCostPer1KGold
    pub cost_per_1k: u32,
    /// C# Settings.MailItemInsurancePercentage
    pub insurance_percent: u32,
    /// C# Settings.MailCapacity
    pub capacity: u32,
}

impl Default for MailIniSettings {
    fn default() -> Self {
        Self {
            auto_send_gold: false,
            auto_send_items: false,
            free_with_stamp: true,
            cost_per_1k: 100,
            insurance_percent: 5,
            capacity: 100,
        }
    }
}

/// 从 `Configs/MailSystem.ini` 加载邮件配置
pub fn load_mail_settings(configs_dir: &Path) -> MailIniSettings {
    let path = configs_dir.join("MailSystem.ini");
    let Ok(content) = fs::read_to_string(&path) else {
        return MailIniSettings::default();
    };
    let parsed = parse_ini(&content);
    let mut s = MailIniSettings::default();
    if let Some(v) = ini_get(&parsed, "AutoSend", "Gold") {
        s.auto_send_gold = v == "1" || v.eq_ignore_ascii_case("true");
    }
    if let Some(v) = ini_get(&parsed, "AutoSend", "Items") {
        s.auto_send_items = v == "1" || v.eq_ignore_ascii_case("true");
    }
    if let Some(v) = ini_get(&parsed, "Rates", "FreeWithStamp") {
        s.free_with_stamp = v == "1" || v.eq_ignore_ascii_case("true");
    }
    s.cost_per_1k = ini_get_i64(&parsed, "Rates", "CostPer1k", s.cost_per_1k as i64).max(0) as u32;
    s.insurance_percent = ini_get_i64(&parsed, "Rates", "InsurancePerItem", s.insurance_percent as i64).max(0) as u32;
    s.capacity = ini_get_i64(&parsed, "General", "MailCapacity", s.capacity as i64).max(0) as u32;
    s
}

/// 婚姻配置（C# Settings.LoadMarriage：Configs/MarriageSystem.ini [Config]）
#[derive(Debug, Clone)]
pub struct MarriageIniSettings {
    /// C# Settings.LoverEXPBonus
    pub lover_exp_bonus: u32,
    /// C# Settings.MarriageCooldown（天）
    pub cooldown_days: i64,
    /// C# Settings.WeddingRingRecall
    pub wedding_ring_recall: bool,
    /// C# Settings.MarriageLevelRequired
    pub level_required: u16,
    /// C# Settings.ReplaceWedRingCost
    pub replace_wedring_cost: u32,
}

impl Default for MarriageIniSettings {
    fn default() -> Self {
        Self {
            lover_exp_bonus: 5,
            cooldown_days: 7,
            wedding_ring_recall: true,
            level_required: 10,
            replace_wedring_cost: 125,
        }
    }
}

/// 从 `Configs/MarriageSystem.ini` 加载婚姻配置
pub fn load_marriage_settings(configs_dir: &Path) -> MarriageIniSettings {
    let path = configs_dir.join("MarriageSystem.ini");
    let Ok(content) = fs::read_to_string(&path) else {
        return MarriageIniSettings::default();
    };
    let parsed = parse_ini(&content);
    let mut s = MarriageIniSettings::default();
    s.lover_exp_bonus = ini_get_i64(&parsed, "Config", "EXPBonus", s.lover_exp_bonus as i64).max(0) as u32;
    s.cooldown_days = ini_get_i64(&parsed, "Config", "MarriageCooldown", s.cooldown_days).max(0);
    if let Some(v) = ini_get(&parsed, "Config", "AllowLoverRecall") {
        s.wedding_ring_recall = v == "1" || v.eq_ignore_ascii_case("true");
    }
    s.level_required = ini_get_i64(&parsed, "Config", "MinimumLevel", s.level_required as i64).clamp(0, 255) as u16;
    s.replace_wedring_cost = ini_get_i64(&parsed, "Config", "ReplaceRingCost", s.replace_wedring_cost as i64).max(0) as u32;
    s
}

/// 师徒配置（C# Settings.LoadMentor：Configs/MentorSystem.ini [Config]）
#[derive(Debug, Clone)]
pub struct MentorIniSettings {
    /// C# Settings.MentorLevelGap
    pub level_gap: u8,
    /// C# Settings.MentorSkillBoost（Rust 已实现硬编码 true，仅记录）
    pub skill_boost: bool,
    /// C# Settings.MentorLength（天）
    pub length_days: u8,
    /// C# Settings.MentorDamageBoost
    pub damage_boost: u8,
    /// C# Settings.MentorExpBoost
    pub exp_boost: u8,
    /// C# Settings.MenteeExpBank
    pub exp_bank: u8,
}

impl Default for MentorIniSettings {
    fn default() -> Self {
        Self {
            level_gap: 10,
            skill_boost: true,
            length_days: 7,
            damage_boost: 10,
            exp_boost: 10,
            exp_bank: 1,
        }
    }
}

/// 从 `Configs/MentorSystem.ini` 加载师徒配置
pub fn load_mentor_settings(configs_dir: &Path) -> MentorIniSettings {
    let path = configs_dir.join("MentorSystem.ini");
    let Ok(content) = fs::read_to_string(&path) else {
        return MentorIniSettings::default();
    };
    let parsed = parse_ini(&content);
    let mut s = MentorIniSettings::default();
    s.level_gap = ini_get_i64(&parsed, "Config", "LevelGap", s.level_gap as i64).clamp(0, 255) as u8;
    if let Some(v) = ini_get(&parsed, "Config", "MenteeSkillBoost") {
        s.skill_boost = v == "1" || v.eq_ignore_ascii_case("true");
    }
    s.length_days = ini_get_i64(&parsed, "Config", "MentorshipLength", s.length_days as i64).clamp(0, 255) as u8;
    s.damage_boost = ini_get_i64(&parsed, "Config", "MentorDamageBoost", s.damage_boost as i64).clamp(0, 255) as u8;
    s.exp_boost = ini_get_i64(&parsed, "Config", "MenteeExpBoost", s.exp_boost as i64).clamp(0, 255) as u8;
    s.exp_bank = ini_get_i64(&parsed, "Config", "PercentXPtoMentor", s.exp_bank as i64).clamp(0, 255) as u8;
    s
}

/// 元素三表（C# Settings.LoadOrbsExp：Configs/OrbsExpList.ini）
#[derive(Debug, Clone, Default)]
pub struct OrbsIniSettings {
    /// C# Settings.OrbsExpList（[Exp] Orb1..4）
    pub exp_list: Vec<i32>,
    /// C# Settings.OrbsDefList（[Def] Orb1..4）
    pub def_list: Vec<i32>,
    /// C# Settings.OrbsDmgList（[Att] Orb1..4）
    pub dmg_list: Vec<i32>,
}

/// 从 `Configs/OrbsExpList.ini` 加载元素三表（文件缺失返回空；调用方回退默认常量）
pub fn load_orbs_settings(configs_dir: &Path) -> OrbsIniSettings {
    let path = configs_dir.join("OrbsExpList.ini");
    let Ok(content) = fs::read_to_string(&path) else {
        return OrbsIniSettings::default();
    };
    let parsed = parse_ini(&content);
    let mut out = OrbsIniSettings::default();
    for (sec, list) in [("Exp", &mut out.exp_list), ("Def", &mut out.def_list), ("Att", &mut out.dmg_list)] {
        for i in 1..=100 {
            let v = ini_get_i64(&parsed, sec, &format!("Orb{}", i), -1);
            if v < 0 {
                break;
            }
            list.push(v as i32);
        }
    }
    out
}

/// 觉醒配置（C# Settings.LoadAwakeAttribute：Configs/AwakeningSystem.ini；缺失返回 C# 默认）
#[derive(Debug, Clone)]
pub struct AwakeningIniSettings {
    /// C# Awake.AwakeSuccessRate（[Attribute] SuccessRate = 70）
    pub success_rate: u8,
    /// C# Awake.AwakeHitRate（[Attribute] HitRate = 70；Rust 觉醒值生成暂未使用）
    pub hit_rate: u8,
    /// C# Awake.MaxAwakeLevel（[Attribute] MaxUpgradeLevel = 5）
    pub max_awake_level: usize,
    /// C# Awake.Awake_WeaponRate（[IncreaseValue] WeaponValue = 1）
    pub weapon_rate: u8,
    /// C# Awake.Awake_HelmetRate（[IncreaseValue] HelmetValue = 1）
    pub helmet_rate: u8,
    /// C# Awake.Awake_ArmorRate（[IncreaseValue] ArmorValue = 5）
    pub armor_rate: u8,
    /// C# Awake.AwakeChanceMax（[Value] ChanceMax_Common..Heroic；默认 [1,2,3,4,5]）
    pub chance_max: [u8; 5],
    /// C# Awake.AwakeMaterialRate（[Materials_IncreaseValue] Materials_Common..Heroic；默认 [1.0;5]）
    pub material_rate: [f32; 5],
}

impl Default for AwakeningIniSettings {
    fn default() -> Self {
        Self {
            success_rate: 70,
            hit_rate: 70,
            max_awake_level: 5,
            weapon_rate: 1,
            helmet_rate: 1,
            armor_rate: 5,
            chance_max: [1, 2, 3, 4, 5],
            material_rate: [1.0, 1.0, 1.0, 1.0, 1.0],
        }
    }
}

/// 从 `Configs/AwakeningSystem.ini` 加载觉醒配置（文件缺失返回 C# 默认）
pub fn load_awakening_settings(configs_dir: &Path) -> AwakeningIniSettings {
    let path = configs_dir.join("AwakeningSystem.ini");
    let Ok(content) = fs::read_to_string(&path) else {
        return AwakeningIniSettings::default();
    };
    let parsed = parse_ini(&content);
    let mut out = AwakeningIniSettings::default();
    out.success_rate = ini_get_i64(&parsed, "Attribute", "SuccessRate", out.success_rate as i64) as u8;
    out.hit_rate = ini_get_i64(&parsed, "Attribute", "HitRate", out.hit_rate as i64) as u8;
    out.max_awake_level = ini_get_i64(&parsed, "Attribute", "MaxUpgradeLevel", out.max_awake_level as i64).max(1) as usize;
    out.weapon_rate = ini_get_i64(&parsed, "IncreaseValue", "WeaponValue", out.weapon_rate as i64) as u8;
    out.helmet_rate = ini_get_i64(&parsed, "IncreaseValue", "HelmetValue", out.helmet_rate as i64) as u8;
    out.armor_rate = ini_get_i64(&parsed, "IncreaseValue", "ArmorValue", out.armor_rate as i64) as u8;
    const GRADES: [&str; 5] = ["Common", "Rare", "Legendary", "Mythical", "Heroic"];
    for i in 0..5 {
        out.chance_max[i] = ini_get_i64(
            &parsed, "Value", &format!("ChanceMax_{}", GRADES[i]), out.chance_max[i] as i64,
        ).max(1) as u8;
        out.material_rate[i] = ini_get_f32(
            &parsed, "Materials_IncreaseValue", &format!("Materials_{}", GRADES[i]), out.material_rate[i],
        );
    }
    out
}

/// 宝石配置（C# Settings.LoadGem：Configs/GemSystem.ini；缺失返回 C# 默认）
#[derive(Debug, Clone)]
pub struct GemIniSettings {
    /// C# Settings.GemStatIndependent（[Config] GemStatIndependent = true）
    pub stat_independent: bool,
}

impl Default for GemIniSettings {
    fn default() -> Self {
        Self { stat_independent: true }
    }
}

/// 从 `Configs/GemSystem.ini` 加载宝石配置（文件缺失返回 C# 默认）
pub fn load_gem_settings(configs_dir: &Path) -> GemIniSettings {
    let path = configs_dir.join("GemSystem.ini");
    let Ok(content) = fs::read_to_string(&path) else {
        return GemIniSettings::default();
    };
    let parsed = parse_ini(&content);
    let mut out = GemIniSettings::default();
    if let Some(v) = ini_get(&parsed, "Config", "GemStatIndependent") {
        out.stat_independent = v == "1" || v.eq_ignore_ascii_case("true");
    }
    out
}

/// 英雄升级经验曲线（C# Settings.LoadHeroEXP：Configs/HeroExpList.ini [Exp] Level1..500；
/// 默认 100/级，缺失项沿用上一级；文件缺失返回空，调用方回退 100/级）
pub fn load_hero_exp_list(configs_dir: &Path) -> Vec<u32> {
    let path = configs_dir.join("HeroExpList.ini");
    let Ok(content) = fs::read_to_string(&path) else {
        return Vec::new();
    };
    let parsed = parse_ini(&content);
    let mut out = Vec::with_capacity(500);
    let mut exp: i64 = 100;
    for i in 1..=500 {
        exp = ini_get_i64(&parsed, "Exp", &format!("Level{}", i), exp);
        out.push(exp.max(1) as u32);
    }
    out
}

/// 服务端基础参数（C# Settings.Load：Configs/Setup.ini 部分字段；缺失返回 C# 默认）
#[derive(Debug, Clone)]
pub struct SetupIniSettings {
    /// C# Settings.GatherOrbsPerLevel（[Optional] GatherOrbsPerLevel = true）
    pub gather_orbs_per_level: bool,
    /// C# Settings.LineMessageTimer（[Optional] LineMessageTimer = 10 分钟）
    pub line_message_timer_minutes: u32,
    /// C# Settings.ItemSealDelay（[Items] SealDelay = 60 分钟）
    pub item_seal_delay_minutes: u64,
    /// C# Settings.PKDelay（[Game] PKDelay = 12 秒）
    pub pk_delay_seconds: u32,
    /// C# Settings.TeleportToNPCCost（[Game] TeleportToNPCCost = 3000）
    pub teleport_to_npc_cost: i32,
    /// C# Settings.GroupInviteDelay（[Game] GroupInviteDelay = 2000ms）
    pub group_invite_delay_ms: i64,
    /// C# Settings.TradeDelay（[Game] TradeDelay = 2000ms）
    pub trade_delay_ms: i64,
    /// C# Settings.MaxBossTames（[Game] MaxBossTames = 1）
    pub max_boss_tames: u32,
    /// C# Settings.RangeAccuracyBonus（[Bonus] RangeAccuracyBonus = 0）
    pub range_accuracy_bonus: i32,
    /// C# Settings.PKTownMapName（[PKTown] PKTownMapName = "3"）
    pub pk_town_map_name: String,
    /// C# Settings.PKTownPositionX（[PKTown] PKTownPositionX = 848）
    pub pk_town_x: i32,
    /// C# Settings.PKTownPositionY（[PKTown] PKTownPositionY = 677）
    pub pk_town_y: i32,
    /// C# Settings.MaxLuck（[Items] MaxLuck = 10）
    pub max_luck: i32,
}

impl Default for SetupIniSettings {
    fn default() -> Self {
        Self {
            gather_orbs_per_level: true,
            line_message_timer_minutes: 10,
            item_seal_delay_minutes: 60,
            pk_delay_seconds: 12,
            teleport_to_npc_cost: 3000,
            group_invite_delay_ms: 2000,
            trade_delay_ms: 2000,
            max_boss_tames: 1,
            range_accuracy_bonus: 0,
            pk_town_map_name: "3".to_string(),
            pk_town_x: 848,
            pk_town_y: 677,
            max_luck: 10,
        }
    }
}

/// 从 `Configs/Setup.ini` 加载服务端基础参数（文件缺失返回 C# 默认）
pub fn load_setup_settings(configs_dir: &Path) -> SetupIniSettings {
    let path = configs_dir.join("Setup.ini");
    let Ok(content) = fs::read_to_string(&path) else {
        return SetupIniSettings::default();
    };
    let parsed = parse_ini(&content);
    let mut out = SetupIniSettings::default();
    if let Some(v) = ini_get(&parsed, "Optional", "GatherOrbsPerLevel") {
        out.gather_orbs_per_level = v == "1" || v.eq_ignore_ascii_case("true");
    }
    out.line_message_timer_minutes = ini_get_i64(&parsed, "Optional", "LineMessageTimer", out.line_message_timer_minutes as i64).max(1) as u32;
    out.item_seal_delay_minutes = ini_get_i64(&parsed, "Items", "SealDelay", out.item_seal_delay_minutes as i64).max(0) as u64;
    out.pk_delay_seconds = ini_get_i64(&parsed, "Game", "PKDelay", out.pk_delay_seconds as i64).max(1) as u32;
    out.teleport_to_npc_cost = ini_get_i64(&parsed, "Game", "TeleportToNPCCost", out.teleport_to_npc_cost as i64) as i32;
    out.group_invite_delay_ms = ini_get_i64(&parsed, "Game", "GroupInviteDelay", out.group_invite_delay_ms);
    out.trade_delay_ms = ini_get_i64(&parsed, "Game", "TradeDelay", out.trade_delay_ms);
    out.max_boss_tames = ini_get_i64(&parsed, "Game", "MaxBossTames", out.max_boss_tames as i64).max(0) as u32;
    out.range_accuracy_bonus = ini_get_i64(&parsed, "Bonus", "RangeAccuracyBonus", out.range_accuracy_bonus as i64) as i32;
    // #2422：PK 城复活点（C# Settings.PKTown*）
    if let Some(v) = ini_get(&parsed, "PKTown", "PKTownMapName") {
        out.pk_town_map_name = v.to_string();
    }
    out.pk_town_x = ini_get_i64(&parsed, "PKTown", "PKTownPositionX", out.pk_town_x as i64) as i32;
    out.pk_town_y = ini_get_i64(&parsed, "PKTown", "PKTownPositionY", out.pk_town_y as i64) as i32;
    // #2424：幸运上限（C# Settings.MaxLuck，最小 1）
    out.max_luck = ini_get_i64(&parsed, "Items", "MaxLuck", out.max_luck as i64).max(1) as i32;
    out
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
    /// #2404：load_exp_list 解析 `[Exp] LevelN`；文件缺失返回空
    #[test]
    fn test_load_exp_list_parsing() {
        let dir = std::env::temp_dir().join("crystal_ini_test_exp");
        std::fs::create_dir_all(&dir).unwrap();
        std::fs::write(dir.join("ExpList.ini"), "[Exp]\nLevel1=100\nLevel2=200\nLevel3=300\n").unwrap();
        let list = load_exp_list(&dir);
        assert_eq!(list, vec![100, 200, 300]);
        std::fs::remove_dir_all(&dir).ok();

        // 文件缺失 → 空
        assert!(load_exp_list(Path::new("C:/definitely/not/exists")).is_empty());
    }

    /// #2404：真实 Daneo1989/Configs/ExpList.ini 加载（500 级）
    #[test]
    fn test_load_real_exp_list() {
        let path = Path::new(concat!(env!("CARGO_MANIFEST_DIR"), "/Daneo1989/Configs/ExpList.ini"));
        if !path.exists() {
            return; // 数据目录缺失时跳过（CI 无数据版本）
        }
        let dir = path.parent().unwrap();
        let list = load_exp_list(dir);
        assert_eq!(list.len(), 500);
        assert_eq!(list[0], 100);
        assert_eq!(list[499], 100);
    }

    /// #2414：load_orbs_settings 解析 [Exp]/[Def]/[Att] Orb1..N；文件缺失返回空
    #[test]
    fn test_load_orbs_settings_parsing() {
        let dir = std::env::temp_dir().join("crystal_ini_test_orbs");
        std::fs::create_dir_all(&dir).unwrap();
        std::fs::write(dir.join("OrbsExpList.ini"), "[Exp]\nOrb1=50\nOrb2=100\nOrb3=150\nOrb4=200\n\n[Def]\nOrb1=2\nOrb2=4\nOrb3=6\nOrb4=8\n\n[Att]\nOrb1=4\nOrb2=8\nOrb3=12\nOrb4=16\n").unwrap();
        let s = load_orbs_settings(&dir);
        assert_eq!(s.exp_list, vec![50, 100, 150, 200]);
        assert_eq!(s.def_list, vec![2, 4, 6, 8]);
        assert_eq!(s.dmg_list, vec![4, 8, 12, 16]);
        std::fs::remove_dir_all(&dir).ok();

        // 文件缺失 → 空
        assert!(load_orbs_settings(Path::new("C:/definitely/not/exists")).exp_list.is_empty());
    }

    /// #2414：真实 Daneo1989/Configs/OrbsExpList.ini 加载（4 档；数据目录缺失时跳过）
    #[test]
    fn test_load_real_orbs_settings() {
        let path = Path::new(concat!(env!("CARGO_MANIFEST_DIR"), "/Daneo1989/Configs/OrbsExpList.ini"));
        if !path.exists() {
            return; // 数据目录缺失时跳过（CI 无数据版本）
        }
        let dir = path.parent().unwrap();
        let s = load_orbs_settings(dir);
        assert_eq!(s.exp_list, vec![50, 100, 150, 200]);
        assert_eq!(s.def_list, vec![2, 4, 6, 8]);
        assert_eq!(s.dmg_list, vec![4, 8, 12, 16]);
    }

    /// #2416：load_awakening_settings / load_gem_settings 解析 + 真实文件集成
    #[test]
    fn test_load_awakening_and_gem_settings() {
        let dir = std::env::temp_dir().join("crystal_ini_test_awakegem");
        std::fs::create_dir_all(&dir).unwrap();
        std::fs::write(
            dir.join("AwakeningSystem.ini"),
            "[Attribute]\nSuccessRate=70\nHitRate=70\nMaxUpgradeLevel=5\n\n[IncreaseValue]\nWeaponValue=1\nHelmetValue=1\nArmorValue=5\n\n[Value]\nChanceMax_Common=1\nChanceMax_Rare=2\nChanceMax_Legendary=3\nChanceMax_Mythical=4\n\n[Materials_IncreaseValue]\nMaterials_Common=1\nMaterials_Rare=1\nMaterials_Legendary=1\nMaterials_Mythical=1\n",
        ).unwrap();
        std::fs::write(dir.join("GemSystem.ini"), "[Config]\nGemStatIndependent=True\n").unwrap();
        let a = load_awakening_settings(&dir);
        assert_eq!(a.success_rate, 70);
        assert_eq!(a.hit_rate, 70);
        assert_eq!(a.max_awake_level, 5);
        assert_eq!(a.weapon_rate, 1);
        assert_eq!(a.helmet_rate, 1);
        assert_eq!(a.armor_rate, 5);
        // Heroic 未配置 → 保持默认 5（对齐 C# ReadByte 默认值）
        assert_eq!(a.chance_max, [1, 2, 3, 4, 5]);
        assert_eq!(a.material_rate, [1.0, 1.0, 1.0, 1.0, 1.0]);
        let g = load_gem_settings(&dir);
        assert!(g.stat_independent);
        std::fs::remove_dir_all(&dir).ok();

        // 文件缺失 → C# 默认
        let d = load_awakening_settings(Path::new("C:/definitely/not/exists"));
        assert_eq!(d.success_rate, 70);
        assert_eq!(d.chance_max, [1, 2, 3, 4, 5]);
        assert!(load_gem_settings(Path::new("C:/definitely/not/exists")).stat_independent);

        // 真实文件集成
        let real = Path::new(concat!(env!("CARGO_MANIFEST_DIR"), "/Daneo1989/Configs"));
        if real.join("AwakeningSystem.ini").exists() {
            let a2 = load_awakening_settings(real);
            assert_eq!(a2.success_rate, 70);
            assert_eq!(a2.hit_rate, 70);
            assert_eq!(a2.max_awake_level, 5);
            assert_eq!(a2.weapon_rate, 1);
            assert_eq!(a2.helmet_rate, 1);
            assert_eq!(a2.armor_rate, 5);
            assert_eq!(a2.chance_max, [1, 2, 3, 4, 5]);
            assert!(load_gem_settings(real).stat_independent);
        }
    }

    /// #2418：load_hero_exp_list 解析（[Exp] Level1..N，缺失沿用上一级）；文件缺失返回空
    #[test]
    fn test_load_hero_exp_list() {
        let dir = std::env::temp_dir().join("crystal_ini_test_heroexp");
        std::fs::create_dir_all(&dir).unwrap();
        // Level1=100, Level2=200, Level3 缺失沿用 200, Level4=400
        std::fs::write(dir.join("HeroExpList.ini"), "[Exp]\nLevel1=100\nLevel2=200\nLevel4=400\n").unwrap();
        let list = load_hero_exp_list(&dir);
        assert_eq!(list.len(), 500);
        assert_eq!(list[0], 100);
        assert_eq!(list[1], 200);
        assert_eq!(list[2], 200); // Level3 缺失 → 沿用 Level2
        assert_eq!(list[3], 400);
        std::fs::remove_dir_all(&dir).ok();

        // 文件缺失 → 空（调用方回退 100/级）
        assert!(load_hero_exp_list(Path::new("C:/definitely/not/exists")).is_empty());
    }

    /// #2420：load_setup_settings 解析 + 真实 Setup.ini 集成
    #[test]
    fn test_load_setup_settings() {
        let dir = std::env::temp_dir().join("crystal_ini_test_setup");
        std::fs::create_dir_all(&dir).unwrap();
        std::fs::write(
            dir.join("Setup.ini"),
            "[Optional]\nGatherOrbsPerLevel=True\nLineMessageTimer=10\n\n[Items]\nSealDelay=60\nMaxLuck=10\n\n[Game]\nPKDelay=12\nTeleportToNPCCost=3000\nGroupInviteDelay=2000\nTradeDelay=2000\nMaxBossTames=1\n\n[Bonus]\nRangeAccuracyBonus=0\n\n[PKTown]\nPKTownMapName=3\nPKTownPositionX=848\nPKTownPositionY=677\n",
        ).unwrap();
        let s = load_setup_settings(&dir);
        assert!(s.gather_orbs_per_level);
        assert_eq!(s.line_message_timer_minutes, 10);
        assert_eq!(s.item_seal_delay_minutes, 60);
        assert_eq!(s.pk_delay_seconds, 12);
        assert_eq!(s.teleport_to_npc_cost, 3000);
        assert_eq!(s.group_invite_delay_ms, 2000);
        assert_eq!(s.trade_delay_ms, 2000);
        assert_eq!(s.max_boss_tames, 1);
        assert_eq!(s.range_accuracy_bonus, 0);
        // #2422：PK 城复活点
        assert_eq!(s.pk_town_map_name, "3");
        assert_eq!(s.pk_town_x, 848);
        assert_eq!(s.pk_town_y, 677);
        // #2424：幸运上限
        assert_eq!(s.max_luck, 10);
        std::fs::remove_dir_all(&dir).ok();

        // 文件缺失 → C# 默认
        let d = load_setup_settings(Path::new("C:/definitely/not/exists"));
        assert!(d.gather_orbs_per_level);
        assert_eq!(d.line_message_timer_minutes, 10);
        assert_eq!(d.teleport_to_npc_cost, 3000);
        assert_eq!(d.pk_town_map_name, "3");
        assert_eq!(d.pk_town_x, 848);
        assert_eq!(d.pk_town_y, 677);
        assert_eq!(d.max_luck, 10);

        // 真实文件集成（Daneo1989/Configs/Setup.ini）
        let real = Path::new(concat!(env!("CARGO_MANIFEST_DIR"), "/Daneo1989/Configs"));
        if real.join("Setup.ini").exists() {
            let r = load_setup_settings(real);
            assert!(r.gather_orbs_per_level);
            assert_eq!(r.line_message_timer_minutes, 10);
            assert_eq!(r.pk_delay_seconds, 12);
            assert_eq!(r.group_invite_delay_ms, 2000);
            assert_eq!(r.trade_delay_ms, 2000);
            assert_eq!(r.range_accuracy_bonus, 0);
            // #2422：PK 城复活点
            assert_eq!(r.pk_town_map_name, "3");
            assert_eq!(r.pk_town_x, 848);
            assert_eq!(r.pk_town_y, 677);
            // #2424：幸运上限
            assert_eq!(r.max_luck, 10);
        }
    }

    /// #2406：load_guild_settings 解析（[Guilds]/经验与上限列表）；文件缺失返回 C# 默认
    #[test]
    fn test_load_guild_settings_parsing() {
        let dir = std::env::temp_dir().join("crystal_ini_test_guild");
        std::fs::create_dir_all(&dir).unwrap();
        std::fs::write(
            dir.join("GuildSettings.ini"),
            "[Guilds]\nMinimumLevel=22\nExpRate=0.01\nPointPerLevel=1\nWarTime=180\nWarCost=30000\nNewbieGuildBuffEnabled=true\nNewbieGuildExpBuff=5\n\n[Exp]\nLevel-0=1\nLevel-1=1\nLevel-2=2\n\n[Cap]\nLevel-0=10\nLevel-1=20\n\n[Required-0]\nItemName=\nAmount=1000000\n\n[Required-1]\nItemName=WoomaHorn\nAmount=1\n",
        )
        .unwrap();
        let s = load_guild_settings(&dir);
        assert_eq!(s.required_level, 22);
        assert_eq!(s.exp_rate, 0.01);
        assert_eq!(s.point_per_level, 1);
        assert_eq!(s.war_cost, 30000);
        assert_eq!(s.war_time, 180);
        assert_eq!(s.experience_list, vec![1, 1, 2]);
        assert_eq!(s.membercap_list, vec![10, 20]);
        // #2412：建会混合消耗（金币 + 物品）
        assert_eq!(s.creation_costs.len(), 2);
        assert!(s.creation_costs[0].item_name.is_none());
        assert_eq!(s.creation_costs[0].amount, 1_000_000);
        assert_eq!(s.creation_costs[1].item_name.as_deref(), Some("WoomaHorn"));
        assert_eq!(s.creation_costs[1].amount, 1);
        std::fs::remove_dir_all(&dir).ok();

        // 文件缺失 → C# 默认
        let d = load_guild_settings(Path::new("C:/definitely/not/exists"));
        assert_eq!(d.required_level, 22);
        assert_eq!(d.war_cost, 3000);
        assert!(d.experience_list.is_empty());
    }

    /// #2406：真实 Daneo1989/Configs/GuildSettings.ini 加载（WarCost=30000/PointPerLevel=1/Exp 21 项/Cap 21 项）
    #[test]
    fn test_load_real_guild_settings() {
        let path = Path::new(concat!(env!("CARGO_MANIFEST_DIR"), "/Daneo1989/Configs/GuildSettings.ini"));
        if !path.exists() {
            return; // 数据目录缺失时跳过
        }
        let dir = path.parent().unwrap();
        let s = load_guild_settings(dir);
        assert_eq!(s.required_level, 22);
        assert_eq!(s.exp_rate, 0.01);
        assert_eq!(s.point_per_level, 1);
        assert_eq!(s.war_time, 180);
        assert_eq!(s.war_cost, 30000);
        assert_eq!(s.experience_list.len(), 21);
        assert_eq!(s.membercap_list.len(), 21);
        // #2412：建会消耗（金币 1M + WoomaHorn×1）
        assert_eq!(s.creation_costs.len(), 2);
        assert!(s.creation_costs[0].item_name.is_none());
        assert_eq!(s.creation_costs[0].amount, 1_000_000);
        assert_eq!(s.creation_costs[1].item_name.as_deref(), Some("WoomaHorn"));
        assert_eq!(s.creation_costs[1].amount, 1);
    }

    /// #2408：Goods/Mail/Marriage/Mentor 四组 ini 解析 + 真实文件集成
    #[test]
    fn test_load_system_settings() {
        // 解析单测（临时文件）
        let dir = std::env::temp_dir().join("crystal_ini_test_sys");
        std::fs::create_dir_all(&dir).unwrap();
        std::fs::write(dir.join("GoodsSystem.ini"), "[Goods]\nOn=True\nMaxStored=50\nBuyBackTime=60\nBuyBackMaxStored=20\nHideAddedStats=True\n").unwrap();
        std::fs::write(dir.join("MailSystem.ini"), "[AutoSend]\nGold=False\nItems=False\n\n[Rates]\nFreeWithStamp=True\nCostPer1k=100\nInsurancePerItem=5\n\n[General]\nMailCapacity=100\n").unwrap();
        std::fs::write(dir.join("MarriageSystem.ini"), "[Config]\nEXPBonus=5\nMarriageCooldown=7\nAllowLoverRecall=True\nMinimumLevel=10\nReplaceRingCost=125\n").unwrap();
        std::fs::write(dir.join("MentorSystem.ini"), "[Config]\nLevelGap=10\nMenteeSkillBoost=True\nMentorshipLength=7\nMentorDamageBoost=10\nMenteeExpBoost=10\nPercentXPtoMentor=1\n").unwrap();
        let g = load_goods_settings(&dir);
        assert_eq!(g.max_stored, 50);
        assert!(g.on);
        let m = load_mail_settings(&dir);
        assert_eq!(m.capacity, 100);
        assert_eq!(m.cost_per_1k, 100);
        let mg = load_marriage_settings(&dir);
        assert_eq!(mg.cooldown_days, 7);
        assert_eq!(mg.replace_wedring_cost, 125);
        let mt = load_mentor_settings(&dir);
        assert_eq!(mt.length_days, 7);
        assert_eq!(mt.exp_bank, 1);
        std::fs::remove_dir_all(&dir).ok();

        // 文件缺失 → C# 默认
        assert_eq!(load_goods_settings(Path::new("C:/definitely/not/exists")).max_stored, 15);
        assert_eq!(load_mail_settings(Path::new("C:/definitely/not/exists")).capacity, 100);
        assert_eq!(load_marriage_settings(Path::new("C:/definitely/not/exists")).cooldown_days, 7);
        assert_eq!(load_mentor_settings(Path::new("C:/definitely/not/exists")).length_days, 7);

        // 真实文件集成
        let real = Path::new(concat!(env!("CARGO_MANIFEST_DIR"), "/Daneo1989/Configs"));
        if real.join("GoodsSystem.ini").exists() {
            assert_eq!(load_goods_settings(real).max_stored, 50);
            assert_eq!(load_marriage_settings(real).cooldown_days, 7);
            assert_eq!(load_mentor_settings(real).length_days, 7);
            assert_eq!(load_mail_settings(real).capacity, 100);
        }
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
