pub mod message_box;
pub mod new_account_dialog;
pub mod change_password_dialog;

pub use message_box::{MessageBox, MessageBoxButtons, MessageBoxResult};
pub use new_account_dialog::NewAccountDialog;
pub use change_password_dialog::ChangePasswordDialog;
use egui_macroquad::egui;

/// 对话框组件通用接口
pub trait Dialog {
    fn show(&mut self, ctx: &egui::Context, open: &mut bool);
}