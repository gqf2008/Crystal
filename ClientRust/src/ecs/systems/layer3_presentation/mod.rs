// ============================================================================
// Layer 3: 表现状态层
// ============================================================================
//
// 职责：
// - 动画状态决策（根据移动状态决定播放什么动画）
// - 音效触发决策（根据游戏事件决定播放什么音效）
// - 怪物动画决策（根据怪物AI状态决定动画）
// - 粒子特效创建（未来）
//
// 输入组件：
// - MovementStateComponent（Layer 2 写入）
// - Player（方向、武器等）
// - GameEvent（事件列表）
// - AIAction（怪物AI状态）
// - Velocity（移动速度）
//
// 输出组件：
// - AnimationStateComponent（动画状态）
// - SoundTriggerComponent（音效触发）
// - Animation（怪物动画）
// - ParticleEmitterComponent（未来）
//
// ============================================================================

pub mod animation_state_system;
pub mod npc_action_system;
pub mod sound_trigger_system;
pub mod monster_animation_state_system;

pub use animation_state_system::AnimationStateSystem;
pub use npc_action_system::NPCActionSystem;
pub use sound_trigger_system::SoundTriggerSystem;
pub use monster_animation_state_system::MonsterAnimationStateSystem;
