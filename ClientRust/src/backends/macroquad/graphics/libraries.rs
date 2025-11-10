// Macroquad 版本的图像库管理器
// 移植自 src/graphics/libraries.rs
// 主要改动：将 ggez::graphics::Image 替换为 macroquad::texture::Texture2D

use anyhow::{Context, Result};
use macroquad::texture::{FilterMode, Texture2D};
use parking_lot::RwLock;
use std::cell::RefCell;
use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::Arc;

use crate::resources::lib_loader::{ImageData, MLibraryData};

// 线程本地缓存：最近访问的纹理（避免重复获取锁）
thread_local! {
    static TEXTURE_CACHE: RefCell<lru::LruCache<(String, usize), Texture2D>> = 
        RefCell::new(lru::LruCache::new(std::num::NonZeroUsize::new(512).unwrap()));
}

/// 图像库管理器（macroquad 版本）
///
/// 负责加载和管理所有游戏资源库
/// - 支持多种库类型：地图、角色、怪物、物品、UI等
/// - 使用 HashMap 存储已加载的纹理
/// - 线程安全（RwLock）
/// - 使用 LRU 缓存减少锁竞争
pub struct LibraryManager {
    /// 数据目录路径
    data_path: PathBuf,
    
    /// 已加载的库数据（包含图像缓存，需要可变访问）
    libraries: RwLock<HashMap<String, MLibraryData>>,
    
    /// 已创建的纹理缓存 (库名 -> 图像索引 -> Texture2D)
    textures: RwLock<HashMap<String, HashMap<usize, Texture2D>>>,
}

impl LibraryManager {
    /// 创建新的库管理器
    pub fn new<P: Into<PathBuf>>(data_path: P) -> Self {
        Self {
            data_path: data_path.into(),
            libraries: RwLock::new(HashMap::new()),
            textures: RwLock::new(HashMap::new()),
        }
    }

    /// 加载库文件
    ///
    /// # 参数
    /// - `lib_name`: 库名称（例如 "MapLib_0", "Prguse"）
    /// - `lib_path`: 库文件路径（相对于 data_path）
    pub fn load_library(&self, lib_name: &str, lib_path: &str) -> Result<()> {
        // 构建完整路径
        let full_path = self.data_path.join(lib_path);
        
        // 加载库数据
        let mut lib_data = MLibraryData::new();
        lib_data.load(&full_path)
            .with_context(|| format!("加载库失败: {}", lib_path))?;
        
        let image_count = lib_data.count();
        println!("✅ 加载库: {} ({} 张图像)", lib_name, image_count);
        
        // 存储库数据
        self.libraries.write().insert(lib_name.to_string(), lib_data);
        
        // 初始化纹理缓存
        self.textures.write().insert(lib_name.to_string(), HashMap::new());
        
        Ok(())
    }

    /// 获取或创建纹理（快速路径 - 仅适用于已缓存的纹理）
    ///
    /// 如果纹理已缓存，直接返回；否则返回 None（不加载）
    /// 
    /// 这是高性能版本，只获取一次读锁
    ///
    /// # 参数
    /// - `lib_name`: 库名称
    /// - `image_index`: 图像索引
    ///
    /// # 返回
    /// - Some(Texture2D) 如果纹理已缓存
    /// - None 如果纹理未缓存或库不存在
    #[inline]
    pub fn get_cached_texture(&self, lib_name: &str, image_index: usize) -> Option<Texture2D> {
        let textures = self.textures.read();
        textures.get(lib_name)?.get(&image_index).cloned()
    }

    /// 获取或创建纹理（完整路径 - 按需加载）
    ///
    /// 如果纹理已缓存，直接返回；否则从库中解压并创建 Texture2D
    ///
    /// 使用线程本地 LRU 缓存减少锁竞争
    ///
    /// # 参数
    /// - `lib_name`: 库名称
    /// - `image_index`: 图像索引
    ///
    /// # 返回
    /// - Some(Texture2D) 如果找到图像
    /// - None 如果库不存在或索引越界
    #[inline]
    pub fn get_or_create_texture(&self, lib_name: &str, image_index: usize) -> Option<Texture2D> {
        let cache_key = (lib_name.to_string(), image_index);
        
        // 超快速路径：检查线程本地 LRU 缓存（无锁）
        let cached = TEXTURE_CACHE.with(|cache| {
            cache.borrow_mut().get(&cache_key).cloned()
        });
        
        if let Some(texture) = cached {
            return Some(texture);
        }

        // 快速路径：检查全局纹理缓存（读锁）
        let texture = {
            let textures = self.textures.read();
            textures.get(lib_name)?.get(&image_index).cloned()
        };
        
        if let Some(texture) = texture {
            // 更新线程本地缓存
            TEXTURE_CACHE.with(|cache| {
                cache.borrow_mut().put(cache_key, texture.clone());
            });
            return Some(texture);
        }

        // 慢速路径：从库中加载图像数据
        let image_data = {
            let mut libraries = self.libraries.write();
            let lib_data = libraries.get_mut(lib_name)?;
            match lib_data.get_image(image_index) {
                Ok(Some(img)) => img.clone(),
                Ok(None) => return None,
                Err(e) => {
                    // 只在非占位符错误时才输出错误信息
                    if e.kind() != std::io::ErrorKind::InvalidData || !e.to_string().contains("占位符") {
                        eprintln!("❌ 加载图像失败: {} [{}] - {}", lib_name, image_index, e);
                    }
                    return None;
                }
            }
        };

        // 创建 Texture2D
        let texture = self.create_texture_from_image_data(&image_data);

        // 缓存纹理到全局缓存
        {
            let mut textures = self.textures.write();
            if let Some(lib_textures) = textures.get_mut(lib_name) {
                lib_textures.insert(image_index, texture.clone());
            }
        }
        
        // 缓存到线程本地 LRU
        TEXTURE_CACHE.with(|cache| {
            cache.borrow_mut().put(cache_key, texture.clone());
        });

        Some(texture)
    }

    /// 从 ImageData 创建 Texture2D
    ///
    /// 核心移植点：ggez::graphics::Image -> macroquad::texture::Texture2D
    fn create_texture_from_image_data(&self, image_data: &ImageData) -> Texture2D {
        let width = image_data.width as u16;
        let height = image_data.height as u16;
        
        // ImageData 已经是 RGBA 格式（在 lib_loader.rs 中完成了 BGRA->RGBA 转换）
        let rgba_data = &image_data.rgba_data;
        
        // 创建 Texture2D（macroquad API）
        let texture = Texture2D::from_rgba8(width, height, rgba_data);
        
        // 设置过滤模式为最近邻（像素艺术）
        texture.set_filter(FilterMode::Nearest);
        
        texture
    }

    /// 获取库的图像数量
    pub fn get_library_count(&self, lib_name: &str) -> Option<usize> {
        let libraries = self.libraries.read();
        libraries.get(lib_name).map(|lib| lib.count())
    }

    /// 检查库是否已加载
    pub fn is_library_loaded(&self, lib_name: &str) -> bool {
        self.libraries.read().contains_key(lib_name)
    }

    /// 清空纹理缓存（释放显存）
    pub fn clear_texture_cache(&self) {
        self.textures.write().clear();
    }

    /// 清空指定库的纹理缓存
    pub fn clear_library_cache(&self, lib_name: &str) {
        if let Some(lib_textures) = self.textures.write().get_mut(lib_name) {
            lib_textures.clear();
        }
    }

    /// 批量预加载纹理（高性能版本）
    ///
    /// 一次性加载多个纹理，只获取一次锁，显著提升性能
    /// 
    /// # 参数
    /// - `requests`: (库名, 图像索引) 列表
    /// 
    /// # 返回
    /// - Vec<Option<Texture2D>>: 按请求顺序返回纹理（如果不存在则为 None）
    pub fn batch_get_or_create_textures(&self, requests: &[(String, usize)]) -> Vec<Option<Texture2D>> {
        let mut results = Vec::with_capacity(requests.len());
        
        // 第一遍：快速检查已缓存的纹理（一次读锁）
        {
            let textures = self.textures.read();
            for (lib_name, image_index) in requests {
                if let Some(lib_textures) = textures.get(lib_name) {
                    if let Some(texture) = lib_textures.get(image_index) {
                        results.push(Some(texture.clone()));
                        continue;
                    }
                }
                results.push(None); // 标记为未缓存
            }
        }
        
        // 第二遍：加载未缓存的纹理
        for (i, (lib_name, image_index)) in requests.iter().enumerate() {
            if results[i].is_some() {
                continue; // 已缓存，跳过
            }
            
            // 从库中加载
            if let Some(texture) = self.get_or_create_texture(lib_name, *image_index) {
                results[i] = Some(texture);
            }
        }
        
        results
    }

    /// 获取数据路径
    pub fn data_path(&self) -> &PathBuf {
        &self.data_path
    }

    /// 加载所有地图库（MapLib_0 到 MapLib_399）
    /// 
    /// 这个方法封装了完整的库映射逻辑，参考 C# MLibrary.cs 的实现
    /// 
    /// # 库映射说明
    /// - **WeMade Mir2** (0-99): 原版传奇2地图库
    /// - **Shanda Mir2** (100-199): 盛大传奇2地图库  
    /// - **WeMade Mir3** (200-299): 原版传奇3地图库（5种地形状态）
    /// - **Shanda Mir3** (300-399): 盛大传奇3地图库（5种地形状态）
    pub fn load_map_libraries(&self) -> Result<()> {
        println!("🗺️  开始加载地图库...");
        
        // ========== WeMade Mir2 (0-99) ==========
        self.load_library("MapLib_0", "Map/WemadeMir2/Tiles.Lib")?;
        self.load_library("MapLib_1", "Map/WemadeMir2/SmTiles.Lib")?;
        
        // Objects 系列 (2-29)
        for i in 2..=29 {
            let lib_num = i;
            let obj_num = i - 2; // Objects.Lib=0, Objects1.Lib=1, ...
            let filename = if obj_num == 0 {
                "Objects.Lib".to_string()
            } else {
                format!("Objects{}.Lib", obj_num)
            };
            let path = format!("Map/WemadeMir2/{}", filename);
            
            if let Err(e) = self.load_library(&format!("MapLib_{}", lib_num), &path) {
                eprintln!("⚠️  MapLib_{} ({}) 加载失败: {}", lib_num, filename, e);
            }
        }
        
        // Objects_32bit (90)
        if let Err(e) = self.load_library("MapLib_90", "Map/WemadeMir2/Objects_32bit.Lib") {
            eprintln!("⚠️  MapLib_90 (Objects_32bit.Lib) 加载失败: {}", e);
        }
        
        // ========== Shanda Mir2 (100-199) ==========
        // Tiles 系列 (100-109)
        for i in 0..10 {
            let lib_num = 100 + i;
            let filename = if i == 0 {
                "Tiles.Lib".to_string()
            } else {
                format!("Tiles{}.Lib", i + 1)
            };
            let path = format!("Map/ShandaMir2/{}", filename);
            
            if let Err(e) = self.load_library(&format!("MapLib_{}", lib_num), &path) {
                eprintln!("⚠️  MapLib_{} ({}) 加载失败: {}", lib_num, filename, e);
            }
        }
        
        // SmTiles 系列 (110-119)
        for i in 0..10 {
            let lib_num = 110 + i;
            let filename = if i == 0 {
                "smTiles.Lib".to_string()
            } else {
                format!("smTiles{}.Lib", i + 1)
            };
            let path = format!("Map/ShandaMir2/{}", filename);
            
            if let Err(e) = self.load_library(&format!("MapLib_{}", lib_num), &path) {
                eprintln!("⚠️  MapLib_{} ({}) 加载失败: {}", lib_num, filename, e);
            }
        }
        
        // Objects 系列 (120-150)
        for i in 0..31 {
            let lib_num = 120 + i;
            let filename = if i == 0 {
                "Objects.Lib".to_string()
            } else {
                format!("Objects{}.Lib", i + 1)
            };
            let path = format!("Map/ShandaMir2/{}", filename);
            
            if let Err(e) = self.load_library(&format!("MapLib_{}", lib_num), &path) {
                eprintln!("⚠️  MapLib_{} ({}) 加载失败: {}", lib_num, filename, e);
            }
        }
        
        // AniTiles1 (190)
        if let Err(e) = self.load_library("MapLib_190", "Map/ShandaMir2/AniTiles1.Lib") {
            eprintln!("⚠️  MapLib_190 (AniTiles1.Lib) 加载失败: {}", e);
        }
        
        // ========== WeMade Mir3 (200-299) ==========
        let mapstates = ["", "wood/", "sand/", "snow/", "forest/"];
        for (state_idx, state) in mapstates.iter().enumerate() {
            let base = 200 + (state_idx * 15);
            
            let lib_files = [
                (0, "Tilesc"),
                (1, "Tiles30c"),
                (2, "Tiles5c"),
                (3, "Smtilesc"),
                (4, "Housesc"),
                (5, "Cliffsc"),
                (6, "Dungeonsc"),
                (7, "Innersc"),
                (8, "Furnituresc"),
                (9, "Wallsc"),
                (10, "smObjectsc"),
                (11, "Animationsc"),
                (12, "Object1c"),
                (13, "Object2c"),
            ];
            
            for (offset, filename) in lib_files.iter() {
                let lib_num = base + offset;
                let path = format!("Map/WemadeMir3/{}{}.Lib", state, filename);
                
                if let Err(e) = self.load_library(&format!("MapLib_{}", lib_num), &path) {
                    eprintln!("⚠️  MapLib_{} ({}) 加载失败: {}", lib_num, path, e);
                }
            }
        }
        
        // ========== Shanda Mir3 (300-399) ==========
        let mapstates_shanda = ["", "wood", "sand", "snow", "forest"];
        for (state_idx, state) in mapstates_shanda.iter().enumerate() {
            let base = 300 + (state_idx * 15);
            let state_suffix = if state.is_empty() { 
                String::new() 
            } else { 
                format!("-{}", state) 
            };
            
            let lib_files = [
                (0, format!("Tilesc{}", state_suffix)),
                (1, format!("Tiles30c{}", state_suffix)),
                (2, format!("Tiles5c{}", state_suffix)),
                (3, format!("Smtilesc{}", state_suffix)),
                (4, format!("Housesc{}", state_suffix)),
                (5, format!("Cliffsc{}", state_suffix)),
                (6, format!("Dungeonsc{}", state_suffix)),
                (7, format!("Innersc{}", state_suffix)),
                (8, format!("Furnituresc{}", state_suffix)),
                (9, format!("Wallsc{}", state_suffix)),
                (10, format!("smObjectsc{}", state_suffix)),
                (11, format!("Animationsc{}", state_suffix)),
                (12, format!("Object1c{}", state_suffix)),
                (13, format!("Object2c{}", state_suffix)),
            ];
            
            for (offset, filename) in lib_files.iter() {
                let lib_num = base + offset;
                let path = format!("Map/ShandaMir3/{}.Lib", filename);
                
                if let Err(e) = self.load_library(&format!("MapLib_{}", lib_num), &path) {
                    eprintln!("⚠️  MapLib_{} ({}) 加载失败: {}", lib_num, path, e);
                }
            }
        }
        
        println!("✅ 地图库加载完成");
        Ok(())
    }
}

impl Default for LibraryManager {
    fn default() -> Self {
        Self::new("Data")
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_library_manager_creation() {
        let manager = LibraryManager::new("Data");
        assert_eq!(manager.data_path(), &PathBuf::from("Data"));
    }

    #[test]
    fn test_library_loading() {
        let manager = LibraryManager::new("Data");
        
        // 测试加载库（如果文件存在）
        if let Ok(_) = manager.load_library("TestLib", "Map/WemadeMir2/Tiles.Lib") {
            assert!(manager.is_library_loaded("TestLib"));
        }
    }
}
