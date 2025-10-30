// Network module - Client networking functionality
// Corresponds to: Client/MirNetwork/

// 简化网络模块
pub mod handlers;         // GameEvent 定义
pub mod builder;          // NetworkBuilder + NetContext
mod client;               // 内部实现：Network (Read + Write + 两线程)

// 导出
pub use builder::{NetworkBuilder, NetContext};
pub use handlers::GameEvent;
