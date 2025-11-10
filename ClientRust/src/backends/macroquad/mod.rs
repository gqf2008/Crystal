// ============================================================================
// Macroquad Backend - macroquad 渲染后端
// ============================================================================

pub mod animation;
pub mod font;
pub mod graphics;
pub mod map_renderer;
pub mod mesh_map_renderer;
pub mod renderer;
pub mod sprite;

pub use animation::{AnimationMode, AnimationStateMachine, FrameAnimation};
pub use font::{FontData, FontManager, FontSize, TextAlign, TextBuilder};
pub use graphics::LibraryManager;
pub use map_renderer::MapRenderer;
pub use mesh_map_renderer::MeshMapRenderer;
pub use renderer::MacroquadRenderer;
pub use sprite::{BatchStats as SpriteBatchStats, CacheStats, SpriteBatch, SpriteData, SpriteManager};
