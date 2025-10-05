//! Legend of Mir 2 - Rust Edition
//! 
//! 将传奇世界客户端从 C# 移植到 Rust
//! 
//! 主要模块:
//! - `graphics`: 图形渲染 (MirGraphics)
//! - `network`: 网络通信 (MirNetwork)
//! - `utils`: 工具函数 (Utils)

pub mod error;
pub mod version;
pub mod settings;
pub mod key_bind_settings;
pub mod program;
pub mod ui;
pub mod downloader;

// 主要功能模块
pub mod controls;
pub mod forms;
pub mod graphics;
pub mod network;
pub mod objects;
pub mod resolution;
pub mod resources;
pub mod utils;
pub mod scenes;
pub mod sounds;

// 重新导出常用类型
pub use error::{ClientError, ClientResult};
pub use settings::ClientSettings;
