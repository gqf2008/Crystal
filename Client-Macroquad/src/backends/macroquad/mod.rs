// ============================================================================
// Macroquad Backend - macroquad 渲染后端
// ============================================================================

pub mod animation;
pub mod font;
pub mod graphics;
pub mod mesh_map_renderer;
// pub mod renderer;  // 暂时禁用，依赖抽象 Renderer trait
pub mod sprite;

pub use animation::{AnimationMode, AnimationStateMachine, FrameAnimation};
pub use font::{FontData, FontManager, FontSize, TextAlign, TextBuilder};
pub use graphics::{Libraries, LibraryArray, LibraryName, MLibrary};
pub use mesh_map_renderer::MeshMapRenderer;
// pub use renderer::MacroquadRenderer;
pub use sprite::{BatchStats as SpriteBatchStats, CacheStats, SpriteBatch, SpriteData, SpriteManager};
