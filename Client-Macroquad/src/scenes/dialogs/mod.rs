pub mod message_box;

pub use message_box::{MessageBox, MessageBoxButtons, MessageBoxResult};

use egui_macroquad::egui;

/// 对话框 trait，所有对话框均实现此 trait
pub trait Dialog {
    /// Is the demo enabled for this integration?
    fn is_enabled(&self, _ctx: &egui::Context) -> bool {
        true
    }

    /// `&'static` so we can also use it as a key to store open/close state.
    fn name(&self) -> &'static str{
        std::any::type_name::<Self>()
    }

    /// Show windows, etc
    fn show(&mut self, ctx: &egui::Context, open: &mut bool);
}