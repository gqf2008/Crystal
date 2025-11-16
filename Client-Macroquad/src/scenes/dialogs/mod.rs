pub mod message_box;

pub use message_box::{MessageBox, MessageBoxButtons, MessageBoxResult};
use egui_macroquad::egui;

/// 对话框组件通用接口
pub trait Dialog {
    fn show(&mut self, ctx: &egui::Context, open: &mut bool);
}