// ============================================================================
// Client Bevy - 库模块
// ============================================================================
// 传奇2 (Legend of Mir 2) 客户端 Bevy 移植版

// 帧表/数据函数参数多是协议与资源布局设计决定，非代码质量问题
#![allow(clippy::too_many_arguments)]

pub mod actor;
pub mod event_bus;
pub mod game;
pub mod map_renderer;
pub mod map_tile_anim;
pub mod network;
pub mod objects;
pub mod resources;
pub mod scenes;
pub mod ui;
