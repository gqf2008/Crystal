// Trade Dialog Module - 交易对话框模块
// 对应C#的TradeDialogs.cs文件，包含交易相关的对话框

pub mod trade_dialog;
pub mod guest_trade_dialog;

// Re-exports
pub use trade_dialog::TradeDialog;
pub use guest_trade_dialog::GuestTradeDialog;