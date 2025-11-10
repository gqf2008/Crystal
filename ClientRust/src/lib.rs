// #![feature(specialization)]
// #![allow(incomplete_features)]
//! Legend of Mir 2 - Rust Edition
//!
//! 将传奇世界客户端从 C# 移植到 Rust
//!
//! ## 架构分层
//!
//! ```
//! resources/     # 🆕 纯数据资源层（零渲染依赖）
//!   ├── lib_loader.rs   # .lib 文件加载
//!   └── map_loader.rs   # 地图数据加载
//!
//! backends/      # 🆕 渲染后端抽象层
//!   ├── types.rs        # 通用类型
//!   ├── ggez/           # ggez 后端
//!   └── macroquad/      # macroquad 后端
//!
//! graphics/      # ggez 特定图形功能（旧代码，逐步迁移）
//! ecs/           # ✅ ECS 游戏逻辑（保持不变）
//! network/       # ✅ 网络通信（保持不变）
//! ```

pub mod error;
pub mod settings;
pub mod version;

// 🆕 纯数据资源管理层（零渲染依赖）
pub mod resources;

// 🆕 渲染后端抽象层（支持多引擎）
pub mod backends;

// 图形功能（包含 ggez 特定代码，逐步迁移到 backends/ggez/）
pub mod graphics;

// 网络通信
pub mod network;

// objects 模块（只在 ggez 后端时完整可用）
#[cfg(feature = "backend-ggez")]
pub mod objects;

// map_code 模块（MapReader 和 CellInfo）- 无渲染依赖，两个后端都可用
#[cfg(all(feature = "backend-macroquad", not(feature = "backend-ggez")))]
pub mod objects {
    pub mod map_code;
    pub use map_code::{CellInfo, MapReader};
}

// ECS 模块（只在使用 ggez 后端时编译）
#[cfg(feature = "backend-ggez")]
pub mod ecs;

// 重新导出常用类型
pub use error::{ClientError, ClientResult};
pub use settings::ClientSettings;
