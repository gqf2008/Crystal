pub mod message_box;
pub mod new_account_dialog;
pub mod change_password_dialog;
pub mod login_dialog;
pub mod new_character_dialog;

pub use message_box::{MessageBox, MessageBoxButtons, MessageBoxResult};
pub use new_account_dialog::NewAccountDialog;
pub use change_password_dialog::ChangePasswordDialog;
pub use login_dialog::{LoginDialog, LoginDialogEvent};
pub use new_character_dialog::{NewCharacterDialog, NewCharacterEvent};
use egui_macroquad::egui;

/// 对话框组件通用接口
pub trait Dialog {
    fn show(&mut self, ctx: &egui::Context, open: &mut bool);
}