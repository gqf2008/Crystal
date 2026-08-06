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

impl Default for ConquestConfig {
    fn default() -> Self {
        Self {
            buy_gold: default_conquest_buy_gold(),
            extend_gold: default_conquest_extend_gold(),
            gt_sale_min_price: default_conquest_gt_sale_min(),
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
}

fn default_item_timeout() -> u32 {
    30
}

fn default_max_drop_gold() -> u32 {
    2000
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
