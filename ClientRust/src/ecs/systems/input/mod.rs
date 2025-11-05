//! # 输入与网络系统模块 (input)
//!
//! **优先级范围**: 100-199
//!
//! ## 模块职责
//!
//! 负责处理玩家输入和网络通信：
//! 1. 收集键盘、鼠标、触控输入
//! 2. 将输入转换为游戏指令
//! 3. 处理网络数据包的接收与发送
//!
//! ## 系统列表
//!
//! | 系统名称 | 优先级 | 依赖组件（读） | 依赖组件（写） | 职责说明 |
//! |---------|--------|----------------|----------------|----------|
//! | `PlayerControlSystem` | 120 | InputEvent, LocalPlayer, Position | Velocity, PlayerAction | 玩家控制逻辑、指令转换 |
//!
//! ## 数据流
//!
//! ```text
//! ggez EventHandler 收集输入
//!         ↓
//! GameContext.frame_input_events (零拷贝访问)
//!         ↓
//! PlayerControlSystem: 转换为游戏指令
//!         ↓
//! 更新 Velocity/PlayerAction
//!         ↓
//! 后续逻辑系统使用（MovementSystem 等）
//! ```
//!
//! ## 使用示例
//!
//! ```rust
//! use crate::ecs::systems::input::PlayerControlSystem;
//! use crate::ecs::components::{InputEvent, LocalPlayer, Position, Velocity};
//!
//! // 创建本地玩家
//! world.spawn((
//!     Position::new(100.0, 100.0),
//!     Velocity::zero(),
//!     LocalPlayer,
//! ));
//!
//! // PlayerControlSystem 会读取 GameContext.frame_input_events
//! // 并更新玩家的 Velocity 和 PlayerAction
//! ```
//!
//! ## 输入处理最佳实践
//!
//! 1. **使用 GameContext 访问输入**：
//!    ```rust
//!    let input = ctx.input();
//!    if input.w_pressed() {
//!        // 处理 W 键
//!    }
//!    ```
//!
//! 2. **使用 InputContext 辅助器**：
//!    ```rust
//!    let input = ctx.input();
//!    let (dx, dy) = input.wasd_direction();  // 获取 WASD 方向向量
//!    ```
//!
//! 3. **避免直接访问 ggez 输入**：
//!    使用 `GameContext.frame_input_events` 获取零拷贝的输入事件列表
//!
//! ## 注意事项
//!
//! - PlayerControlSystem 必须在物理系统之前执行（优先级 120 < 500）
//! - 输入事件在每帧结束后清空（`GameContext.clear_frame_events()`）
//! - 网络事件通过 `GameContext.net_events` 访问（由 NetworkSystem 填充）

pub mod player_control_system;   
pub use player_control_system::PlayerControlSystem;