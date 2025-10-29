//! Layer 7: 渲染层 (1000-1999)
//! 
//! 所有系统都实现 DrawSystem trait
//! 优先级范围：1000-1999
//! 
//! 执行顺序（从底层到顶层）：
//! - MapRenderSystem(1000) - 地图渲染
//! - SpriteRenderSystem(1010) - 精灵实体渲染
//! - EffectRenderSystem(1020) - 特效渲染
//! - UIRenderSystem(1030) - UI界面渲染
//! - DebugSystem(1100) - 调试信息（混合系统：System + DrawSystem）

pub mod map_system;
pub mod sprite_system;
pub mod effect_system;
pub mod ui_system;
pub mod debug_system;

pub use map_system::MapRenderSystem;
pub use sprite_system::SpriteRenderSystem;
pub use effect_system::EffectRenderSystem;
pub use ui_system::UIRenderSystem;
pub use debug_system::DebugSystem;