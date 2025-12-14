//! 商城对话框模块
//!
//! 拆分为以下子模块：
//! - dialog: 主结构体定义和构造函数
//! - types: 枚举和数据结构
//! - sample_items: 示例商品数据
//! - rendering: 绘制方法
//! - interaction: 交互处理方法

pub mod dialog;
mod types;
mod sample_items;
mod rendering;
mod interaction;

pub use dialog::GameShopDialogHybrid;
pub use types::*;
pub use sample_items::create_sample_items;
