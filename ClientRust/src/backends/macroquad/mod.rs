// ============================================================================
// Macroquad Backend - macroquad 渲染后端
// ============================================================================

pub mod animation;
pub mod font;
pub mod renderer;
pub mod sprite;

pub use animation::{AnimationMode, AnimationStateMachine, FrameAnimation};
pub use font::{FontData, FontManager, FontSize, TextAlign, TextBuilder};
pub use renderer::MacroquadRenderer;
pub use sprite::{BatchStats, CacheStats, SpriteBatch, SpriteData, SpriteManager};
