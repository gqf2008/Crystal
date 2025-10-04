// Graphics module - Rendering and visual effects
// Corresponds to: Client/MirGraphics/

pub mod dx_manager;        // NEW: Phase 2 - DXManager (对应 DXManager.cs)
pub mod sprite_pipeline;   // NEW: Phase 2 - SpritePipeline (对应 Sprite)
pub mod texture_loader;
pub mod sprite_renderer;
pub mod character_renderer;

pub use dx_manager::{DXManager, TextureHandle, BlendMode};
pub use sprite_pipeline::{SpritePipeline, SpriteVertex};
pub use texture_loader::{MLibrary, TextureManager, ImageInfo, TextureKey};
pub use sprite_renderer::{SpriteRenderer, SpriteInstance};
pub use character_renderer::{CharacterRenderer, CharacterAppearance};

// TODO: Add graphics modules as they are ported
// pub mod animation;
// pub mod effect;
