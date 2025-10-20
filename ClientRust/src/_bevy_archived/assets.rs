// Bevy Assets - MLibrary 资源加载系统
use bevy::prelude::*;
use bevy::render::render_resource::{Extent3d, TextureDimension, TextureFormat};

use crate::graphics::libraries::{self, LibraryName, LibraryArray, get_library, get_library_from_array};

/// MLibrary 资源容器 (Bevy Resource)
/// 
/// 使用全局 libraries 系统,不再维护自己的加载器
#[derive(Resource)]
pub struct MLibraryResource {
    pub loaded: bool,  // 是否已加载核心库
}

impl MLibraryResource {
    pub fn new() -> Self {
        Self {
            loaded: false,
        }
    }
    
    /// 从全局库系统获取图像并转换为 Bevy Image
    pub fn get_bevy_image(
        lib_name: LibraryName,
        image_index: usize,
    ) -> Option<Image> {
        let lib_arc = get_library(lib_name)?;
        let mut lib = lib_arc.lock().unwrap();
        
        // 获取图像信息和 BGRA 数据
        let (info, bgra_data) = lib.get_image_with_data(image_index).ok()?;
        
        Some(Image {
            data: Some(bgra_data),
            texture_descriptor: bevy::render::render_resource::TextureDescriptor {
                label: None,
                size: Extent3d {
                    width: info.width as u32,
                    height: info.height as u32,
                    depth_or_array_layers: 1,
                },
                mip_level_count: 1,
                sample_count: 1,
                dimension: TextureDimension::D2,
                format: TextureFormat::Bgra8UnormSrgb,
                usage: bevy::render::render_resource::TextureUsages::TEXTURE_BINDING
                    | bevy::render::render_resource::TextureUsages::COPY_DST,
                view_formats: &[],
            },
            ..default()
        })
    }
    
    /// 从数组库获取图像
    pub fn get_bevy_image_from_array(
        array_type: LibraryArray,
        array_index: usize,
        image_index: usize,
    ) -> Option<Image> {
        let lib_arc = get_library_from_array(array_type, array_index)?;
        let mut lib = lib_arc.lock().unwrap();
        
        // 获取图像信息和 BGRA 数据
        let (info, bgra_data) = lib.get_image_with_data(image_index).ok()?;
        
        Some(Image {
            data: Some(bgra_data),
            texture_descriptor: bevy::render::render_resource::TextureDescriptor {
                label: None,
                size: Extent3d {
                    width: info.width as u32,
                    height: info.height as u32,
                    depth_or_array_layers: 1,
                },
                mip_level_count: 1,
                sample_count: 1,
                dimension: TextureDimension::D2,
                format: TextureFormat::Bgra8UnormSrgb,
                usage: bevy::render::render_resource::TextureUsages::TEXTURE_BINDING
                    | bevy::render::render_resource::TextureUsages::COPY_DST,
                view_formats: &[],
            },
            ..default()
        })
    }
}

/// 启动时加载 MLibrary 资源
/// 
/// 使用全局 libraries 系统加载核心库
pub fn load_mlibrary_system(
    mut mlibrary_res: ResMut<MLibraryResource>,
) {
    println!("🔄 开始加载核心图形库...");
    
    // 使用全局库管理器加载核心库
    match libraries::load_core_libraries() {
        Ok(()) => {
            mlibrary_res.loaded = true;
            println!("✅ 核心图形库加载完成!");
            println!("   📦 包含: ChrSel, Title, Prguse, Prguse2, Magic, Magic2,");
            println!("           Weather, Effect, Items, MagIcon, BuffIcon");
        }
        Err(e) => {
            println!("❌ 核心库加载失败: {}", e);
            println!("   ⚠️ 游戏可能无法正常显示图形");
        }
    }
}
