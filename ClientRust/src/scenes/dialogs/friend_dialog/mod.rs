// Friend Dialog Module - 好友对话框模块
// 对应C#的FriendDialog.cs文件，包含好友相关的对话框

pub mod friend_dialog;
pub mod friend_row;
pub mod memo_dialog;

// Re-exports
pub use friend_dialog::FriendDialog;
pub use friend_row::FriendRow;
pub use memo_dialog::MemoDialog;