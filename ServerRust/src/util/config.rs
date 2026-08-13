use serde::Deserialize;

#[derive(Debug, Deserialize, Clone)]
pub struct ServerConfig {
    pub network: NetworkConfig,
    pub database: DatabaseConfig,
    pub server: ServerWorldConfig,
    /// Social / 行会 / 任务系统配置 (PR 收尾工作:把散落的硬编码常量集中到 cfg)
    #[serde(default)]
    pub social: SocialConfig,
    /// 攻城 / 行会领地（GT）配置（对齐 C# Settings.BuyGTGold/ExtendGT 等）
    #[serde(default)]
    pub conquest: ConquestConfig,
    /// 休息经验加成配置（C# Settings.Rested*）
    #[serde(default)]
    pub rested: RestedConfig,
    /// PvP 开关配置（C# Settings.PvpCan*）
    #[serde(default)]
    pub pvp: PvpConfig,
    /// 精炼配置（C# Settings.Refine*）
    #[serde(default)]
    pub refine: RefineConfig,
}

#[derive(Debug, Deserialize, Clone)]
pub struct ConquestConfig {
    /// 购买领地所需行会金币（C# Settings.BuyGTGold）
    #[serde(default = "default_conquest_buy_gold")]
    pub buy_gold: u64,
    /// 延长领地租期费用（C# Settings.ExtendGT）
    #[serde(default = "default_conquest_extend_gold")]
    pub extend_gold: u64,
    /// 领地挂售最低价格（C# GTSale 最低 200 万）
    #[serde(default = "default_conquest_gt_sale_min")]
    pub gt_sale_min_price: u64,
    /// 领地租期天数（C# Settings.GTDays = 30；BuyGT 初始 + ExtendGT 延长）
    #[serde(default = "default_conquest_gt_days")]
    pub gt_days: u32,
}

fn default_conquest_buy_gold() -> u64 {
    10_000_000 // C# Settings.BuyGTGold
}

fn default_conquest_extend_gold() -> u64 {
    1_000_000 // C# Settings.ExtendGT
}

fn default_conquest_gt_sale_min() -> u64 {
    2_000_000
}

fn default_conquest_gt_days() -> u32 {
    30
}

impl Default for ConquestConfig {
    fn default() -> Self {
        Self {
            buy_gold: default_conquest_buy_gold(),
            extend_gold: default_conquest_extend_gold(),
            gt_sale_min_price: default_conquest_gt_sale_min(),
            gt_days: default_conquest_gt_days(),
        }
    }
}

/// PvP 开关（C# Settings.cs：PvpCanFreeze/PvpCanResistPoison，默认 false）
#[derive(Debug, Deserialize, Clone)]
pub struct PvpConfig {
    /// PvP 中是否允许冰冻（C# PvpCanFreeze；CatTongue 玩家冰冻门控）
    #[serde(default)]
    pub can_freeze: bool,
    /// PvP 中是否允许毒抗/眩晕（C# PvpCanResistPoison；TwinDrakeBlade 玩家眩晕门控）
    #[serde(default)]
    pub can_resist_poison: bool,
}

impl Default for PvpConfig {
    fn default() -> Self {
        Self {
            can_freeze: false,
            can_resist_poison: false,
        }
    }
}

/// 精炼配置（C# Settings.Refine*：Settings.cs:250-260）
#[derive(Debug, Deserialize, Clone)]
pub struct RefineConfig {
    /// 精炼基础成功率（C# RefineBaseChance = 20）
    #[serde(default = "default_refine_base_chance")]
    pub base_chance: u8,
    /// 精炼完成时间（分钟，C# RefineTime = 20）
    #[serde(default = "default_refine_time_minutes")]
    pub time_minutes: u32,
    /// 精炼加成值（C# RefineIncrease = 1）
    #[serde(default = "default_refine_increase")]
    pub increase: u8,
    /// 抱击概率（C# RefineCritChance = 10）
    #[serde(default = "default_refine_crit_chance")]
    pub crit_chance: u8,
    /// 抱击加成倍率（C# RefineCritIncrease = 2）
    #[serde(default = "default_refine_crit_increase")]
    pub crit_increase: u8,
    /// 武器已加属性折扣（C# RefineWepStatReduce = 6）
    #[serde(default = "default_refine_wep_stat_reduce")]
    pub wep_stat_reduce: u8,
    /// 非武器已加属性折扣（C# RefineItemStatReduce = 15）
    #[serde(default = "default_refine_item_stat_reduce")]
    pub item_stat_reduce: u8,
    /// 精炼费用系数（C# RefineCost = 125；cost = RequiredAmount*10*RefineCost）
    #[serde(default = "default_refine_cost")]
    pub cost: u32,
    /// 精炼矿石名（C# RefineOreName = "BlackIronOre"）
    #[serde(default = "default_refine_ore_name")]
    pub ore_name: String,
}

fn default_refine_base_chance() -> u8 {
    20
}

fn default_refine_time_minutes() -> u32 {
    20
}

fn default_refine_increase() -> u8 {
    1
}

fn default_refine_crit_chance() -> u8 {
    10
}

fn default_refine_crit_increase() -> u8 {
    2
}

fn default_refine_wep_stat_reduce() -> u8 {
    6
}

fn default_refine_item_stat_reduce() -> u8 {
    15
}

fn default_refine_cost() -> u32 {
    125
}

fn default_refine_ore_name() -> String {
    "BlackIronOre".to_string()
}

impl Default for RefineConfig {
    fn default() -> Self {
        Self {
            base_chance: default_refine_base_chance(),
            time_minutes: default_refine_time_minutes(),
            increase: default_refine_increase(),
            crit_chance: default_refine_crit_chance(),
            crit_increase: default_refine_crit_increase(),
            wep_stat_reduce: default_refine_wep_stat_reduce(),
            item_stat_reduce: default_refine_item_stat_reduce(),
            cost: default_refine_cost(),
            ore_name: default_refine_ore_name(),
        }
    }
}

/// 休息经验加成（C# Settings.Rested*：安全区/下线累积，BuffType.Rested +ExpRatePercent）
#[derive(Debug, Deserialize, Clone)]
pub struct RestedConfig {
    /// 累积 1 个休息单位所需秒数（C# Settings.RestedPeriod = 60，计数按秒）
    #[serde(default = "default_rested_period")]
    pub period_secs: u32,
    /// 每个休息单位对应的加成时长（分钟，C# Settings.RestedBuffLength = 10）
    #[serde(default = "default_rested_buff_minutes")]
    pub buff_length_minutes: u32,
    /// 休息经验加成百分比（C# Settings.RestedExpBonus = 5）
    #[serde(default = "default_rested_exp_bonus")]
    pub exp_bonus_percent: u32,
    /// 最大可累积加成单位数（C# Settings.RestedMaxBonus = 24）
    #[serde(default = "default_rested_max_bonus")]
    pub max_bonus: u32,
}

fn default_rested_period() -> u32 {
    60
}

fn default_rested_buff_minutes() -> u32 {
    10
}

fn default_rested_exp_bonus() -> u32 {
    5
}

fn default_rested_max_bonus() -> u32 {
    24
}

impl Default for RestedConfig {
    fn default() -> Self {
        Self {
            period_secs: default_rested_period(),
            buff_length_minutes: default_rested_buff_minutes(),
            exp_bonus_percent: default_rested_exp_bonus(),
            max_bonus: default_rested_max_bonus(),
        }
    }
}

#[derive(Debug, Deserialize, Clone)]
pub struct SocialConfig {
    /// 创建行会所需金币 (master C# Guild_CreationCost = GUILD_CREATION_COST_GOLD)
    /// 之前是 social.rs 里的硬编码常量 1_000_000,现在从 cfg 读取。
    /// cfg 字段名 guild_creation_cost_gold (snake_case),TOML 示例见 server.toml。
    #[serde(default = "default_guild_creation_cost")]
    pub guild_creation_cost_gold: u64,
    /// 是否启用配偶（结婚戒指）召回（C# Settings.WeddingRingRecall）
    #[serde(default = "default_wedding_ring_recall")]
    pub wedding_ring_recall_enabled: bool,
    /// 创建行会所需等级（C# Settings.Guild_RequiredLevel = 22）
    #[serde(default = "default_guild_required_level")]
    pub guild_required_level: u16,
    /// 新手行会名称（C# Settings.NewbieGuild，非 GM 禁止创建该名称）
    #[serde(default = "default_newbie_guild")]
    pub newbie_guild: String,
    /// 新手行会经验 buff 开关（C# Settings.NewbieGuildBuffEnabled = true）
    #[serde(default = "default_newbie_guild_buff_enabled")]
    pub newbie_guild_buff_enabled: bool,
    /// 新手行会经验加成 %（C# Settings.NewbieGuildExpBuff = 5）
    #[serde(default = "default_newbie_guild_exp_buff")]
    pub newbie_guild_exp_buff: i32,
    /// 行会经验倍率（C# Settings.Guild_ExpRate = 0.01）
    #[serde(default = "default_guild_exp_rate")]
    pub guild_exp_rate: f64,
    /// 行会每级分配点数（C# Settings.Guild_PointPerLevel = 0）
    #[serde(default = "default_guild_point_per_level")]
    pub guild_point_per_level: u8,
    /// 行会各级所需经验（C# Settings.Guild_ExperienceList，索引=等级）
    #[serde(default)]
    pub guild_experience_list: Vec<i64>,
    /// 行会各级成员上限（C# Settings.Guild_MembercapList，索引=等级）
    #[serde(default)]
    pub guild_membercap_list: Vec<i32>,
    /// 是否允许创建角色（C# Settings.AllowNewCharacter = true）
    #[serde(default = "default_allow_new_character")]
    pub allow_new_character: bool,
    /// 是否允许删除角色（C# Settings.AllowDeleteCharacter = true）
    #[serde(default = "default_allow_delete_character")]
    pub allow_delete_character: bool,
    /// 是否允许创建刺客（C# Settings.AllowCreateAssassin = true）
    #[serde(default = "default_true")]
    pub allow_create_assassin: bool,
    /// 是否允许创建弓箭手（C# Settings.AllowCreateArcher = true）
    #[serde(default = "default_true")]
    pub allow_create_archer: bool,
    /// 是否允许创建英雄（C# Settings.AllowNewHero = true）
    #[serde(default = "default_true")]
    pub allow_new_hero: bool,
    /// 英雄可创建职业（C# Settings.Hero_CanCreateClass[5]，默认全 true）
    #[serde(default = "default_hero_can_create_class")]
    pub hero_can_create_class: Vec<bool>,
    /// 创建英雄所需等级（C# Settings.Hero_RequiredLevel = 22；NPC [@CREATEHERO] 页门槛）
    #[serde(default = "default_hero_required_level")]
    pub hero_required_level: u8,
    /// 邮件寄金币费用（每 1000 金币，C# Settings.MailCostPer1KGold = 100）
    #[serde(default = "default_mail_cost_per_1k_gold")]
    pub mail_cost_per_1k_gold: u32,
    /// 邮件寄物品保险百分比（C# Settings.MailItemInsurancePercentage = 5）
    #[serde(default = "default_mail_item_insurance_percentage")]
    pub mail_item_insurance_percentage: u32,
    /// 邮票免费寄信（C# Settings.MailFreeWithStamp = true；Rust 暂无邮票，默认按收费处理）
    #[serde(default = "default_true")]
    pub mail_free_with_stamp: bool,
    /// 收件箱容量上限（C# Settings.MailCapacity = 100；登录时清理已读已收取无附件的旧邮件）
    #[serde(default = "default_mail_capacity")]
    pub mail_capacity: u32,
    /// 是否允许进入游戏（C# Settings.AllowStartGame，默认 false；GM 不受限；这里默认 true 避免破坏现状）
    #[serde(default = "default_true")]
    pub allow_start_game: bool,
    /// 是否允许修改密码（C# Settings.AllowChangePassword）
    #[serde(default = "default_true")]
    pub allow_change_password: bool,
    /// 是否允许注册新账号（C# Settings.AllowNewAccount）
    #[serde(default = "default_true")]
    pub allow_new_account: bool,
    /// 是否允许登录（C# Settings.AllowLogin）
    #[serde(default = "default_true")]
    pub allow_login: bool,
    /// 行会宣战费用（C# Settings.Guild_WarCost = 3000；<$GUILDWARFEE>）
    #[serde(default = "default_guild_war_cost")]
    pub guild_war_cost: u32,
    /// 行会战争时长（秒，C# Settings.Guild_WarTime = 180；<$GUILDWARTIME>）
    #[serde(default = "default_guild_war_time")]
    pub guild_war_time: i64,
    /// 离婚后再次结婚等待天数（C# Settings.MarriageCooldown = 7）
    #[serde(default = "default_marriage_cooldown_days")]
    pub marriage_cooldown_days: i64,
    /// 结婚最低等级（C# Settings.MarriageLevelRequired = 10）
    #[serde(default = "default_marriage_level_required")]
    pub marriage_level_required: u16,
    /// 更换婚戒费用系数（C# Settings.ReplaceWedRingCost = 125；cost = RequiredAmount*10*Cost）
    #[serde(default = "default_replace_wedring_cost")]
    pub replace_wedring_cost: u32,
    /// 配偶同图 16 格内经验加成 %（C# Settings.LoverEXPBonus = 5）
    #[serde(default = "default_lover_exp_bonus")]
    pub lover_exp_bonus: u32,
    /// 师徒等级差下限（C# Settings.MentorLevelGap = 10）
    #[serde(default = "default_mentor_level_gap")]
    pub mentor_level_gap: u8,
    /// 徒弟同图同组经验加成 %（C# Settings.MentorExpBoost = 10）
    #[serde(default = "default_mentor_exp_boost")]
    pub mentor_exp_boost: u8,
    /// 导师伤害加成 %（C# Settings.MentorDamageBoost = 10）
    #[serde(default = "default_mentor_damage_boost")]
    pub mentor_damage_boost: u8,
    /// 徒弟经验转导师 %（C# Settings.MenteeExpBank = 1）
    #[serde(default = "default_mentee_exp_bank")]
    pub mentee_exp_bank: u8,
    /// 师徒期限（天，C# Settings.MentorLength = 7）
    #[serde(default = "default_mentor_length_days")]
    pub mentor_length_days: u8,
}

fn default_allow_new_character() -> bool {
    true
}

fn default_allow_delete_character() -> bool {
    true
}

fn default_true() -> bool {
    true
}

fn default_hero_can_create_class() -> Vec<bool> {
    vec![true; 5]
}

fn default_hero_required_level() -> u8 {
    22 // C# Settings.Hero_RequiredLevel
}

fn default_mail_cost_per_1k_gold() -> u32 {
    100
}

fn default_mail_capacity() -> u32 {
    100
}

fn default_mail_item_insurance_percentage() -> u32 {
    5
}

fn default_guild_war_cost() -> u32 {
    3000
}

fn default_guild_war_time() -> i64 {
    180
}

fn default_marriage_cooldown_days() -> i64 {
    7
}

fn default_marriage_level_required() -> u16 {
    10
}

fn default_replace_wedring_cost() -> u32 {
    125
}

fn default_lover_exp_bonus() -> u32 {
    5
}

fn default_mentor_level_gap() -> u8 {
    10
}

fn default_mentor_exp_boost() -> u8 {
    10
}

fn default_mentor_damage_boost() -> u8 {
    10
}

fn default_mentee_exp_bank() -> u8 {
    1
}

fn default_mentor_length_days() -> u8 {
    7
}

fn default_wedding_ring_recall() -> bool {
    true
}

fn default_guild_required_level() -> u16 {
    22
}

fn default_newbie_guild_buff_enabled() -> bool {
    true
}

fn default_newbie_guild_exp_buff() -> i32 {
    5
}

fn default_guild_exp_rate() -> f64 {
    0.01
}

fn default_guild_point_per_level() -> u8 {
    0
}

fn default_newbie_guild() -> String {
    "NewbieGuild".to_string()
}

fn default_guild_creation_cost() -> u64 {
    1_000_000
}

impl Default for SocialConfig {
    fn default() -> Self {
        Self {
            guild_creation_cost_gold: default_guild_creation_cost(),
            wedding_ring_recall_enabled: default_wedding_ring_recall(),
            guild_required_level: default_guild_required_level(),
            newbie_guild: default_newbie_guild(),
            newbie_guild_buff_enabled: default_newbie_guild_buff_enabled(),
            newbie_guild_exp_buff: default_newbie_guild_exp_buff(),
            guild_exp_rate: default_guild_exp_rate(),
            guild_point_per_level: default_guild_point_per_level(),
            guild_experience_list: Vec::new(),
            guild_membercap_list: Vec::new(),
            allow_new_character: default_allow_new_character(),
            allow_delete_character: default_allow_delete_character(),
            allow_create_assassin: default_true(),
            allow_create_archer: default_true(),
            allow_new_hero: default_true(),
            hero_can_create_class: default_hero_can_create_class(),
            hero_required_level: default_hero_required_level(),
            mail_cost_per_1k_gold: default_mail_cost_per_1k_gold(),
            mail_item_insurance_percentage: default_mail_item_insurance_percentage(),
            mail_free_with_stamp: default_true(),
            mail_capacity: default_mail_capacity(),
            allow_start_game: default_true(),
            allow_change_password: default_true(),
            allow_new_account: default_true(),
            allow_login: default_true(),
            guild_war_cost: default_guild_war_cost(),
            guild_war_time: default_guild_war_time(),
            marriage_cooldown_days: default_marriage_cooldown_days(),
            marriage_level_required: default_marriage_level_required(),
            replace_wedring_cost: default_replace_wedring_cost(),
            lover_exp_bonus: default_lover_exp_bonus(),
            mentor_level_gap: default_mentor_level_gap(),
            mentor_exp_boost: default_mentor_exp_boost(),
            mentor_damage_boost: default_mentor_damage_boost(),
            mentee_exp_bank: default_mentee_exp_bank(),
            mentor_length_days: default_mentor_length_days(),
        }
    }
}

#[derive(Debug, Deserialize, Clone)]
pub struct NetworkConfig {
    /// 监听地址
    pub listen_addr: String,
    /// 帧编码 XOR 密钥
    pub xor_key: u8,
    /// 最大连接数
    pub max_connections: usize,
}

#[derive(Debug, Deserialize, Clone)]
pub struct DatabaseConfig {
    /// SQLite 数据库路径
    pub path: String,
}

#[derive(Debug, Deserialize, Clone)]
pub struct ServerWorldConfig {
    /// 游戏主循环 tick 间隔（毫秒）
    pub tick_ms: u64,
    /// 地图数据目录
    pub map_data_dir: String,
    /// 全局掉落倍率（C# Settings.DropRate，默认 1.0；影响掉落概率 chance * drop_rate）
    #[serde(default = "default_drop_rate")]
    pub drop_rate: f64,
    /// 玩家升级经验曲线（C# Settings.ExperienceList，索引=Level-1；空表回退 ×1.5）
    #[serde(default)]
    pub experience_list: Vec<i64>,
    /// 地面物品超时秒数（C# Settings.ItemTimeOut = 30）
    #[serde(default = "default_item_timeout")]
    pub item_timeout_secs: u32,
    /// 金币掉落每堆上限（C# Settings.MaxDropGold = 2000）
    #[serde(default = "default_max_drop_gold")]
    pub max_drop_gold: u32,
    /// 金币是否落地（C# Settings.DropGold = true：落地；false：直接进击杀者背包并按组队平分）
    #[serde(default = "default_drop_gold")]
    pub drop_gold: bool,
    /// 服务器公告文件（C# Settings.NoticePath = EnvirPath/Notice.txt；首行 Title=，其余为消息）
    #[serde(default = "default_notice_path")]
    pub notice_path: String,
    /// 死亡经验惩罚百分比（0=关闭，对齐 C# 无通用死亡惩罚；>0 时按当前等级经验百分比扣除）
    #[serde(default = "default_death_exp_penalty_percent")]
    pub death_exp_penalty_percent: u32,
    /// 服务端移动节流间隔毫秒（0=关闭，默认关以兼容 Bevy 客户端节奏；>0 时按 C# HumanObject MoveDelay=600ms/动作 节流，Slow 毒 ×2）
    #[serde(default = "default_movement_pacing_ms")]
    pub movement_pacing_ms: u64,
    /// 长期未登录角色自动归档月数（C# Settings.ArchiveInactiveCharacterAfterMonths = 12；启动时执行）
    #[serde(default = "default_archive_inactive_after_months")]
    pub archive_inactive_after_months: u32,
    /// 怪物召回（防风筝）总开关（C# Settings.MonsterRecallEnabled = true）
    #[serde(default = "default_true")]
    pub monster_recall_enabled: bool,
    /// 怪物召回距离（C# Settings.MonsterRecallRange = 12；Math.Max(1,...)）
    #[serde(default = "default_monster_recall_range")]
    pub monster_recall_range: i32,
    /// 怪物召回冷却毫秒（C# Settings.MonsterRecallCooldown = 5000；Math.Max(0,...)）
    #[serde(default = "default_monster_recall_cooldown_ms")]
    pub monster_recall_cooldown_ms: u64,
    /// 怪物等级差经验衰减开关（C# Settings.ExpMobLevelDifference = true；关闭时不减）
    #[serde(default = "default_true")]
    pub exp_mob_level_difference: bool,
    /// 回血权重（C# Settings.HealthRegenWeight = 10：healthRegen += regen * HealthRecovery / weight）
    #[serde(default = "default_health_regen_weight")]
    pub health_regen_weight: u32,
    /// 回蓝权重（C# Settings.ManaRegenWeight = 10）
    #[serde(default = "default_mana_regen_weight")]
    pub mana_regen_weight: u32,
    /// 商店隐藏附加属性（C# Settings.GoodsHideAddedStats = true）
    #[serde(default = "default_goods_hide_added_stats")]
    pub goods_hide_added_stats: bool,
    /// NPC 二手货/回购系统开关（C# Settings.GoodsOn = true；关闭后卖出不入 BuyBack/UsedGoods，[@BUYUSED]/[@BUYBACK] 不响应）
    #[serde(default = "default_goods_on")]
    pub goods_on: bool,
    /// 每 NPC 每种物品索引的二手货（UsedGoods）上限（C# Settings.GoodsMaxStored = 15）
    #[serde(default = "default_goods_max_stored")]
    pub goods_max_stored: u32,
    /// 回购（BuyBack）过期分钟数（C# Settings.GoodsBuyBackTime = 60；过期未回购转入二手货）
    #[serde(default = "default_goods_buy_back_time_minutes")]
    pub goods_buy_back_time_minutes: u32,
    /// 每玩家每 NPC 回购（BuyBack）上限（C# Settings.GoodsBuyBackMaxStored = 20）
    #[serde(default = "default_goods_buy_back_max_stored")]
    pub goods_buy_back_max_stored: u32,
    /// 精英怪配置（C# Settings.MonsterRarity* 第一阶段：单级精英，默认保留 Rust 当前值；
    /// C# Elite 参考：2.25x HP / 75% 掉落加成）
    #[serde(default)]
    pub rarity: RarityConfig,
}

fn default_health_regen_weight() -> u32 {
    10
}

fn default_mana_regen_weight() -> u32 {
    10
}

fn default_goods_hide_added_stats() -> bool {
    true
}

fn default_goods_on() -> bool {
    true
}

fn default_goods_max_stored() -> u32 {
    15
}

fn default_goods_buy_back_time_minutes() -> u32 {
    60
}

fn default_goods_buy_back_max_stored() -> u32 {
    20
}

fn default_item_timeout() -> u32 {
    30
}

fn default_max_drop_gold() -> u32 {
    2000
}

fn default_drop_gold() -> bool {
    true
}

fn default_notice_path() -> String {
    "Notice.txt".to_string()
}

fn default_movement_pacing_ms() -> u64 {
    0
}

fn default_archive_inactive_after_months() -> u32 {
    12
}

fn default_monster_recall_range() -> i32 {
    12
}

fn default_monster_recall_cooldown_ms() -> u64 {
    5000
}

fn default_death_exp_penalty_percent() -> u32 {
    0
}

#[derive(Debug, Deserialize, Clone)]
pub struct RarityConfig {
    /// 精英概率百分比（C# Settings.MonsterRarityEliteChancePercent = 0.1；#2360）
    #[serde(default = "default_elite_chance")]
    pub elite_chance_percent: f64,
    /// Uncommon 概率百分比（C# Settings.MonsterRarityUncommonChancePercent = 3.0）
    #[serde(default = "default_uncommon_chance")]
    pub uncommon_chance_percent: f64,
    /// Rare 概率百分比（C# Settings.MonsterRarityRareChancePercent = 0.75）
    #[serde(default = "default_rare_chance")]
    pub rare_chance_percent: f64,
    /// Uncommon 倍率（C# MonsterRarityUncommon*）
    #[serde(default = "default_uncommon_hp")]
    pub uncommon_hp_multiplier: f64,
    #[serde(default = "default_uncommon_defense")]
    pub uncommon_defense_multiplier: f64,
    #[serde(default = "default_uncommon_damage")]
    pub uncommon_damage_multiplier: f64,
    #[serde(default = "default_uncommon_exp")]
    pub uncommon_exp_multiplier: f64,
    #[serde(default = "default_uncommon_gold")]
    pub uncommon_gold_multiplier: f64,
    #[serde(default = "default_uncommon_item_bonus")]
    pub uncommon_item_drop_bonus_percent: i32,
    #[serde(default = "default_uncommon_gold_bonus")]
    pub uncommon_gold_drop_bonus_percent: i32,
    /// Rare 倍率（C# MonsterRarityRare*）
    #[serde(default = "default_rare_hp")]
    pub rare_hp_multiplier: f64,
    #[serde(default = "default_rare_defense")]
    pub rare_defense_multiplier: f64,
    #[serde(default = "default_rare_damage")]
    pub rare_damage_multiplier: f64,
    #[serde(default = "default_rare_exp")]
    pub rare_exp_multiplier: f64,
    #[serde(default = "default_rare_gold")]
    pub rare_gold_multiplier: f64,
    #[serde(default = "default_rare_item_bonus")]
    pub rare_item_drop_bonus_percent: i32,
    #[serde(default = "default_rare_gold_bonus")]
    pub rare_gold_drop_bonus_percent: i32,
    /// 精英 HP 倍率
    #[serde(default = "default_elite_hp_multiplier")]
    pub elite_hp_multiplier: f64,
    /// 精英防御倍率（C# MonsterRarityEliteDefenseMultiplier = 1.55）
    #[serde(default = "default_elite_defense_multiplier")]
    pub elite_defense_multiplier: f64,
    /// 精英伤害倍率
    #[serde(default = "default_elite_dmg_multiplier")]
    pub elite_dmg_multiplier: f64,
    /// 精英经验倍率
    #[serde(default = "default_elite_xp_multiplier")]
    pub elite_xp_multiplier: f64,
    /// 精英物品掉落加成 %（C# Settings.MonsterRarityEliteItemDropBonusPercent = 75）
    #[serde(default = "default_elite_item_drop_bonus_percent")]
    pub elite_item_drop_bonus_percent: i32,
    /// 精英金币掉落加成 %（C# Settings.MonsterRarityEliteGoldDropBonusPercent = 75）
    #[serde(default = "default_elite_gold_drop_bonus_percent")]
    pub elite_gold_drop_bonus_percent: i32,
    /// 精英金币倍率（C# MonsterRarityData.Elite.GoldMultiplier = 2.50，ApplyGoldModifier）
    #[serde(default = "default_elite_gold_multiplier")]
    pub elite_gold_multiplier: f64,
}

fn default_elite_chance() -> f64 {
    0.1 // C# MonsterRarityEliteChancePercent
}

fn default_elite_defense_multiplier() -> f64 {
    1.55
}

fn default_uncommon_chance() -> f64 {
    3.0
}
fn default_rare_chance() -> f64 {
    0.75
}
fn default_uncommon_hp() -> f64 {
    1.25
}
fn default_uncommon_defense() -> f64 {
    1.15
}
fn default_uncommon_damage() -> f64 {
    1.15
}
fn default_uncommon_exp() -> f64 {
    1.20
}
fn default_uncommon_gold() -> f64 {
    1.25
}
fn default_uncommon_item_bonus() -> i32 {
    15
}
fn default_uncommon_gold_bonus() -> i32 {
    15
}
fn default_rare_hp() -> f64 {
    1.60
}
fn default_rare_defense() -> f64 {
    1.30
}
fn default_rare_damage() -> f64 {
    1.35
}
fn default_rare_exp() -> f64 {
    1.60
}
fn default_rare_gold() -> f64 {
    1.75
}
fn default_rare_item_bonus() -> i32 {
    35
}
fn default_rare_gold_bonus() -> i32 {
    35
}

fn default_elite_hp_multiplier() -> f64 {
    2.25 // C# MonsterRarityData.Elite HpMultiplier
}

fn default_elite_dmg_multiplier() -> f64 {
    1.65 // C# Elite DamageMultiplier
}

fn default_elite_xp_multiplier() -> f64 {
    2.20 // C# Elite ExpMultiplier
}

fn default_elite_item_drop_bonus_percent() -> i32 {
    75
}

fn default_elite_gold_drop_bonus_percent() -> i32 {
    75
}

fn default_elite_gold_multiplier() -> f64 {
    2.50
}

impl Default for RarityConfig {
    fn default() -> Self {
        Self {
            elite_chance_percent: default_elite_chance(),
            uncommon_chance_percent: default_uncommon_chance(),
            rare_chance_percent: default_rare_chance(),
            uncommon_hp_multiplier: default_uncommon_hp(),
            uncommon_defense_multiplier: default_uncommon_defense(),
            uncommon_damage_multiplier: default_uncommon_damage(),
            uncommon_exp_multiplier: default_uncommon_exp(),
            uncommon_gold_multiplier: default_uncommon_gold(),
            uncommon_item_drop_bonus_percent: default_uncommon_item_bonus(),
            uncommon_gold_drop_bonus_percent: default_uncommon_gold_bonus(),
            rare_hp_multiplier: default_rare_hp(),
            rare_defense_multiplier: default_rare_defense(),
            rare_damage_multiplier: default_rare_damage(),
            rare_exp_multiplier: default_rare_exp(),
            rare_gold_multiplier: default_rare_gold(),
            rare_item_drop_bonus_percent: default_rare_item_bonus(),
            rare_gold_drop_bonus_percent: default_rare_gold_bonus(),
            elite_hp_multiplier: default_elite_hp_multiplier(),
            elite_defense_multiplier: default_elite_defense_multiplier(),
            elite_dmg_multiplier: default_elite_dmg_multiplier(),
            elite_xp_multiplier: default_elite_xp_multiplier(),
            elite_item_drop_bonus_percent: default_elite_item_drop_bonus_percent(),
            elite_gold_drop_bonus_percent: default_elite_gold_drop_bonus_percent(),
            elite_gold_multiplier: default_elite_gold_multiplier(),
        }
    }
}

fn default_drop_rate() -> f64 {
    1.0
}

impl Default for ServerConfig {
    fn default() -> Self {
        Self {
            network: NetworkConfig {
                listen_addr: "0.0.0.0:7000".to_string(),
                xor_key: 0xAA,
                max_connections: 1024,
            },
            database: DatabaseConfig {
                path: "data/server.db".to_string(),
            },
            server: ServerWorldConfig {
                tick_ms: 100,
                map_data_dir: "Data".to_string(),
                drop_rate: default_drop_rate(),
                experience_list: Vec::new(),
                item_timeout_secs: default_item_timeout(),
                max_drop_gold: default_max_drop_gold(),
                drop_gold: default_drop_gold(),
                health_regen_weight: default_health_regen_weight(),
                mana_regen_weight: default_mana_regen_weight(),
                goods_hide_added_stats: default_goods_hide_added_stats(),
                goods_on: default_goods_on(),
                goods_max_stored: default_goods_max_stored(),
                goods_buy_back_time_minutes: default_goods_buy_back_time_minutes(),
                goods_buy_back_max_stored: default_goods_buy_back_max_stored(),
                rarity: RarityConfig::default(),
                notice_path: default_notice_path(),
                death_exp_penalty_percent: default_death_exp_penalty_percent(),
                movement_pacing_ms: default_movement_pacing_ms(),
                archive_inactive_after_months: default_archive_inactive_after_months(),
                monster_recall_enabled: default_true(),
                monster_recall_range: default_monster_recall_range(),
                monster_recall_cooldown_ms: default_monster_recall_cooldown_ms(),
                exp_mob_level_difference: default_true(),
            },
            social: SocialConfig::default(),
            conquest: ConquestConfig::default(),
            rested: RestedConfig::default(),
            pvp: PvpConfig::default(),
            refine: RefineConfig::default(),
        }
    }
}

pub fn load_config(path: &str) -> anyhow::Result<ServerConfig> {
    let content = std::fs::read_to_string(path)?;
    let config: ServerConfig = toml::from_str(&content)?;
    Ok(config)
}

#[cfg(test)]
mod tests {
    use super::*;

    /// #2360：RarityConfig 默认值与 C# Settings/MonsterRarityData 对齐
    #[test]
    fn rarity_defaults_match_csharp_settings() {
        let c = RarityConfig::default();
        assert_eq!(c.elite_chance_percent, 0.1); // C# MonsterRarityEliteChancePercent
        assert_eq!(c.rare_chance_percent, 0.75); // C# MonsterRarityRareChancePercent
        assert_eq!(c.uncommon_chance_percent, 3.0); // C# MonsterRarityUncommonChancePercent
        assert_eq!(c.elite_hp_multiplier, 2.25);
        assert_eq!(c.elite_defense_multiplier, 1.55);
        assert_eq!(c.elite_dmg_multiplier, 1.65);
        assert_eq!(c.elite_xp_multiplier, 2.20);
        assert_eq!(c.elite_gold_multiplier, 2.50);
        assert_eq!(c.elite_item_drop_bonus_percent, 75);
        assert_eq!(c.elite_gold_drop_bonus_percent, 75);
    }

    /// #2360：ConquestConfig 默认值与 C# Settings 对齐（BuyGTGold/ExtendGT/GTSale 最低 200 万/GTDays）
    #[test]
    fn conquest_defaults_match_csharp_settings() {
        let c = ConquestConfig::default();
        assert_eq!(c.buy_gold, 10_000_000); // C# BuyGTGold
        assert_eq!(c.extend_gold, 1_000_000); // C# ExtendGT
        assert_eq!(c.gt_sale_min_price, 2_000_000); // C# NPCSegment GTSale 最低 200 万
        assert_eq!(c.gt_days, 30); // C# GTDays
    }

    /// #2366：SocialConfig 英雄创建等级门槛默认 22（C# Settings.Hero_RequiredLevel）
    #[test]
    fn hero_required_level_defaults_to_22() {
        let c = SocialConfig::default();
        assert_eq!(c.hero_required_level, 22);
    }

    /// #2376：Goods 配置默认值与 C# Settings 对齐（GoodsOn/GoodsMaxStored/GoodsBuyBackTime/GoodsBuyBackMaxStored）
    #[test]
    fn goods_defaults_match_csharp_settings() {
        let c = ServerConfig::default().server;
        assert!(c.goods_on);
        assert_eq!(c.goods_max_stored, 15);
        assert_eq!(c.goods_buy_back_time_minutes, 60);
        assert_eq!(c.goods_buy_back_max_stored, 20);
    }

    /// #2382：MailCapacity 默认 100（C# Settings.MailCapacity）
    #[test]
    fn mail_capacity_defaults_to_100() {
        let c = ServerConfig::default().social;
        assert_eq!(c.mail_capacity, 100);
    }

    /// #2384：归档月数默认 12（C# Settings.ArchiveInactiveCharacterAfterMonths）
    #[test]
    fn archive_inactive_after_months_defaults_to_12() {
        let c = ServerConfig::default().server;
        assert_eq!(c.archive_inactive_after_months, 12);
    }

    /// #2390：怪物召回配置默认值（C# Settings.MonsterRecall*）
    #[test]
    fn monster_recall_defaults_match_csharp() {
        let c = ServerConfig::default().server;
        assert!(c.monster_recall_enabled);
        assert_eq!(c.monster_recall_range, 12);
        assert_eq!(c.monster_recall_cooldown_ms, 5000);
        assert!(c.exp_mob_level_difference);
    }

    /// #2392：精炼配置默认值（C# Settings.Refine*）
    #[test]
    fn refine_defaults_match_csharp() {
        let c = RefineConfig::default();
        assert_eq!(c.base_chance, 20);
        assert_eq!(c.time_minutes, 20);
        assert_eq!(c.increase, 1);
        assert_eq!(c.crit_chance, 10);
        assert_eq!(c.crit_increase, 2);
        assert_eq!(c.wep_stat_reduce, 6);
        assert_eq!(c.item_stat_reduce, 15);
        assert_eq!(c.cost, 125);
        assert_eq!(c.ore_name, "BlackIronOre");
    }

    /// #2394：婚姻/配偶配置默认值（C# Settings.Marriage*/ReplaceWedRingCost/LoverEXPBonus）
    #[test]
    fn marriage_defaults_match_csharp() {
        let c = ServerConfig::default().social;
        assert_eq!(c.marriage_cooldown_days, 7);
        assert_eq!(c.marriage_level_required, 10);
        assert_eq!(c.replace_wedring_cost, 125);
        assert_eq!(c.lover_exp_bonus, 5);
    }

    /// #2396：师徒配置默认值（C# Settings.MentorLevelGap/MentorExpBoost）
    #[test]
    fn mentor_defaults_match_csharp() {
        let c = ServerConfig::default().social;
        assert_eq!(c.mentor_level_gap, 10);
        assert_eq!(c.mentor_exp_boost, 10);
        assert_eq!(c.mentor_damage_boost, 10);
        assert_eq!(c.mentee_exp_bank, 1);
        assert_eq!(c.mentor_length_days, 7);
    }

    /// #2402：真实 config/server.toml 能被解析（新配置段不破坏服务器启动）
    #[test]
    fn load_real_server_toml_parses() {
        let path = concat!(env!("CARGO_MANIFEST_DIR"), "/config/server.toml");
        let cfg = load_config(path).expect("server.toml 必须能解析");
        // 抽查近期新增段/字段的真实文件值
        assert!(cfg.server.goods_on);
        assert_eq!(cfg.server.goods_max_stored, 50); // 对齐 GoodsSystem.ini MaxStored（#2408）
        assert_eq!(cfg.server.monster_recall_range, 12);
        assert_eq!(cfg.server.archive_inactive_after_months, 12);
        assert_eq!(cfg.social.marriage_cooldown_days, 7);
        assert_eq!(cfg.social.mentor_damage_boost, 10);
        assert_eq!(cfg.social.mentee_exp_bank, 1);
        assert_eq!(cfg.refine.cost, 125);
        assert_eq!(cfg.refine.base_chance, 20);
        assert_eq!(cfg.refine.ore_name, "BlackIronOre");
    }
}
