// Graphics module - Rendering and visual effects
// Corresponds to: Client/MirGraphics/
// 
// C# 原版只有 3 个文件:
// - DXManager.cs (Direct3D9 管理)
// - MLibrary.cs (图像库)
// - ParticleEngine.cs (粒子引擎)

// === ggez 渲染系统 (新) ===
pub mod ggez_manager_simple;       // ggez 渲染管理器 (简化版,推荐)
// #[allow(dead_code)]
// pub mod ggez_manager;              // ggez 渲染管理器 (完整版,待修复 - 暂时禁用)

// === wgpu 渲染系统 (旧,已废弃) ===
// 以下模块依赖 wgpu/winit/bytemuck,已禁用
// pub mod dx_manager;                // 对应 DXManager.cs
// pub mod sprite_renderer;           // 对应 SlimDX.Sprite (wgpu 实现 - 批处理模式)
// pub mod sprite_instanced_renderer; // 对应 SlimDX.Sprite (wgpu 实现 - GPU实例化模式)

// === 核心模块 ===
pub mod mlibrary;                  // 对应 MLibrary.cs
// pub mod particle_engine;           // 对应 ParticleEngine.cs - 暂时禁用(依赖 dx_manager)
// pub mod particles;                 // 对应 Client/MirGraphics/Particles/ - 暂时禁用(依赖 dx_manager)
pub mod libraries;                 // 对应 Libraries static class

// === 库管理导出 ===
pub use libraries::{
    LibraryName, LibraryArray, Libraries, LIBRARIES,
    get_library, get_library_from_array, get_map_library,
    initialize_all_libraries,
    load_library,
    set_data_path,
    load_core_libraries,
    load_all_libraries,
    is_library_loaded,
};

// === ggez 导出 (推荐使用) ===
pub use ggez_manager_simple::GgezManager;
pub use ggez_manager_simple::{Canvas, DrawParam, Color, Rect, Text, Mesh, DrawMode};

// === wgpu 导出 (已废弃,已禁用) ===
// pub use dx_manager::{DXManager, TextureHandle, BlendMode};
// pub use sprite_renderer::{SpriteRenderer, SpriteVertex, create_sprite_vertices};
// pub use sprite_instanced_renderer::{SpriteInstancedRenderer, SpriteInstance, QuadVertex};

// === 核心导出 ===
pub use mlibrary::{MLibrary, TextureManager, ImageInfo, TextureKey};
// pub use particle_engine::{ParticleEngine, ParticleType, ParticleImageInfo, get_time};
// pub use particles::Particle;
