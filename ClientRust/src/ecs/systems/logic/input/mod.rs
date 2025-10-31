// ============================================================================
// Layer 1: 输入与网络层 (Input & Network Layer)
// 优先级范围: 50-199
// ============================================================================
//
// ## 模块职责
//
// 负责游戏的最底层数据采集，包括：
// 1. 捕获用户输入（键盘、鼠标）
// 2. 接收网络数据包（从网络线程）
// 3. 转换原始输入为游戏事件
// 4. 写入 GlobalEvents 组件，供上层系统读取
//
// ## 设计原则
//
// - **只写不读**: Layer 1 系统只写入 GlobalEvents，不读取其他组件
// - **数据采集**: 不处理游戏逻辑，只负责数据收集
// - **最高优先级**: 确保在所有逻辑系统之前执行
//
// ## 系统列表
//
// | 系统 | 优先级 | 职责 |
// |------|--------|------|
// | NetworkSyncSystem | 50 | 从网络线程接收数据包，写入 GlobalEvents.network_incoming |
// | InputSystem | 100 | 收集键盘/鼠标输入，写入 GlobalEvents.keyboard_events/mouse_events |
// | PlayerControlSystem | 110 | 处理玩家控制逻辑（移动、攻击等） |
// | GameEventSystem | 120 | 分发游戏事件到具体处理逻辑 |
//
// ## 输入依赖
//
// - ggez::Context (键盘/鼠标状态)
// - crossbeam_channel::Receiver<ServerPacket> (网络数据包)
//
// ## 输出组件
//
// - **GlobalEvents.keyboard_events**: 键盘事件队列
// - **GlobalEvents.mouse_events**: 鼠标事件队列
// - **GlobalEvents.ime_events**: IME输入事件
// - **GlobalEvents.network_incoming**: 网络数据包队列
// - **GlobalEvents.game_events**: 游戏事件队列
//
// ## 数据流
//
// ```
// 用户输入 → InputSystem → GlobalEvents.keyboard_events
//                                     ↓
//                                Layer 2 系统读取并处理
//
// 网络线程 → NetworkSyncSystem → GlobalEvents.network_incoming
//                                        ↓
//                                   网络事件处理系统
// ```
//
// ## 注意事项
//
// ⚠️ **NetworkSyncSystem 暂时禁用**: 依赖旧的 network::protocol，需要重构后启用
// ⚠️ **GameEventSystem 职责**: 需要明确与 GlobalEvents 的职责边界（见 ARCHITECTURE_REVIEW.md）
//
// ============================================================================

// TODO: network_sync_system_v2 依赖旧的network::protocol，暂时禁用
// pub mod network_sync_system_v2;  // 🆕 新版本
pub mod input_system;
pub mod player_control_system;
pub mod game_event_system;


// pub use network_sync_system_v2::NetworkSyncSystem;  // 🆕 导出新版本
pub use input_system::InputSystem;
pub use player_control_system::PlayerControlSystem;
pub use game_event_system::{GameEventSystem, InternalEvent};