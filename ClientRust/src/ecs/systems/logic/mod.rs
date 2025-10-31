pub mod combat_skill;
pub mod decision;
pub mod event_cleanup_system;
pub mod input;
pub mod physics_movement;
pub mod state_update; // 🧹 事件清理系统

// 重新导出所有系统
pub use combat_skill::*;
pub use decision::*;
pub use event_cleanup_system::EventCleanupSystem;
pub use input::*;
pub use physics_movement::*;
pub use state_update::*;

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

    // Layer 6: Event Cleanup (900)
    EventCleanupSystem,
);
