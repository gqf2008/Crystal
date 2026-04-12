// ============================================================================
// Client Macroquad - 库模块
// ============================================================================
//
// 导出所有核心模块供 bin 使用

// 网络协议/游戏逻辑函数参数较多是协议设计决定，非代码质量问题
#![allow(clippy::too_many_arguments)]
// UI 对话框枚举因携带不同类型数据导致大小差异，属正常设计
#![allow(clippy::large_enum_variant)]
// 复杂类型由协议包结构决定
#![allow(clippy::type_complexity)]
// 全局玩家实体只有一个，用 for 循环代替 .next() 是为了保持与 ECS 查询的一致性
#![allow(clippy::never_loop)]

pub mod camera;
pub mod compat;
pub mod components;
pub mod coord;
pub mod core;
pub mod event_bus;
pub mod game;
pub mod network;
pub mod map_renderer;
pub mod objects;
pub mod resources;
pub mod scenes;
pub mod systems;
pub mod ui;
pub mod utils;

// ✨ ecs_macros 兼容性别名
pub mod ecs {
    pub use crate::game::GameContext;
    pub use crate::systems;
    pub use crate::components;
}
