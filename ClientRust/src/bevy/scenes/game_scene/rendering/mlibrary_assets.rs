// MLibrary Assets - Bevy 资源系统集成
// 
// 功能说明:
// 将 MLibrary 纹理系统集成到 Bevy 的资源管理系统
// 负责从 .lib 文件加载纹理并转换为 Bevy Image 资源
//
// 复用策略:
// - 完全复用 graphics::libraries 的加载逻辑
// - 完全复用 graphics::mlibrary::MLibrary
// - 只是适配到 Bevy 的 Assets<Image> 系统
//
// 参考:
// - ClientRust/src/graphics/mlibrary.rs - MLibrary 核心实现
// - ClientRust/src/graphics/libraries.rs - 库管理 (已经处理好所有加载逻辑)

use bevy::prelude::*;
use bevy::render::render_resource::{Extent3d, TextureDimension, TextureFormat};
use std::collections::HashMap;
use std::path::PathBuf;

// ==================== 完全复用现有的库管理系统 ====================
use crate::graphics::libraries::{
    self, 
    LibraryArray, 
    get_library_from_array,
    get_map_library,
    initialize_all_libraries,
};
use crate::graphics::mlibrary::MLibrary;

/// Bevy 资源: MLibrary 纹理资产管理器
/// 
/// 职责:
/// 1. 调用 graphics::libraries 加载 .lib 文件 (复用)
/// 2. 从 MLibrary 提取图像数据并转换为 Bevy Image (适配)
/// 3. 缓存 Bevy 纹理 Handle (优化)
/// 4. 提供简洁的查询接口
#[derive(Resource)]
pub struct MLibraryAssets {
    /// 数据路径 (Data 文件夹)
    data_path: PathBuf,
    
    /// 已创建的 Bevy 纹理 Handle 缓存
    /// Key: "MapLibs_0_100" (LibraryArray_索引_图像索引)
    /// Value: Bevy Image Handle
    texture_cache: HashMap<String, Handle<Image>>,
    
    /// 统计信息
    cache_hits: usize,
    cache_misses: usize,
}

impl MLibraryAssets {
    /// 创建新的 MLibraryAssets
    /// 
    /// # 参数
    /// - `data_path`: Data 文件夹路径
    pub fn new(data_path: PathBuf) -> Self {
        Self {
            data_path,
            texture_cache: HashMap::new(),
            cache_hits: 0,
            cache_misses: 0,
        }
    }
    
    /// 预加载所有必要的库
    /// 
    /// 调用 graphics::libraries 的 initialize_all_libraries
    /// 这会加载 MapLibs[0-399] 和所有游戏内容库
    pub fn preload_all_libraries(&mut self) -> Result<(), String> {
        info!("🎮 开始预加载所有 MLibrary 库...");
        
        // 复用 libraries.rs 的初始化逻辑
        initialize_all_libraries(self.data_path.to_str().unwrap())
            .map_err(|e| format!("库初始化失败: {:?}", e))?;
        
        info!("✅ 所有 MLibrary 库预加载完成");
        Ok(())
    }
    
    /// 获取或加载纹理 (从 MapLibs)
    /// 
    /// 这是最常用的方法,用于地图渲染
    /// 
    /// # 参数
    /// - `file_index`: MapLibs 数组索引 (0-399)
    /// - `image_index`: 图像索引 (在 .lib 文件中的索引)
    /// - `images`: Bevy Assets<Image> 引用
    /// 
    /// # 返回
    /// - Some(Handle<Image>): 纹理句柄
    /// - None: 加载失败
    pub fn get_map_texture(
        &mut self,
        file_index: i16,
        image_index: usize,
        images: &mut Assets<Image>,
    ) -> Option<Handle<Image>> {
        self.get_texture_from_array(
            LibraryArray::MapLibs,
            file_index as usize,
            image_index,
            images,
        )
    }
    
    /// 获取或加载纹理 (从任意数组库)
    /// 
    /// 通用方法,支持所有 LibraryArray 类型
    /// 
    /// # 参数
    /// - `array_type`: 数组库类型 (MapLibs, Monsters, NPCs 等)
    /// - `lib_index`: 数组索引
    /// - `image_index`: 图像索引
    /// - `images`: Bevy Assets<Image> 引用
    /// 
    /// # 返回
    /// - Some(Handle<Image>): 纹理句柄
    /// - None: 加载失败
    pub fn get_texture_from_array(
        &mut self,
        array_type: LibraryArray,
        lib_index: usize,
        image_index: usize,
        images: &mut Assets<Image>,
    ) -> Option<Handle<Image>> {
        // 1. 生成缓存 key
        let cache_key = format!("{:?}_{}_{}", array_type, lib_index, image_index);
        
        // 2. 检查缓存
        if let Some(handle) = self.texture_cache.get(&cache_key) {
            self.cache_hits += 1;
            return Some(handle.clone());
        }
        
        self.cache_misses += 1;
        
        // 3. 从 libraries.rs 获取 MLibrary (已经加载好了)
        let mlibrary = get_library_from_array(array_type, lib_index)?;
        let mut lib = mlibrary.lock().unwrap();
        
        // 4. 从 MLibrary 获取图像数据
        let (image_info, image_data) = lib.get_image_with_data(image_index).ok()?;
        
        // 5. 转换为 Bevy Image
        let bevy_image = self.convert_to_bevy_image(&image_info, &image_data);
        
        // 6. 添加到 Bevy Assets 并缓存 Handle
        let handle = images.add(bevy_image);
        self.texture_cache.insert(cache_key, handle.clone());
        
        Some(handle)
    }
    
    /// 将 MLibrary 图像数据转换为 Bevy Image
    /// 
    /// MLibrary 使用 BGRA8 格式,Bevy 使用 Rgba8UnormSrgb
    /// 需要交换 R 和 B 通道
    /// 
    /// # 参数
    /// - `image_info`: MLibrary 图像信息
    /// - `image_data`: BGRA8 原始数据
    /// 
    /// # 返回
    /// - Image: Bevy 图像
    fn convert_to_bevy_image(
        &self,
        image_info: &crate::graphics::mlibrary::ImageInfo,
        image_data: &[u8],
    ) -> Image {
        let width = image_info.width as u32;
        let height = image_info.height as u32;
        
        // MLibrary 使用 BGRA8,需要转换为 RGBA8
        let mut rgba_data = Vec::with_capacity(image_data.len());
        for chunk in image_data.chunks_exact(4) {
            rgba_data.push(chunk[2]); // R (from B)
            rgba_data.push(chunk[1]); // G
            rgba_data.push(chunk[0]); // B (from R)
            rgba_data.push(chunk[3]); // A
        }
        
        Image::new(
            Extent3d {
                width,
                height,
                depth_or_array_layers: 1,
            },
            TextureDimension::D2,
            rgba_data,
            TextureFormat::Rgba8UnormSrgb,
            Default::default(),
        )
    }
    
    /// 获取地图库信息
    /// 
    /// 用于调试和统计
    pub fn get_map_library_info(&self, file_index: i16) -> Option<usize> {
        let mlibrary = get_map_library(file_index)?;
        let lib = mlibrary.lock().unwrap();
        Some(lib.count())
    }
    
    /// 清理未使用的纹理缓存
    /// 
    /// 定期调用来释放内存
    /// 
    /// # 参数
    /// - `images`: Bevy Assets<Image> 引用
    /// 
    /// # 返回
    /// - 清理的纹理数量
    pub fn cleanup_unused_textures(&mut self, images: &Assets<Image>) -> usize {
        let before_count = self.texture_cache.len();
        
        // 移除 Assets 中不存在的 Handle
        self.texture_cache.retain(|_key, handle| {
            images.get(handle).is_some()
        });
        
        let removed = before_count - self.texture_cache.len();
        
        if removed > 0 {
            debug!("🧹 清理了 {} 个未使用的纹理缓存", removed);
        }
        
        removed
    }
    
    /// 获取缓存统计信息
    pub fn get_cache_stats(&self) -> CacheStats {
        CacheStats {
            cache_size: self.texture_cache.len(),
            cache_hits: self.cache_hits,
            cache_misses: self.cache_misses,
            hit_rate: if self.cache_hits + self.cache_misses > 0 {
                self.cache_hits as f32 / (self.cache_hits + self.cache_misses) as f32
            } else {
                0.0
            },
        }
    }
    
    /// 清空所有缓存
    pub fn clear_cache(&mut self) {
        self.texture_cache.clear();
        self.cache_hits = 0;
        self.cache_misses = 0;
        info!("🗑️ 已清空所有纹理缓存");
    }
}

/// 缓存统计信息
#[derive(Debug, Clone)]
pub struct CacheStats {
    pub cache_size: usize,
    pub cache_hits: usize,
    pub cache_misses: usize,
    pub hit_rate: f32,
}

// ==================== Bevy 系统 ====================

/// 系统: 初始化 MLibraryAssets 资源
/// 
/// 在 GameScene 启动时调用
/// 这会预加载所有必要的库
pub fn setup_mlibrary_assets(mut commands: Commands) {
    info!("🎨 初始化 MLibraryAssets...");
    
    // TODO: 从配置读取 data_path
    let data_path = PathBuf::from("Data");
    
    let mut assets = MLibraryAssets::new(data_path);
    
    // 预加载所有库 (复用 libraries.rs 的逻辑)
    if let Err(e) = assets.preload_all_libraries() {
        error!("❌ 预加载纹理库失败: {}", e);
        // 即使失败也创建资源,允许后续懒加载
    }
    
    commands.insert_resource(assets);
    info!("✅ MLibraryAssets 资源已初始化");
}

/// 系统: 定期清理未使用的纹理
/// 
/// 建议每 30 秒运行一次
pub fn cleanup_mlibrary_textures_system(
    mut mlibrary_assets: ResMut<MLibraryAssets>,
    images: Res<Assets<Image>>,
) {
    let removed = mlibrary_assets.cleanup_unused_textures(&images);
    
    if removed > 0 {
        let stats = mlibrary_assets.get_cache_stats();
        info!("🧹 纹理清理完成: 移除 {} 个, 剩余 {} 个", removed, stats.cache_size);
    }
}

/// 系统: 调试输出 MLibraryAssets 统计信息
/// 
/// 用于性能分析和调试
pub fn debug_mlibrary_stats_system(
    mlibrary_assets: Res<MLibraryAssets>,
) {
    let stats = mlibrary_assets.get_cache_stats();
    
    info!(
        "📊 MLibrary 统计: {} 个纹理缓存 | 命中率 {:.1}% ({}/{})",
        stats.cache_size,
        stats.hit_rate * 100.0,
        stats.cache_hits,
        stats.cache_hits + stats.cache_misses
    );
}

// ==================== 使用示例 ====================

/// 示例: 如何在渲染系统中使用 MLibraryAssets
/// 
/// ```rust
/// fn render_map_tile_system(
///     mut mlibrary_assets: ResMut<MLibraryAssets>,
///     mut images: ResMut<Assets<Image>>,
///     mut commands: Commands,
/// ) {
///     // 获取地图纹理 (MapLibs[0], 图像索引 100)
///     if let Some(texture_handle) = mlibrary_assets.get_map_texture(0, 100, &mut images) {
///         // 创建 Sprite
///         commands.spawn(SpriteBundle {
///             texture: texture_handle,
///             ..default()
///         });
///     }
/// }
/// ```
#[allow(dead_code)]
fn example_usage() {
    // 这只是文档示例,不会被编译
}
