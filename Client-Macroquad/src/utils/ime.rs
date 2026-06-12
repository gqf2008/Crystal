//! IME 输入法支持
//!
//! 唯一 IME 入口 — 所有聊天/输入框都通过此模块调用 `miniquad::window::*`。
//!
//! ## 背景
//! 历史上曾存在 3 套实现：
//! - `utils/ime.rs` (本文件): thin wrapper
//! - `platform/ime.rs`: Windows API (imm32.dll) — 已被移除
//! - `miniquad::window::set_ime_*`: 第三方库直接调用
//!
//! 当前统一收敛到本文件,任何新增的 IME 需求都必须:
//! 1. 优先调用 `set_ime_enabled` / `set_ime_position` 包装函数
//! 2. 严禁重新引入 `platform/ime.rs` 或直接调用 `miniquad::window`
//!
//! 依赖 miniquad PR #591 (已合并), 通过 Cargo.toml patch 引入。

/// 启用/禁用 IME 输入法
pub fn set_ime_enabled(enabled: bool) {
    macroquad::miniquad::window::set_ime_enabled(enabled);
}

/// 设置 IME 候选窗口位置(屏幕坐标, i32)
pub fn set_ime_position(x: i32, y: i32) {
    macroquad::miniquad::window::set_ime_position(x, y);
}
