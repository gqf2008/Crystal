// ============================================================================
// Layer 1: 输入与网络层
// ============================================================================
//
// 职责：
// - 捕获原始输入（鼠标、键盘）
// - 接收网络数据包
// - 转换为游戏命令
//
// 输出组件：
// - PlayerInputComponent（玩家输入意图）
// - ServerStateComponent（服务器权威状态）
//
// ============================================================================

pub mod input_collecting_system;
pub mod client_network_system;

pub use input_collecting_system::InputCollectingSystem;
pub use client_network_system::ClientNetworkSystem;
