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
// | 系统 | 优先级 | 职责 | 状态 |
// |------|--------|------|------|
// | NetworkSyncSystem | 50 | 双向网络同步：接收服务器事件、发送客户端命令 | ✅ 活跃 |
// | InputSystem | 100 | 收集键盘/鼠标输入，写入 GlobalEvents | ✅ 活跃 |
// | PlayerControlSystem | 110 | 处理玩家控制逻辑（移动、攻击等） | ✅ 活跃 |
// | GameEventDispatcher | 120 | 分发游戏事件到具体处理逻辑 | ✅ 活跃 |
//
// ## 网络架构说明
//
// ### 新架构 (完全 ECS 化)
//
// ```
// ┌─────────────────────────────────────────────────────────┐
// │               NetworkSyncSystem (优先级 50)              │
// ├─────────────────────────────────────────────────────────┤
// │                                                          │
// │  📥 服务器 → 客户端 (接收)                               │
// │     NetContext.try_recv() → ServerPacket                │
// │     ↓ PacketHandlers 转换                               │
// │     GameEvent → GlobalEvents.network_incoming            │
// │                                                          │
// │  📤 客户端 → 服务器 (发送)                               │
// │     GlobalEvents.network_outgoing (Channel)              │
// │     ↓ 协议转换                                           │
// │     GameEvent → ClientPacket → NetContext.send()         │
// │                                                          │
// └─────────────────────────────────────────────────────────┘
// ```
//
// ### 与旧架构的区别
//
// **旧架构 (SelectScene/LoginScene)**:
// - Scene 直接调用 `net_ctx.try_recv()`
// - 网络事件在 Scene 中处理 (非 ECS)
// - 适用于简单 UI 场景
//
// **新架构 (GameScene)**:
// - NetworkSyncSystem 统一处理网络同步
// - 所有事件通过 GlobalEvents 传递
// - 完全 ECS 化,利用系统组合
//
// ## 输入依赖
//
// - ggez::Context (键盘/鼠标状态)
// - crossbeam_channel::Receiver<ServerPacket> (网络数据包)
//
// ## 输出组件
//
// - **GlobalEvents.keyboard_events**: 键盘事件队列 (InputSystem 写入)
// - **GlobalEvents.mouse_events**: 鼠标事件队列 (InputSystem 写入)
// - **GlobalEvents.ime_events**: IME输入事件 (InputSystem 写入)
// - **GlobalEvents.game_events**: 游戏事件队列 (各系统写入)
// - ~~**GlobalEvents.network_incoming**~~: 网络包队列 (当前未使用)
//
// ## 数据流
//
// ```
// 用户输入 (键盘/鼠标)
//     ↓
// InputSystem (优先级 100)
//     ↓
// GlobalEvents.keyboard_events / mouse_events
//     ↓
// PlayerControlSystem (优先级 110) 读取并处理
//     ↓
// 更新玩家组件 (Position, Velocity, etc.)
//     ↓
// GameEventDispatcher (优先级 120) 分发游戏事件
//     ↓
// 其他系统处理游戏逻辑
// ```
//
// ## 注意事项
//
// ❌ **NetworkSyncSystem 已废弃**: 
//    - 当前网络事件由 Scene 直接从 NetContext 读取
//    - GameScene 中的网络同步需要重新设计
//
// ✅ **GameEventDispatcher 职责明确**:
//    - 只读取 GlobalEvents.game_events
//    - 不维护内部队列
//    - 详见系统文件注释
//
// ============================================================================

// 系统模块
pub mod player_control_system; // ✅ 玩家控制系统 (优先级 110)
pub mod game_event_system;    // ✅ 游戏事件分发系统 (优先级 120)

pub use player_control_system::PlayerControlSystem;
pub use game_event_system::{GameEventDispatcher, GameEventSystem, InternalEvent};