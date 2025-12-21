// UI Module - 基于原版Crystal客户端的UI系统 (纯 Native 版本)

pub mod components;
pub mod dialogs;
pub mod additive;
pub mod text_renderer;
pub mod ui_state;
pub mod widgets;

// 导出 text_renderer
pub use text_renderer::*;