// Graphics module - Rendering and visual effects
// Corresponds to: Client/MirGraphics/
// 
// C# 原版只有 3 个文件:
// - DXManager.cs (Direct3D9 管理)
// - MLibrary.cs (图像库)
// - ParticleEngine.cs (粒子引擎)

pub mod dx_manager;        // 对应 DXManager.cs
pub mod texture_loader;    // 对应 MLibrary.cs (改名为 texture_loader 更清晰)

pub use dx_manager::DXManager;
pub use texture_loader::{MLibrary, TextureManager, ImageInfo, TextureKey};

// TODO: 按需添加
// pub mod particle_engine;  // 对应 ParticleEngine.cs
