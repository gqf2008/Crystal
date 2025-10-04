// Graphics module - Rendering and visual effects
// Corresponds to: Client/MirGraphics/
// 
// C# 原版只有 3 个文件:
// - DXManager.cs (Direct3D9 管理)
// - MLibrary.cs (图像库)
// - ParticleEngine.cs (粒子引擎)

pub mod dx_manager;        // 对应 DXManager.cs
pub mod mlibrary;          // 对应 MLibrary.cs
pub mod sprite_renderer;   // 对应 SlimDX.Sprite (wgpu 实现)
pub mod particle_engine;   // 对应 ParticleEngine.cs
pub mod particles;         // 对应 Client/MirGraphics/Particles/

pub use dx_manager::{DXManager, TextureHandle, BlendMode};
pub use mlibrary::{MLibrary, TextureManager, ImageInfo, TextureKey};
pub use sprite_renderer::{SpriteRenderer, SpriteVertex, create_sprite_vertices};
pub use particle_engine::{ParticleEngine, ParticleType, ParticleImageInfo, get_time};
pub use particles::{Particle, ParticleTrait, FogParticle};
