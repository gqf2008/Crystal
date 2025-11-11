// Macroquad graphics backend
// 图形库管理（macroquad 版本）

pub mod libraries;
pub mod mlibrary;

// 导出主要类型
pub use libraries::{Libraries, LibraryArray, LibraryName, LIBRARIES, get_map_library};
pub use mlibrary::MLibrary;

// 向后兼容：旧代码可能使用 LibraryManager
pub type LibraryManager = Libraries;