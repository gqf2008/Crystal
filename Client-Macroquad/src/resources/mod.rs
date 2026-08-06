pub mod libraries;
pub mod map_reader;
pub mod mlibrary;
pub mod resource_manager;

pub use libraries::*;
pub use map_reader::MapReader;
pub use mlibrary::MLibrary;

// 新的高性能资源管理器
pub use resource_manager::{
    cache_stats,
    clear_cache,
    get_from_array,
    get_library,
    get_map_library,
    get_map_size, // 地图纹理函数（仅 Macroquad 渲染）
    get_map_texture,
    get_size,
    get_texture,
    init_array,
    load_to_array,
    preload_libraries,
    set_cache_size,
    set_data_path,
    CacheStats,
    LruCache,
    ResourceManager,
};
