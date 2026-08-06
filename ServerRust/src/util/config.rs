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
    1_000_000
}

fn default_conquest_extend_gold() -> u64 {
    500_000
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
    /// 邮件寄金币费用（每 1000 金币，C# Settings.MailCostPer1KGold = 100）
    #[serde(default = "default_mail_cost_per_1k_gold")]
    pub mail_cost_per_1k_gold: u32,
    /// 邮件寄物品保险百分比（C# Settings.MailItemInsurancePercentage = 5）
    #[serde(default = "default_mail_item_insurance_percentage")]
    pub mail_item_insurance_percentage: u32,
    /// 邮票免费寄信（C# Settings.MailFreeWithStamp = true；Rust 暂无邮票，默认按收费处理）
    #[serde(default = "default_true")]
    pub mail_free_with_stamp: bool,
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

fn default_mail_cost_per_1k_gold() -> u32 {
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

fn default_wedding_ring_recall() -> bool {
    true
}

fn default_guild_required_level() -> u16 {
    22
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
            allow_new_character: default_allow_new_character(),
            allow_delete_character: default_allow_delete_character(),
            allow_create_assassin: default_true(),
            allow_create_archer: default_true(),
            allow_new_hero: default_true(),
            hero_can_create_class: default_hero_can_create_class(),
            mail_cost_per_1k_gold: default_mail_cost_per_1k_gold(),
            mail_item_insurance_percentage: default_mail_item_insurance_percentage(),
            mail_free_with_stamp: default_true(),
            allow_start_game: default_true(),
            allow_change_password: default_true(),
            allow_new_account: default_true(),
            allow_login: default_true(),
            guild_war_cost: default_guild_war_cost(),
            guild_war_time: default_guild_war_time(),
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
    /// 地面物品超时秒数（C# Settings.ItemTimeOut = 30）
    #[serde(default = "default_item_timeout")]
    pub item_timeout_secs: u32,
    /// 金币掉落每堆上限（C# Settings.MaxDropGold = 2000）
    #[serde(default = "default_max_drop_gold")]
    pub max_drop_gold: u32,
    /// 服务器公告文件（C# Settings.NoticePath = EnvirPath/Notice.txt；首行 Title=，其余为消息）
    #[serde(default = "default_notice_path")]
    pub notice_path: String,
    /// 精英怪配置（C# Settings.MonsterRarity* 第一阶段：单级精英，默认保留 Rust 当前值；
    /// C# Elite 参考：2.25x HP / 75% 掉落加成）
    #[serde(default)]
    pub rarity: RarityConfig,
}

fn default_item_timeout() -> u32 {
    30
}

fn default_max_drop_gold() -> u32 {
    2000
}

fn default_notice_path() -> String {
    "Notice.txt".to_string()
}

#[derive(Debug, Deserialize, Clone)]
pub struct RarityConfig {
    /// 精英概率百分比（1..=100）
    #[serde(default = "default_elite_chance")]
    pub elite_chance_percent: u8,
    /// 精英 HP 倍率
    #[serde(default = "default_elite_hp_multiplier")]
    pub elite_hp_multiplier: f64,
    /// 精英伤害倍率
    #[serde(default = "default_elite_dmg_multiplier")]
    pub elite_dmg_multiplier: f64,
    /// 精英经验倍率
    #[serde(default = "default_elite_xp_multiplier")]
    pub elite_xp_multiplier: f64,
}

fn default_elite_chance() -> u8 {
    3
}

fn default_elite_hp_multiplier() -> f64 {
    2.0
}

fn default_elite_dmg_multiplier() -> f64 {
    1.5
}

fn default_elite_xp_multiplier() -> f64 {
    2.0
}

impl Default for RarityConfig {
    fn default() -> Self {
        Self {
            elite_chance_percent: default_elite_chance(),
            elite_hp_multiplier: default_elite_hp_multiplier(),
            elite_dmg_multiplier: default_elite_dmg_multiplier(),
            elite_xp_multiplier: default_elite_xp_multiplier(),
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
                item_timeout_secs: default_item_timeout(),
                max_drop_gold: default_max_drop_gold(),
                rarity: RarityConfig::default(),
                notice_path: default_notice_path(),
            },
            social: SocialConfig::default(),
            conquest: ConquestConfig::default(),
        }
    }
}

pub fn load_config(path: &str) -> anyhow::Result<ServerConfig> {
    let content = std::fs::read_to_string(path)?;
    let config: ServerConfig = toml::from_str(&content)?;
    Ok(config)
}
