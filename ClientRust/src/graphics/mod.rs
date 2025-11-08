// Graphics module - Rendering and visual effects
// Corresponds to: Client/MirGraphics/
//
// C# 原版只有 3 个文件:
// - DXManager.cs (Direct3D9 管理)
// - MLibrary.cs (图像库)
// - ParticleEngine.cs (粒子引擎)
//
// 注意：此模块包含 ggez 特定的代码，正在逐步迁移到 backends/ggez/
// 纯数据资源加载已移至 resources/ 模块

// libraries 模块依赖 MLibrary，只在 ggez 后端可用
#[cfg(feature = "backend-ggez")]
pub mod libraries; // 对应 Libraries static class

#[cfg(feature = "backend-ggez")]
pub mod mlibrary; // 对应 MLibrary.cs (ggez 版本，依赖 objects::frames)

// === 库管理导出 ===
#[cfg(feature = "backend-ggez")]
pub use libraries::{
    get_all_map_libraries, get_library, get_library_from_array, get_map_library,
    initialize_all_libraries, is_library_loaded, load_all_libraries, load_core_libraries,
    load_library, set_data_path, Libraries, LibraryArray, LibraryName, LIBRARIES,
};

// === 核心导出 ===
#[cfg(feature = "backend-ggez")]
pub use mlibrary::{ImageInfo, MLibrary};

// ========== ggez 特定的绘制函数 ==========
// 只在使用 ggez 后端时编译这些函数
#[cfg(feature = "backend-ggez")]
pub mod ggez_helpers {
    use super::*;

    /// 简单的精灵绘制辅助函数
    pub fn draw_sprite_at(
        ctx: &mut ggez::graphics::GraphicsContext,
        canvas: &mut ggez::graphics::Canvas,
        library_name: &LibraryName,
        index: i32,
        x: f32,
        y: f32,
    ) -> anyhow::Result<()> {
        use ggez::graphics::DrawParam;

        if let Some(library) = get_library(library_name.clone()) {
            let mut lib = library
                .try_lock().ok_or_else(|| anyhow::anyhow!("Failed to lock library"))?;

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
        ctx: &mut ggez::graphics::GraphicsContext,
        canvas: &mut ggez::graphics::Canvas,
        library_name: &LibraryName,
        index: i32,
        x: f32,
        y: f32,
    ) -> anyhow::Result<()> {
        use ggez::graphics::DrawParam;

        if let Some(library) = get_library(library_name.clone()) {
            let mut lib = library
                .try_lock().ok_or_else(|| anyhow::anyhow!("Failed to lock library"))?;

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
        ctx: &mut ggez::graphics::GraphicsContext,
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
            let mut lib = library
                .try_lock().ok_or_else(|| anyhow::anyhow!("Failed to lock library"))?;

            if let Ok(image_info) = lib.get_or_create_texture(ctx, index as usize) {
                if let Some(image) = &image_info.image {
                    canvas.draw(
                        image,
                        DrawParam::default().dest([x, y]).scale([scale_x, scale_y]),
                    );
                }
            }
        }
        Ok(())
    }

    /// 带完整参数的精灵绘制（位置、偏移、缩放、旋转、颜色）
    pub fn draw_sprite_full(
        ctx: &mut ggez::graphics::GraphicsContext,
        canvas: &mut ggez::graphics::Canvas,
        library_name: &LibraryName,
        index: i32,
        x: f32,
        y: f32,
        scale_x: f32,
        scale_y: f32,
        color: ggez::graphics::Color,
    ) -> anyhow::Result<()> {
        use ggez::graphics::DrawParam;

        if let Some(library) = get_library(library_name.clone()) {
            let mut lib = library
                .try_lock().ok_or_else(|| anyhow::anyhow!("Failed to lock library"))?;

            if let Ok(image_info) = lib.get_or_create_texture(ctx, index as usize) {
                if let Some(image) = &image_info.image {
                    let offset_x = x + image_info.x as f32;
                    let offset_y = y + image_info.y as f32;
                    canvas.draw(
                        image,
                        DrawParam::default()
                            .dest([offset_x, offset_y])
                            .scale([scale_x, scale_y])
                            .color(color),
                    );
                }
            }
        }
        Ok(())
    }
}

// 为了向后兼容，在 ggez 后端时重导出这些函数
#[cfg(feature = "backend-ggez")]
pub use ggez_helpers::*;

/// 带偏移量的精灵绘制辅助函数（使用纹理自带的偏移量）
/// UseOffSet = true 时使用此函数
#[cfg(feature = "backend-ggez")]
pub fn draw_sprite_with_offset(
    ctx: &mut ggez::graphics::GraphicsContext,
    canvas: &mut ggez::graphics::Canvas,
    library_name: &LibraryName,
    index: i32,
    x: f32,
    y: f32,
) -> anyhow::Result<()> {
    use ggez::graphics::DrawParam;

    if let Some(library) = get_library(library_name.clone()) {
        let mut lib = library
            .try_lock().ok_or_else(|| anyhow::anyhow!("Failed to lock library"))?;

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
#[cfg(feature = "backend-ggez")]
pub fn draw_sprite_scaled(
    ctx: &mut ggez::graphics::GraphicsContext,
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
        let mut lib = library
            .try_lock().ok_or_else(|| anyhow::anyhow!("Failed to lock library"))?;

        if let Ok(image_info) = lib.get_or_create_texture(ctx, index as usize) {
            if let Some(image) = &image_info.image {
                canvas.draw(
                    image,
                    DrawParam::default().dest([x, y]).scale([scale_x, scale_y]),
                );
            }
        }
    }
    Ok(())
}

/// 带混合效果的精灵绘制辅助函数（对应 C# 的 DrawBlend）
/// 使用 alpha blending 和指定的混合率
#[cfg(feature = "backend-ggez")]
pub fn draw_sprite_blend(
    ctx: &mut ggez::graphics::GraphicsContext,
    canvas: &mut ggez::graphics::Canvas,
    library_name: &LibraryName,
    index: i32,
    x: f32,
    y: f32,
    color: ggez::graphics::Color,
    use_offset: bool,
    rate: f32,
) -> anyhow::Result<()> {
    if let Some(library) = get_library(library_name.clone()) {
        let mut lib = library
            .try_lock().ok_or_else(|| anyhow::anyhow!("Failed to lock library"))?;

        lib.draw_blend(ctx, canvas, index as usize, x, y, color, use_offset, rate)?;
    }
    Ok(())
}
