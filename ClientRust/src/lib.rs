// #![feature(specialization)]
// #![allow(incomplete_features)]
//! Legend of Mir 2 - Rust Edition
//! 
//! 将传奇世界客户端从 C# 移植到 Rust

pub mod error;
pub mod version;
pub mod settings;
// 主要功能模块
pub mod graphics;          // ✅ 已迁移到 ggez
pub mod network;           // ✅ 不依赖 winit/wgpu
pub mod objects;           // ✅ 不依赖 winit/wgpu
// ECS 模块 (GGEZ + hecs 架构)
pub mod ecs;               // 🆕 轻量级 ECS 架构 (推荐)

// 重新导出常用类型
pub use error::{ClientError, ClientResult};
pub use settings::ClientSettings;
