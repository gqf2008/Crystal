//! 更新系统模块 (Layer 1-6)
//! 
//! 所有系统都实现 System trait
//! 
//! 层级结构：
//! - Layer 1: input - 输入处理 (50-199)
//! - Layer 2: decision - 决策层 (200-299)
//! - Layer 3: combat_skill - 战斗技能 (300-399)
//! - Layer 4: physics_movement - 物理运动 (400-499)
//! - Layer 5: state_update - 状态更新 (500-599)
//! - Layer 6: network_sync - 网络同步 (600-699)

pub mod input;
pub mod decision;
pub mod combat_skill;
pub mod physics_movement;
pub mod state_update;
pub mod network_sync;
pub mod network_event_system;  // 🆕 网络事件系统
pub mod event_cleanup_system;  // 🆕 事件清理系统

// 重新导出所有系统
pub use input::*;
pub use decision::*;
pub use combat_skill::*;
pub use physics_movement::*;
pub use state_update::*;
pub use network_sync::*;
pub use network_event_system::NetworkEventSystem;  // 🆕 导出网络事件系统
pub use event_cleanup_system::EventCleanupSystem;  // 🆕 导出事件清理系统

// ============================================================================
// 为所有纯逻辑系统批量实现 IntoSystemKind
// ============================================================================

crate::logic_system!(
    // Layer 1: Input (50-199)
    input::InputSystem,
    input::PlayerControlSystem,
    
    // Layer 2: Decision (200-299)
    decision::MonsterAISystem,
    decision::NpcAISystem,
    decision::NpcDialogueSystem,
    
    // Layer 3: Combat & Skill (300-399)
    combat_skill::SkillSystem,
    combat_skill::CombatSystem,
    
    // Layer 4: Physics & Movement (400-499)
    physics_movement::MovementSystem,
    physics_movement::CollisionSystem,
    physics_movement::CameraFollowSystem,
    
    // Layer 5: State Update (500-599)
    state_update::AnimationSystem,
    state_update::ParticleSystem,
    state_update::SoundSystem,
    state_update::CameraSystem,
    state_update::HealthRegenSystem,
    // state_update::MapUpdateSystem,  // ⚠️ 尚未实现 System trait
    
    // Layer 6: Network Sync (600-699)
    network_sync::NetworkSendSystem,
    network_sync::SyncSystem,
    network_sync::ClientPredictionSystem,
    
    // Special Systems
    // NetworkEventSystem,  // ⚠️ 尚未实现 System trait
    EventCleanupSystem,
);