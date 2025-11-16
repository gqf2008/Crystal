pub mod libraries;
pub mod mlibrary;
pub mod map_reader;
pub mod resource_manager;

pub use libraries::*;
pub use mlibrary::MLibrary;
pub use map_reader::MapReader;

// 新的高性能资源管理器
pub use resource_manager::{
    cache_stats, clear_cache, get_egui_texture, get_from_array, get_library, get_map_library,
    get_map_texture, get_map_size,  // 地图纹理函数（仅 Macroquad 渲染）
    get_size, get_texture, init_array, load_to_array, preload_libraries, set_cache_size,
    set_data_path, CacheStats, LruCache, ResourceManager,
};

