//! 更新系统模块 (Layer 1-6)
//! 
//! 所有系统都实现 System trait
//! 
//! 层级结构：
//! - Layer 1: input - 输入处理 (50-199)
//! - Layer 2: decision - 决策层 (200-299)
//! - Layer 3: combat_skill - 战斗技能 (300-399)
//! - Layer 4: physics_movement - 物理运动 (400-499)
//! - Layer 5: state_update - 状态更新 (500-599)
//! - Layer 6: network_sync - 网络同步 (600-699)

pub mod input;
pub mod decision;
pub mod combat_skill;
pub mod physics_movement;
pub mod state_update;
pub mod network_sync;
pub mod network_event_system;  // 🆕 网络事件系统

// 重新导出所有系统
pub use input::*;
pub use decision::*;
pub use combat_skill::*;
pub use physics_movement::*;
pub use state_update::*;
pub use network_sync::*;
pub use network_event_system::NetworkEventSystem;  // 🆕 导出网络事件系统