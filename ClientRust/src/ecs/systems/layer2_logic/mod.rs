// ============================================================================
// Layer 2: 核心逻辑层
// ============================================================================
//
// 职责：
// - 客户端预测（LocalPrediction）
// - 物理移动（Movement）
// - 服务器校正（Reconciliation）
// - 平滑插值（Interpolation）
// - 游戏逻辑（Monster/NPC/Combat/Magic）
//
// 输入组件：
// - PlayerInputComponent（Layer 1 写入）
// - ServerStateComponent（Layer 1 写入）
//
// 输出组件：
// - MovementStateComponent（移动状态）
// - VelocityComponent（速度）
// - PathComponent（路径）
//
// ============================================================================

// 核心移动系统
pub mod local_prediction_system;
pub mod movement_system;
pub mod reconciliation_system;
pub mod interpolation_system;

// 游戏逻辑系统
pub mod monster_system;
pub mod npc_system;
pub mod combat_system;
pub mod magic_cast_system;

pub use local_prediction_system::LocalPredictionSystem;
pub use movement_system::MovementSystem as MovementSystemV2;
pub use reconciliation_system::ReconciliationSystem;
pub use interpolation_system::InterpolationSystem;
pub use monster_system::MonsterSystem;
pub use npc_system::NPCSystem;
pub use combat_system::CombatSystem;
pub use magic_cast_system::MagicCastSystem;
