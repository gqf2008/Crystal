//! Layer 1: 输入与网络层 (50-199)
//! 
//! 所有系统都实现 System trait
//! 
//! 优先级顺序：
//! - NetworkRecvSystem(50) - 网络数据接收
//! - InputSystem(100) - 输入收集与处理
//! - PlayerControlSystem(110) - 玩家控制转换
//! - GameEventSystem(120) - 游戏事件分发

pub mod network_recv_system;
pub mod input_system;
pub mod player_control_system;
pub mod game_event_system;


pub use network_recv_system::NetworkRecvSystem;
pub use input_system::InputSystem;
pub use player_control_system::PlayerControlSystem;
pub use game_event_system::{GameEventSystem, InternalEvent};