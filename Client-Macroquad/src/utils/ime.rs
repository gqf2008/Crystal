//! IME 输入方法桩函数
//!
//! 原版 miniquad 的本地 patch 被移除后，`miniquad::window::set_ime_enabled`
//! 和 `set_ime_position` 不再可用。此处提供空实现以保证编译通过，
//! 待上游提供替代方案后可替换为真实实现。

/// 启用/禁用 IME 输入法
#[allow(dead_code)]
pub fn set_ime_enabled(_enabled: bool) {
    // TODO: 上游 miniquad 恢复 IME 支持后实现
}

/// 设置 IME 候选窗口位置（屏幕坐标，已考虑 DPI 缩放）
#[allow(dead_code)]
pub fn set_ime_position(_x: i32, _y: i32) {
    // TODO: 上游 miniquad 恢复 IME 支持后实现
}
