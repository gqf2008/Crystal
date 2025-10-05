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
pub mod downloader;

// 主要功能模块
pub mod graphics;          // ✅ 已迁移到 ggez
pub mod network;           // ✅ 不依赖 winit/wgpu
pub mod objects;           // ✅ 不依赖 winit/wgpu
pub mod resolution;        // ✅ 不依赖 winit/wgpu
pub mod resources;         // ✅ 不依赖 winit/wgpu
pub mod utils;             // ✅ 不依赖 winit/wgpu
pub mod scenes;            // ⚠️ 部分使用 winit 类型(Scene trait)

// 以下模块依赖 winit/wgpu/rodio，暂时禁用
// 使用 main_ggez.rs 替代
// pub mod program;        // 依赖 winit
// pub mod ui;             // 依赖 winit
// pub mod controls;       // 依赖 winit
// pub mod forms;          // 依赖 winit
// pub mod sounds;         // 依赖 rodio

// 重新导出常用类型
pub use error::{ClientError, ClientResult};
pub use settings::ClientSettings;
