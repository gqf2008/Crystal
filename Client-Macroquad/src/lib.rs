// ============================================================================
// Client Macroquad - 库模块
// ============================================================================
//
// 导出所有核心模块供 bin 使用

pub mod camera;
pub mod compat;
pub mod components;
pub mod coord;
pub mod core;
pub mod event_bus;
pub mod game;
pub mod network;
pub mod map_renderer;
pub mod resources;
pub mod scenes;
pub mod systems;

// ✨ ecs_macros 兼容性别名
pub mod ecs {
    pub use crate::game::GameContext;
    pub use crate::systems;
    pub use crate::components;
}
