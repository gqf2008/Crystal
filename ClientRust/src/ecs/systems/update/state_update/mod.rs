pub mod animation_system;
pub mod health_regen_system;
pub mod particle_system;
pub mod sound_system;
pub mod camera_system;
pub mod map_update_system;
pub mod event_cleanup_system;  // 🆕 事件清理系统
pub mod game_event_system;     // 🆕 游戏事件处理系统

pub use animation_system::AnimationSystem;
pub use health_regen_system::HealthRegenSystem;
pub use particle_system::ParticleSystem;
pub use sound_system::SoundSystem;
pub use camera_system::CameraSystem;
pub use map_update_system::{MapUpdateSystem, MapManager};
pub use event_cleanup_system::{EventCleanupSystem, EventCollectorSystem};  // 🆕
pub use game_event_system::GameEventSystem;  // 🆕
