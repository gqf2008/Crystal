//! Protocol Packets Module
//!
//! This module contains all server packet definitions organized by system.

// Sub-modules
pub mod account;
pub mod buff; // 新增: Buff/状态系统
pub mod chat; // 新增: 聊天系统
pub mod combat; // 新增: 战斗系统
pub mod group;
pub mod guild;
pub mod hero;
pub mod item;
pub mod magic;
pub mod map; // 新增: 地图系统
pub mod npc;
pub mod object;
pub mod player;
pub mod quest;
pub mod trade; // 新增: 交易系统

// Re-export all packet types for convenience
pub use account::*;
pub use buff::*; // 新增
pub use chat::*; // 新增
pub use combat::*; // 新增
pub use group::*;
pub use guild::*;
pub use hero::*;
pub use item::*;
pub use magic::*;
pub use map::*; // 新增
pub use npc::*;
pub use object::*;
pub use player::*;
pub use quest::*;
pub use trade::*; // 新增
