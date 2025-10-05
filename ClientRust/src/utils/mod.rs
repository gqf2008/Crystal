// Utils module - Utility functions and helpers
// Corresponds to: Client/Utils/

pub mod browser_helper;
pub mod file_helper;

// Re-export commonly used functions
pub use browser_helper::{open_default_browser, open_chrome_browser};
pub use file_helper::{FileInformation, Download};
