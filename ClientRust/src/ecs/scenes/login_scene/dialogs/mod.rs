//! 对话框实体工厂模块
// 所有对话框都使用ECS架构

pub mod new_account_entity;
pub mod login_dialog_entity;
pub mod connecting_box_entity;
pub mod message_box_entity;
pub mod change_password_entity;

// 导出所有对话框
pub use new_account_entity::*;
pub use login_dialog_entity::*;
pub use connecting_box_entity::*;
pub use message_box_entity::*;
pub use change_password_entity::*;
