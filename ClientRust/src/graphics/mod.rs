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
pub use mlibrary::{MLibrary, ImageInfo};
// get_library, get_map_library 已在上面的 libraries 导出中定义
pub use libraries::get_all_map_libraries;
// pub use particle_engine::{ParticleEngine, ParticleType, ParticleImageInfo, get_time};
// pub use particles::Particle;

// === 辅助函数 ===
// draw_sprite_at, draw_sprite_with_offset, draw_sprite_scaled 在下面定义

/// 简单的精灵绘制辅助函数
pub fn draw_sprite_at(
    ctx: &mut ggez::Context,
    canvas: &mut ggez::graphics::Canvas,
    library_name: &LibraryName,
    index: i32,
    x: f32,
    y: f32,
) -> anyhow::Result<()> {
    use ggez::graphics::DrawParam;
    
    if let Some(library) = get_library(library_name.clone()) {
        let mut lib = library.try_lock()
            .map_err(|e| anyhow::anyhow!("Failed to lock library: {}", e))?;
        
        if let Ok(image_info) = lib.get_or_create_texture(ctx, index as usize) {
            if let Some(image) = &image_info.image {
                canvas.draw(image, DrawParam::default().dest([x, y]));
            }
        }
    }
    Ok(())
}

/// 带偏移量的精灵绘制辅助函数（使用纹理自带的偏移量）
/// UseOffSet = true 时使用此函数
pub fn draw_sprite_with_offset(
    ctx: &mut ggez::Context,
    canvas: &mut ggez::graphics::Canvas,
    library_name: &LibraryName,
    index: i32,
    x: f32,
    y: f32,
) -> anyhow::Result<()> {
    use ggez::graphics::DrawParam;
    
    if let Some(library) = get_library(library_name.clone()) {
        let mut lib = library.try_lock()
            .map_err(|e| anyhow::anyhow!("Failed to lock library: {}", e))?;
        
        if let Ok(image_info) = lib.get_or_create_texture(ctx, index as usize) {
            if let Some(image) = &image_info.image {
                // 应用纹理偏移量
                let offset_x = x + image_info.x as f32;
                let offset_y = y + image_info.y as f32;
                canvas.draw(image, DrawParam::default().dest([offset_x, offset_y]));
            }
        }
    }
    Ok(())
}

/// 带缩放的精灵绘制辅助函数
pub fn draw_sprite_scaled(
    ctx: &mut ggez::Context,
    canvas: &mut ggez::graphics::Canvas,
    library_name: &LibraryName,
    index: i32,
    x: f32,
    y: f32,
    scale_x: f32,
    scale_y: f32,
) -> anyhow::Result<()> {
    use ggez::graphics::DrawParam;
    
    if let Some(library) = get_library(library_name.clone()) {
        let mut lib = library.try_lock()
            .map_err(|e| anyhow::anyhow!("Failed to lock library: {}", e))?;
        
        if let Ok(image_info) = lib.get_or_create_texture(ctx, index as usize) {
            if let Some(image) = &image_info.image {
                canvas.draw(
                    image, 
                    DrawParam::default()
                        .dest([x, y])
                        .scale([scale_x, scale_y])
                );
            }
        }
    }
    Ok(())
}
