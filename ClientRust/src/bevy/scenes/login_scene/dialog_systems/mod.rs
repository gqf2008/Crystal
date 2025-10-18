// Dialog Systems Module - 对话框系统模块
// 包含新建账号对话框和修改密码对话框

pub mod new_account_dialog;
pub mod change_password_dialog;

// Re-export public functions
pub use new_account_dialog::spawn_new_account_dialog;
pub use change_password_dialog::spawn_change_password_dialog;
