//! # 物理与移动系统模块 (logic/physics)
//!
//! **优先级范围**: 500-599
//!
//! ## 模块职责
//!
//! 负责游戏中所有实体的物理运动和碰撞检测：
//! 1. 应用速度到位置（纯物理运动）
//! 2. 碰撞检测与响应
//! 3. 地图更新（瓦片动画、地图切换）
//! 4. 寻路计算
//!
//! ## 设计原则
//!
//! - **纯物理**: 只处理物理运动，不包含游戏逻辑（寻路、AI 在 decision 层）
//! - **读写分离**: 读取 Velocity，写入 Position
//! - **碰撞响应**: 检测碰撞后修正位置
//!
//! ## 系统列表
//!
//! | 系统 | 优先级 | 依赖组件（读） | 依赖组件（写） | 职责 |
//! |------|--------|----------------|----------------|------|
//! | PathfindingSystem | 520 | Position, PathTarget | Path | 寻路计算 |
//! | MovementSystem | 500 | Velocity, Position | Position | 纯物理移动：Position += Velocity * dt |
//! | CollisionSystem | 510 | Position, Collider, MapData | Position | 碰撞检测与位置修正 |
//! | MapUpdateSystem | 500 | MapData, AnimatedTile | AnimatedTile | 地图瓦片动画更新 |
//!
//! ## 数据流
//!
//! ```text
//! Layer 2 (decision) 计算 Velocity
//!         ↓
//! PathfindingSystem: 计算 Path
//!         ↓
//! MovementSystem: Position += Velocity * dt
//!         ↓
//! CollisionSystem: 检测碰撞 → 修正 Position
//!         ↓
//! MapUpdateSystem: 更新地图瓦片动画
//!         ↓
//! Layer 3 (presentation) 使用更新后的 Position
//! ```
//!
//! ## 使用示例
//!
//! ```rust
//! use crate::systems::logic::physics::{MovementSystem, CollisionSystem};
//! use crate::components::{Position, Velocity};
//!
//! // 创建移动实体
//! world.spawn((
//!     Position::new(100.0, 100.0),
//!     Velocity::new(50.0, 0.0),  // 向右移动
//! ));
//!
//! // MovementSystem 会自动更新位置
//! // CollisionSystem 会检测并修正碰撞
//! ```
//!
//! ## 注意事项
//!
//! - MovementSystem 必须在 CollisionSystem 之前执行（通过优先级保证）
//! - MapUpdateSystem 是独立的，可以与其他系统并行
//! - PathfindingSystem 的计算结果会被 AI 系统使用
// ============================================================================

pub mod collision_system;
pub mod map_load_system;
pub mod map_update_system;
pub mod movement_system;
pub mod pathfinding_system;
pub use collision_system::CollisionSystem;
pub use map_load_system::{MapLoadSystem, MapManager};
pub use map_update_system::{MapSwitchRequest, MapUpdateSystem};
pub use movement_system::MovementSystem;
pub use pathfinding_system::PathfindingSystem;
