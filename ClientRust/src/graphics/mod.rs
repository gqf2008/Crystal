// Graphics module - Rendering and visual effects
// Corresponds to: Client/MirGraphics/

pub mod texture_loader;
pub mod sprite_renderer;
pub mod character_renderer;

pub use texture_loader::{MLibrary, TextureManager, ImageInfo, TextureKey};
pub use sprite_renderer::{SpriteRenderer, SpriteVertex, SpriteInstance};
pub use character_renderer::{CharacterRenderer, CharacterAppearance};

// TODO: Add graphics modules as they are ported
// pub mod animation;
// pub mod effect;
