use serde::Deserialize;

#[derive(Debug, Deserialize, Clone)]
pub struct ServerConfig {
    pub network: NetworkConfig,
    pub database: DatabaseConfig,
    pub server: ServerWorldConfig,
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
            },
        }
    }
}

pub fn load_config(path: &str) -> anyhow::Result<ServerConfig> {
    let content = std::fs::read_to_string(path)?;
    let config: ServerConfig = toml::from_str(&content)?;
    Ok(config)
}
