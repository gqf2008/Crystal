pub mod libraries;
pub mod mlibrary;
pub mod map_reader;
pub mod resource_manager;

pub use libraries::init_map_libraries;
pub use mlibrary::MLibrary;
pub use map_reader::MapReader;
pub use resource_manager::ResourceManager;

use std::rc::Rc;
use std::cell::RefCell;

/// 获取地图库 (兼容函数)
pub fn get_map_library(_index: usize) -> Option<Rc<RefCell<MLibrary>>> {
    // TODO: 实现地图库获取逻辑
    // 暂时返回 None,等待实际实现
    None
}
