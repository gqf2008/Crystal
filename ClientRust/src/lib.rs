// #![feature(specialization)]
// #![allow(incomplete_features)]
//! Legend of Mir 2 - Rust Edition
//!
//! 将传奇世界客户端从 C# 移植到 Rust
//!
//! ## 架构分层
//!
//! ```
//! graphics/      # ggez 图形功能
//! objects/       # 游戏对象（地图、玩家等）
//! ecs/           # ECS 游戏逻辑
//! network/       # 网络通信
//! ```

pub mod error;
pub mod settings;
pub mod version;

// 图形功能（ggez 后端）
pub mod graphics;

// 网络通信
pub mod network;

// 游戏对象
pub mod objects;

// ECS 模块
pub mod ecs;

// 重新导出常用类型
pub use error::{ClientError, ClientResult};
pub use settings::ClientSettings;
