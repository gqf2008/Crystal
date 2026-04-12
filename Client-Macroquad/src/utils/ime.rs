//! IME 输入法支持
//!
//! 依赖 miniquad PR #591（已合并），通过 Cargo.toml patch 引入。

/// 启用/禁用 IME 输入法
pub fn set_ime_enabled(enabled: bool) {
    macroquad::miniquad::window::set_ime_enabled(enabled);
}

/// 设置 IME 候选窗口位置（屏幕坐标）
pub fn set_ime_position(x: i32, y: i32) {
    macroquad::miniquad::window::set_ime_position(x, y);
}
