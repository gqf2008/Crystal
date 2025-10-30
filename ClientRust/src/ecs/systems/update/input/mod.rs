//! Layer 1: 输入与网络层 (50-199)
//! 
//! 所有系统都实现 System trait
//! 
//! 优先级顺序：
//! - NetworkSyncSystem(50) - 网络数据包同步到 GlobalEvents
//! - InputSystem(100) - 输入收集写入 GlobalEvents
//! - PlayerControlSystem(110) - 从 GlobalEvents 读取并处理玩家控制
//! - GameEventSystem(120) - 游戏事件分发

pub mod network_sync_system_v2;  // 🆕 新版本
pub mod input_system;
pub mod player_control_system;
pub mod game_event_system;


pub use network_sync_system_v2::NetworkSyncSystem;  // 🆕 导出新版本
pub use input_system::InputSystem;
pub use player_control_system::PlayerControlSystem;
pub use game_event_system::{GameEventSystem, InternalEvent};