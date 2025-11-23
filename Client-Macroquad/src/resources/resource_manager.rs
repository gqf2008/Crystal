//! 高性能资源管理器
//!
//! 特性:
//! 1. 简洁高效的API设计
//! 2. LRU纹理缓存支持
//! 3. 无锁单线程设计(RefCell)
//! 4. 懒加载和智能缓存
//! 5. 全局单例便捷访问

use std::cell::RefCell;
use std::collections::HashMap;
use std::path::Path;
use std::rc::Rc;

use egui_macroquad::egui;

use super::mlibrary::{ImageInfo, MLibrary};
use crate::resources::libraries::{LibraryArray, LibraryName};

// ==================== LRU 缓存实现 ====================

/// LRU 缓存节点
struct LruNode<K, V> {
    key: K,
    value: V,
    prev: Option<usize>,
    next: Option<usize>,
}

/// 无锁 LRU 缓存
///
/// 使用双向链表实现，O(1) 访问和淘汰
pub struct LruCache<K, V>
where
    K: std::hash::Hash + Eq + Clone,
{
    capacity: usize,
    nodes: Vec<LruNode<K, V>>,
    map: HashMap<K, usize>,
    head: Option<usize>,
    tail: Option<usize>,
    free_list: Vec<usize>,
}

impl<K, V> LruCache<K, V>
where
    K: std::hash::Hash + Eq + Clone,
{
    /// 创建新的 LRU 缓存
    pub fn new(capacity: usize) -> Self {
        Self {
            capacity,
            nodes: Vec::with_capacity(capacity),
            map: HashMap::with_capacity(capacity),
            head: None,
            tail: None,
            free_list: Vec::new(),
        }
    }

    /// 获取缓存项
    pub fn get(&mut self, key: &K) -> Option<&V> {
        if let Some(&idx) = self.map.get(key) {
            self.move_to_front(idx);
            Some(&self.nodes[idx].value)
        } else {
            None
        }
    }

    /// 插入或更新缓存项
    pub fn put(&mut self, key: K, value: V) {
        // 如果已存在，更新值并移到前面
        if let Some(&idx) = self.map.get(&key) {
            self.nodes[idx].value = value;
            self.move_to_front(idx);
            return;
        }

        // 如果缓存已满，淘汰最久未使用的项
        if self.map.len() >= self.capacity && self.capacity > 0 {
            self.evict_lru();
        }

        // 添加新节点
        let idx = if let Some(free_idx) = self.free_list.pop() {
            self.nodes[free_idx] = LruNode {
                key: key.clone(),
                value,
                prev: None,
                next: self.head,
            };
            free_idx
        } else {
            let idx = self.nodes.len();
            self.nodes.push(LruNode {
                key: key.clone(),
                value,
                prev: None,
                next: self.head,
            });
            idx
        };

        self.map.insert(key, idx);

        if let Some(head_idx) = self.head {
            self.nodes[head_idx].prev = Some(idx);
        }
        self.head = Some(idx);

        if self.tail.is_none() {
            self.tail = Some(idx);
        }
    }

    /// 移动节点到链表头部
    fn move_to_front(&mut self, idx: usize) {
        if self.head == Some(idx) {
            return;
        }

        // 从当前位置移除
        let prev = self.nodes[idx].prev;
        let next = self.nodes[idx].next;

        if let Some(p) = prev {
            self.nodes[p].next = next;
        }
        if let Some(n) = next {
            self.nodes[n].prev = prev;
        }

        if self.tail == Some(idx) {
            self.tail = prev;
        }

        // 插入到头部
        self.nodes[idx].prev = None;
        self.nodes[idx].next = self.head;

        if let Some(head_idx) = self.head {
            self.nodes[head_idx].prev = Some(idx);
        }

        self.head = Some(idx);
    }

    /// 淘汰最久未使用的项
    fn evict_lru(&mut self) {
        if let Some(tail_idx) = self.tail {
            let key = self.nodes[tail_idx].key.clone();
            self.map.remove(&key);

            if let Some(prev_idx) = self.nodes[tail_idx].prev {
                self.nodes[prev_idx].next = None;
                self.tail = Some(prev_idx);
            } else {
                self.head = None;
                self.tail = None;
            }

            self.free_list.push(tail_idx);
        }
    }

    /// 清空缓存
    pub fn clear(&mut self) {
        self.nodes.clear();
        self.map.clear();
        self.head = None;
        self.tail = None;
        self.free_list.clear();
    }

    /// 获取缓存大小
    pub fn len(&self) -> usize {
        self.map.len()
    }

    /// 检查缓存是否为空
    pub fn is_empty(&self) -> bool {
        self.map.is_empty()
    }
}

// ==================== 资源管理器 ====================

/// 纹理缓存键
#[derive(Hash, Eq, PartialEq, Clone)]
struct TextureKey {
    library: String,
    index: usize,
}

/// 高性能资源管理器
pub struct ResourceManager {
    /// 数据根目录
    data_path: String,

    /// 单体库缓存
    libraries: HashMap<LibraryName, Rc<RefCell<MLibrary>>>,

    /// 数组库缓存
    array_libraries: HashMap<LibraryArray, Vec<Option<Rc<RefCell<MLibrary>>>>>,

    /// Macroquad 纹理 LRU 缓存
    texture_cache: LruCache<TextureKey, ImageInfo>,

    /// egui 纹理 LRU 缓存 (存储包含egui纹理句柄的ImageInfo)
    egui_texture_cache: LruCache<TextureKey, ImageInfo>,

    /// 缓存容量配置
    texture_cache_size: usize,
    egui_cache_size: usize,
}

impl ResourceManager {
    /// 创建新的资源管理器
    pub fn new() -> Self {
        Self {
            data_path: "Data".to_string(),
            libraries: HashMap::new(),
            array_libraries: HashMap::new(),
            texture_cache: LruCache::new(1000), // 默认缓存1000个纹理
            egui_texture_cache: LruCache::new(500), // 默认缓存500个egui纹理
            texture_cache_size: 1000,
            egui_cache_size: 500,
        }
    }

    /// 设置数据路径
    #[inline]
    pub fn set_data_path(&mut self, path: impl Into<String>) {
        self.data_path = path.into();
    }

    /// 设置纹理缓存大小
    pub fn set_cache_size(&mut self, texture_size: usize, egui_size: usize) {
        self.texture_cache_size = texture_size;
        self.egui_cache_size = egui_size;
        self.texture_cache = LruCache::new(texture_size);
        self.egui_texture_cache = LruCache::new(egui_size);
    }

    // ==================== 库管理 ====================

    /// 获取或加载库
    #[inline]
    pub fn get_library(&mut self, name: LibraryName) -> Option<Rc<RefCell<MLibrary>>> {
        if let Some(lib) = self.libraries.get(&name) {
            println!("✅ 库已缓存: {:?}", name);
            return Some(lib.clone());
        }

        // 懒加载
        let path = format!("{}/{}", self.data_path, name.default_path());
        println!("🔍 尝试加载库: {:?} 从路径: {}", name, path);
        
        match MLibrary::open(&path) {
            Ok(lib) => {
                println!("✅ 成功加载库: {:?}, 图像数: {}", name, lib.count());
                let rc = Rc::new(RefCell::new(lib));
                self.libraries.insert(name, rc.clone());
                Some(rc)
            }
            Err(e) => {
                println!("❌ 加载库失败: {:?}, 路径: {}, 错误: {:?}", name, path, e);
                None
            }
        }
    }

    /// 初始化数组库
    pub fn init_array(&mut self, array_type: LibraryArray, size: usize) {
        self.array_libraries.insert(array_type, vec![None; size]);
    }

    /// 加载到数组库
    pub fn load_to_array(
        &mut self,
        array_type: LibraryArray,
        index: usize,
        path: impl AsRef<Path>,
    ) -> std::io::Result<()> {
        let array = self
            .array_libraries
            .get_mut(&array_type)
            .ok_or_else(|| std::io::Error::from(std::io::ErrorKind::NotFound))?;

        if index >= array.len() {
            return Err(std::io::Error::from(std::io::ErrorKind::InvalidInput));
        }

        match MLibrary::open(path) {
            Ok(lib) => {
                array[index] = Some(Rc::new(RefCell::new(lib)));
                Ok(())
            }
            Err(_) => {
                array[index] = None;
                Ok(()) // 不阻止其他库加载
            }
        }
    }

    /// 从数组库获取
    #[inline]
    pub fn get_from_array(
        &self,
        array_type: LibraryArray,
        index: usize,
    ) -> Option<Rc<RefCell<MLibrary>>> {
        self.array_libraries.get(&array_type)?.get(index)?.clone()
    }

    // ==================== 纹理缓存 ====================

    /// 获取或创建纹理（带LRU缓存）
    pub fn get_texture(&mut self, lib_name: LibraryName, index: usize) -> Option<ImageInfo> {
        let key = TextureKey {
            library: lib_name.to_string(),
            index,
        };

        // 检查缓存
        if let Some(cached) = self.texture_cache.get(&key) {
            return Some(cached.clone());
        }

        // 加载纹理
        let lib = self.get_library(lib_name)?;
        let mut lib_ref = lib.borrow_mut();
        let info = lib_ref.get_or_create_texture(index).ok()?.clone();

        // 缓存
        self.texture_cache.put(key, info.clone());

        Some(info)
    }

    /// 获取或创建 egui 纹理（带LRU缓存）
    /// 
    /// 返回包含 egui 纹理句柄的 ImageInfo
    pub fn get_egui_texture(
        &mut self,
        ctx: &egui::Context,
        lib_name: LibraryName,
        index: usize,
    ) -> Option<ImageInfo> {
        let key = TextureKey {
            library: lib_name.to_string(),
            index,
        };

        // 检查缓存
        if let Some(cached) = self.egui_texture_cache.get(&key) {
            return Some(cached.clone());
        }

        println!("🎨 创建 egui 纹理: {:?} index={}", lib_name, index);

        // 加载纹理
        let mut info = self.get_texture(lib_name, index)?;
        let texture = info.image.as_ref()?;

        // 转换为 egui 纹理
        let image_data = texture.get_texture_data();
        let width = texture.width() as usize;
        let height = texture.height() as usize;

        println!("📐 纹理尺寸: {}x{}", width, height);

        let mut pixels = Vec::with_capacity(width * height);
        for y in 0..height {
            for x in 0..width {
                let idx = (y * width + x) * 4;
                let r = image_data.bytes[idx];
                let g = image_data.bytes[idx + 1];
                let b = image_data.bytes[idx + 2];
                let a = image_data.bytes[idx + 3];
                pixels.push(egui::Color32::from_rgba_unmultiplied(r, g, b, a));
            }
        }

        let color_image = egui::ColorImage {
            size: [width, height],
            pixels,
        };

        let cache_key = format!("{}_{}", key.library, key.index);
        let handle = ctx.load_texture(&cache_key, color_image, Default::default());

        // 在 ImageInfo 中设置 egui 纹理
        info.egui_texture = Some(handle);

        // 缓存
        self.egui_texture_cache.put(key, info.clone());

        Some(info)
    }

    /// 获取图像尺寸（无需创建纹理）
    #[inline]
    pub fn get_size(&mut self, lib_name: LibraryName, index: usize) -> Option<(i16, i16)> {
        let lib = self.get_library(lib_name)?;
        let mut lib_ref = lib.borrow_mut();
        lib_ref.get_size(index).ok()
    }

    /// 清空纹理缓存
    pub fn clear_texture_cache(&mut self) {
        self.texture_cache.clear();
        self.egui_texture_cache.clear();
    }

    /// 获取缓存统计信息
    pub fn cache_stats(&self) -> CacheStats {
        CacheStats {
            texture_cache_size: self.texture_cache.len(),
            texture_cache_capacity: self.texture_cache_size,
            egui_cache_size: self.egui_texture_cache.len(),
            egui_cache_capacity: self.egui_cache_size,
            loaded_libraries: self.libraries.len(),
        }
    }
}

impl Default for ResourceManager {
    fn default() -> Self {
        Self::new()
    }
}

/// 缓存统计信息
#[derive(Debug, Clone)]
pub struct CacheStats {
    pub texture_cache_size: usize,
    pub texture_cache_capacity: usize,
    pub egui_cache_size: usize,
    pub egui_cache_capacity: usize,
    pub loaded_libraries: usize,
}

// ==================== 全局单例 ====================

thread_local! {
    static RESOURCE_MANAGER: RefCell<ResourceManager> = RefCell::new(ResourceManager::new());
}

// ==================== 便捷访问函数 ====================

/// 设置数据路径
#[inline]
pub fn set_data_path(path: impl Into<String>) {
    RESOURCE_MANAGER.with(|rm| rm.borrow_mut().set_data_path(path));
}

/// 设置缓存大小
#[inline]
pub fn set_cache_size(texture_size: usize, egui_size: usize) {
    RESOURCE_MANAGER.with(|rm| rm.borrow_mut().set_cache_size(texture_size, egui_size));
}

/// 获取纹理
#[inline]
pub fn get_texture(lib_name: LibraryName, index: usize) -> Option<ImageInfo> {
    RESOURCE_MANAGER.with(|rm| rm.borrow_mut().get_texture(lib_name, index))
}

/// 获取 egui 纹理（返回包含 egui_texture 字段的 ImageInfo）
#[inline]
pub fn get_egui_texture(
    ctx: &egui::Context,
    lib_name: LibraryName,
    index: usize,
) -> Option<ImageInfo> {
    RESOURCE_MANAGER.with(|rm| rm.borrow_mut().get_egui_texture(ctx, lib_name, index))
}

/// 获取图像尺寸
#[inline]
pub fn get_size(lib_name: LibraryName, index: usize) -> Option<(i16, i16)> {
    RESOURCE_MANAGER.with(|rm| rm.borrow_mut().get_size(lib_name, index))
}

/// 获取库
#[inline]
pub fn get_library(name: LibraryName) -> Option<Rc<RefCell<MLibrary>>> {
    RESOURCE_MANAGER.with(|rm| rm.borrow_mut().get_library(name))
}

/// 初始化数组库
#[inline]
pub fn init_array(array_type: LibraryArray, size: usize) {
    RESOURCE_MANAGER.with(|rm| rm.borrow_mut().init_array(array_type, size));
}

/// 加载到数组库
#[inline]
pub fn load_to_array(
    array_type: LibraryArray,
    index: usize,
    path: impl AsRef<Path>,
) -> std::io::Result<()> {
    RESOURCE_MANAGER.with(|rm| rm.borrow_mut().load_to_array(array_type, index, path))
}

/// 从数组库获取
#[inline]
pub fn get_from_array(
    array_type: LibraryArray,
    index: usize,
) -> Option<Rc<RefCell<MLibrary>>> {
    RESOURCE_MANAGER.with(|rm| rm.borrow().get_from_array(array_type, index))
}

/// 获取地图库（快捷方式）
#[inline]
pub fn get_map_library(index: i16) -> Option<Rc<RefCell<MLibrary>>> {
    if index < 0 || index >= 400 {
        return None;
    }
    get_from_array(LibraryArray::MapLibs, index as usize)
}

// ==================== 地图纹理便捷访问 ====================

/// 获取地图纹理（一步到位，带 LRU 缓存）
/// 
/// 这是最便捷的方式，直接从 file_index 和 image_index 获取纹理
/// 
/// # Example
/// ```rust
/// // ✅ 新方式 - 一行搞定
/// if let Some(info) = get_map_texture(file_index, image_index) {
///     if let Some(texture) = &info.image {
///         draw_texture_ex(texture, x, y, WHITE, params);
///     }
/// }
/// 
/// // ❌ 旧方式 - 需要 4 行
/// // let lib = get_map_library(file_index)?;
/// // let mut lib_guard = lib.borrow_mut();
/// // let info = lib_guard.get_or_create_texture(image_index as usize)?;
/// // let texture = info.image.as_ref()?;
/// ```
#[inline]
pub fn get_map_texture(file_index: i16, image_index: i32) -> Option<ImageInfo> {
    if file_index < 0 || file_index >= 400 {
        return None;
    }
    
    let key = TextureKey {
        library: format!("MapLib_{}", file_index),
        index: image_index as usize,
    };
    
    RESOURCE_MANAGER.with(|rm| {
        let mut rm = rm.borrow_mut();
        
        // 检查缓存
        if let Some(cached) = rm.texture_cache.get(&key) {
            return Some(cached.clone());
        }
        
        // ✅ 从 LIBRARIES 获取地图库（而不是 RESOURCE_MANAGER 的独立副本）
        let lib = crate::resources::libraries::get_from_array(
            LibraryArray::MapLibs,
            file_index as usize
        )?;
        
        let mut lib_ref = lib.borrow_mut();
        let info = lib_ref.get_or_create_texture(image_index as usize).ok()?.clone();
        
        // 缓存
        rm.texture_cache.put(key, info.clone());
        
        Some(info)
    })
}

/// 获取地图纹理尺寸（高效，无需加载纹理）
/// 
/// # Example
/// ```rust
/// if let Some((w, h)) = get_map_size(file_index, image_index) {
///     println!("地图瓦片尺寸: {}x{}", w, h);
/// }
/// ```
#[inline]
pub fn get_map_size(file_index: i16, image_index: i32) -> Option<(i16, i16)> {
    if file_index < 0 || file_index >= 400 {
        return None;
    }
    
    RESOURCE_MANAGER.with(|rm| {
        let rm = rm.borrow();
        let lib = rm.get_from_array(LibraryArray::MapLibs, file_index as usize)?;
        let mut lib_ref = lib.borrow_mut();
        lib_ref.get_size(image_index as usize).ok()
    })
}

/// 清空缓存
#[inline]
pub fn clear_cache() {
    RESOURCE_MANAGER.with(|rm| rm.borrow_mut().clear_texture_cache());
}

/// 获取缓存统计
#[inline]
pub fn cache_stats() -> CacheStats {
    RESOURCE_MANAGER.with(|rm| rm.borrow().cache_stats())
}

/// 批量预加载库（推荐在后台线程调用）
pub fn preload_libraries(libs: &[LibraryName]) {
    RESOURCE_MANAGER.with(|rm| {
        let mut rm = rm.borrow_mut();
        for &lib_name in libs {
            let _ = rm.get_library(lib_name);
        }
    });
}

// ==================== 使用示例 ====================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_lru_cache() {
        let mut cache = LruCache::new(3);

        cache.put("a", 1);
        cache.put("b", 2);
        cache.put("c", 3);

        assert_eq!(cache.get(&"a"), Some(&1));
        assert_eq!(cache.get(&"b"), Some(&2));
        assert_eq!(cache.get(&"c"), Some(&3));

        // 应该淘汰 a
        cache.put("d", 4);
        assert_eq!(cache.get(&"a"), None);
        assert_eq!(cache.get(&"d"), Some(&4));
    }

    #[test]
    fn test_resource_manager() {
        let mut rm = ResourceManager::new();
        rm.set_data_path("Data");

        // 测试缓存大小设置
        rm.set_cache_size(100, 50);
        let stats = rm.cache_stats();
        assert_eq!(stats.texture_cache_capacity, 100);
        assert_eq!(stats.egui_cache_capacity, 50);
    }
}
