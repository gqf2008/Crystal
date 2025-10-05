// Forms module - Main application windows
// Corresponds to: Client/Forms/

pub mod launcher;      // AMain.cs - Patcher/Launcher window
pub mod main_window;   // CMain.cs - Game main window
pub mod config;        // Config.cs - Configuration dialog

// Re-export main types
pub use launcher::LauncherWindow;
pub use main_window::MainWindow;
pub use config::ConfigWindow;
