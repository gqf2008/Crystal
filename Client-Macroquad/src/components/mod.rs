// ============================================================================
// ECS Components - 组件定义
// ============================================================================

// 核心组件
pub mod core;
pub mod render;
pub mod movement;
pub mod player;
pub mod map;

// 游戏逻辑组件
pub mod combat;
pub mod input;
pub mod network;
pub mod debug;
pub mod settings;

// 高级组件
pub mod actor;
pub mod character_select;
pub mod events;
pub mod item;
pub mod particle;
pub mod prediction;
pub mod quest;
pub mod sound;
pub mod spell;

// 重新导出常用组件
pub use core::*;
pub use render::*;
pub use movement::*;
pub use player::*;
pub use map::*;
pub use combat::*;
pub use input::*;
pub use network::*;
pub use debug::*;
pub use actor::*;
pub use character_select::*;
pub use events::*;
pub use item::*;
pub use particle::*;
pub use prediction::*;
pub use quest::*;
pub use sound::*;
pub use spell::*;
