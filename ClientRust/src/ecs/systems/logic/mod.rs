pub mod combat_skill;
pub mod decision;
pub mod input;
pub mod physics;
pub mod update;

// 重新导出所有系统
pub use combat_skill::*;
pub use decision::*;
pub use input::*;
pub use physics::*;
pub use update::*;

// ============================================================================
// 为所有纯逻辑系统批量实现 IntoSystemKind
// ============================================================================

crate::logic_system!(
    input::PlayerControlSystem,
    // Layer 2: Decision (200-299)
    decision::MonsterAISystem,
    decision::NpcAISystem,
    decision::NpcDialogueSystem,
    // Layer 3: Combat & Skill (300-399)
    combat_skill::SkillSystem,
    combat_skill::CombatSystem,
    // Layer 4: Physics & Movement (400-499)
    physics::MovementSystem,
    physics::CollisionSystem,
    physics::CameraFollowSystem,
    // Layer 5: State Update (500-599)
    update::AnimationSystem,
    update::TileAnimationSystem,
    update::ParticleSystem,
    update::SoundSystem,
    update::HealthRegenSystem,
    update::MapLoadSystem,
    update::MapUpdateSystem,
    update::CameraSystem,
);
