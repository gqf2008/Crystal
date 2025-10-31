// ============================================================================
// Layer 4: 物理与移动层 (Physics & Movement Layer)
// 优先级范围: 400-499
// ============================================================================
//
// ## 模块职责
//
// 负责游戏中所有实体的物理运动和碰撞检测：
// 1. 应用速度到位置（纯物理运动）
// 2. 碰撞检测与响应
// 3. 相机跟随玩家
//
// ## 设计原则
//
// - **纯物理**: 只处理物理运动，不包含游戏逻辑（寻路、AI在Layer 2）
// - **读写分离**: 读取 Velocity，写入 Position
// - **碰撞响应**: 检测碰撞后修正位置
//
// ## 系统列表
//
// | 系统 | 优先级 | 职责 |
// |------|--------|------|
// | MovementSystem | 400 | 纯物理移动：Position += Velocity * dt |
// | CollisionSystem | 410 | 碰撞检测与位置修正 |
// | CameraFollowSystem | 420 | 相机跟随玩家移动 |
//
// ## 输入组件
//
// - **Velocity**: 速度向量（由 Layer 2 计算）
// - **Position**: 当前位置
// - **Collider**: 碰撞体组件
// - **Player**: 玩家标记（相机跟随目标）
//
// ## 输出组件
//
// - **Position**: 更新后的位置
// - **Camera.position**: 相机位置（跟随玩家）
//
// ## 数据流
//
// ```
// Layer 2 计算 Velocity
//         ↓
// MovementSystem: Position += Velocity * dt
//         ↓
// CollisionSystem: 检测碰撞 → 修正 Position
//         ↓
// CameraFollowSystem: Camera.position = Player.position
//         ↓
// Layer 5 使用更新后的 Position
// ```
//
// ## 注意事项
//
// ⚠️ **CameraFollowSystem vs CameraSystem**: 
//    - CameraFollowSystem (Layer 4): 更新相机位置（跟随逻辑）
//    - CameraSystem (Layer 5): 相机渲染配置（边缘滚动、缩放）
//    - 建议合并为一个系统（见 ARCHITECTURE_REVIEW.md）
//
// ============================================================================

pub mod movement_system;
pub mod collision_system;
pub mod camera_follow_system;

pub use movement_system::MovementSystem;
pub use collision_system::CollisionSystem;
pub use camera_follow_system::CameraFollowSystem;
