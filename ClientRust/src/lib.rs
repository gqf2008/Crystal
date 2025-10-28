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
// pub mod key_bind_settings;  // TODO: 需要实现
// pub mod downloader;
// pub mod program;           // 🔧 客户端运行时和初始化逻辑

// 主要功能模块
pub mod graphics;          // ✅ 已迁移到 ggez
pub mod network;           // ✅ 不依赖 winit/wgpu
pub mod objects;           // ✅ 不依赖 winit/wgpu
// pub mod resolution;        // ✅ 不依赖 winit/wgpu
// pub mod resources;         // ✅ 不依赖 winit/wgpu
// pub mod scenes;            // ⚠️ 部分使用 winit 类型(Scene trait)
// pub mod ui;                // UI模块
// pub mod controls;          // ⚠️ 部分使用 winit 类型(Control trait)
// pub mod systems;           // 🆕 GameScene 子系统架构

// ECS 模块 (GGEZ + hecs 架构)
pub mod ecs;               // 🆕 轻量级 ECS 架构 (推荐)

// 🆕 算法模块 - 游戏通用算法（寻路、碰撞检测等）
pub mod algorithms;

// Bevy 模块 (新架构 - 暂时禁用)
// pub mod bevy;              // Bevy 0.17.2 ECS 架构 (已搁置,使用 GGEZ + hecs)

// 重新导出常用类型
pub use error::{ClientError, ClientResult};
pub use settings::ClientSettings;
