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
//! | `InputStateSystem` | (建议帧末尾) | - | InputState | 追踪上一帧输入状态，提供边缘检测基础 |
//! | `PlayerControlSystem` | 120 | InputEvent, LocalPlayer, Position | Velocity, PlayerAction | 玩家控制逻辑、指令转换 |
//!
//! ## 数据流
//!
//! ```text
//! macroquad 轮询输入（`GameContext.input()`）
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
//! use crate::systems::input::PlayerControlSystem;
//! use crate::components::{InputEvent, LocalPlayer, Position, Velocity};
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
//! 3. **避免散落地直接访问 macroquad 全局输入**：
//!    统一通过 `GameContext.input()`（便于后续替换/录制回放）
//!
//! ## 注意事项
//!
//! - PlayerControlSystem 必须在物理系统之前执行（优先级 120 < 500）
//! - `InputStateSystem` 需要在“读取输入的系统之后”执行，才可以把“本帧状态”保存为“下一帧的 prev 状态”

pub mod input_state_system;
pub mod auto_potion_system;
pub mod local_player_ai_system;
pub mod player_control_system;

pub use input_state_system::InputStateSystem;   
pub use auto_potion_system::AutoPotionSystem;
pub use local_player_ai_system::LocalPlayerAiSystem;
pub use player_control_system::PlayerControlSystem;