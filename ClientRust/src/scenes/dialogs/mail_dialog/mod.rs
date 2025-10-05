// Mail Dialog Module - 邮件对话框模块
// 对应C#的MailDialogs.cs文件，包含多个邮件相关对话框

pub mod mail_list_dialog;
pub mod mail_item_row;
pub mod mail_compose_letter_dialog;
pub mod mail_compose_parcel_dialog;
pub mod mail_read_letter_dialog;
pub mod mail_read_parcel_dialog;

// Re-exports
pub use mail_list_dialog::{MailListDialog, ClientMail, MailType, MailStatus};
pub use mail_item_row::MailItemRow;
pub use mail_compose_letter_dialog::MailComposeLetterDialog;
pub use mail_compose_parcel_dialog::MailComposeParcelDialog;
pub use mail_read_letter_dialog::MailReadLetterDialog;
pub use mail_read_parcel_dialog::MailReadParcelDialog;