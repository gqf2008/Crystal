// LoginScene ECS 模块
// 完全使用Entity-Component-System架构

pub mod components;
pub mod ui;
pub mod systems;
pub mod dialogs;  // ✅ ECS对话框

// LoginScene主结构
mod scene;

// 导出所有ECS组件和系统
pub use components::*;
pub use ui::*;
pub use systems::*;
pub use dialogs::*;  // 导出所有对话框工厂函数和句柄

// 导出LoginScene
pub use scene::{LoginScene, BanInfo};
