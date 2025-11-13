// ============================================================================
// ResourceManager - 全局资源管理器
// ============================================================================
//
// 管理游戏所有资源:
// - MLibrary: 图像库
// - MapReader: 地图数据
// - SoundManager: 音频管理
// - 其他资源缓存

use std::collections::HashMap;
use std::rc::Rc;
use std::cell::RefCell;

use super::mlibrary::MLibrary;
use super::map_reader::MapReader;

/// 全局资源管理器
/// 
/// 整个游戏生命周期存在，直接放在 GameWorld 中
#[derive(Default)]
pub struct ResourceManager {
    /// 图像库缓存
    /// key: 库索引或名称
    pub libraries: HashMap<String, Rc<RefCell<MLibrary>>>,
    
    /// 地图数据缓存
    /// key: 地图文件名
    pub maps: HashMap<String, MapReader>,
    
    /// 纹理缓存（如果需要）
    /// 存储已加载的纹理以避免重复加载
    pub texture_cache: HashMap<String, macroquad::prelude::Texture2D>,
    
    // 音频缓存（预留）
    // pub sounds: HashMap<String, Sound>,
}

impl ResourceManager {
    pub fn new() -> Self {
        Self {
            libraries: HashMap::new(),
            maps: HashMap::new(),
            texture_cache: HashMap::new(),
        }
    }
    
    /// 获取或加载图像库
    pub fn get_library(&mut self, name: &str) -> Option<Rc<RefCell<MLibrary>>> {
        if let Some(lib) = self.libraries.get(name) {
            return Some(lib.clone());
        }
        
        // TODO: 实际加载逻辑
        // let lib = MLibrary::load(name)?;
        // let rc_lib = Rc::new(RefCell::new(lib));
        // self.libraries.insert(name.to_string(), rc_lib.clone());
        // Some(rc_lib)
        
        None
    }
    
    /// 获取或加载地图
    pub fn get_map(&mut self, map_name: &str) -> Option<&MapReader> {
        if self.maps.contains_key(map_name) {
            return self.maps.get(map_name);
        }
        
        // TODO: 实际加载逻辑
        // let map = MapReader::load(map_name)?;
        // self.maps.insert(map_name.to_string(), map);
        // self.maps.get(map_name)
        
        None
    }
    
    /// 获取或加载纹理
    pub fn get_texture(&self, path: &str) -> Option<macroquad::prelude::Texture2D> {
        if let Some(tex) = self.texture_cache.get(path) {
            return Some(tex.clone());
        }
        
        // TODO: 实际加载逻辑
        // let tex = load_texture(path).await.ok()?;
        // self.texture_cache.insert(path.to_string(), tex);
        // Some(tex)
        
        None
    }
    
    /// 清理未使用的资源
    pub fn cleanup_unused(&mut self) {
        // TODO: 实现引用计数检查和清理
        self.libraries.retain(|_, lib| Rc::strong_count(lib) > 1);
    }
    
    /// 预加载常用资源
    pub fn preload_common_resources(&mut self) {
        // TODO: 预加载 UI、常用角色资源等
    }
}
