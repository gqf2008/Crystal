// ============================================================================
// 平台特定功能模块
// ============================================================================

#[cfg(target_os = "windows")]
pub mod ime;

#[cfg(target_os = "windows")]
pub use ime::set_ime_position;

// 非 Windows 平台的空实现
#[cfg(not(target_os = "windows"))]
pub fn set_ime_position(_x: i32, _y: i32) {
    // 在非 Windows 平台上不做任何事情
}
