// LoginScene Constants
// 所有常量定义集中在此文件,方便维护和调整

use bevy::prelude::Color;

// ============================================================================
// Animation Constants
// ============================================================================

/// 背景动画总帧数 (从 Prguse 索引 1-19)
pub const ANIMATION_FRAME_COUNT: usize = 19;

/// 每帧动画延迟时间 (秒)
pub const ANIMATION_DELAY: f32 = 0.1; // 100ms per frame

// ============================================================================
// Dialog Dimensions
// ============================================================================

/// 对话框宽度 (像素)
pub const DIALOG_WIDTH: f32 = 328.0;

/// 对话框高度 (像素)
pub const DIALOG_HEIGHT: f32 = 220.0;

// ============================================================================
// Input Validation Constants
// ============================================================================

/// 账号ID最小长度
pub const MIN_ACCOUNT_ID_LENGTH: usize = 3;

/// 账号ID最大长度
pub const MAX_ACCOUNT_ID_LENGTH: usize = 15;

/// 密码最小长度
pub const MIN_PASSWORD_LENGTH: usize = 5;

/// 密码最大长度
pub const MAX_PASSWORD_LENGTH: usize = 15;

// ============================================================================
// UI Colors
// ============================================================================

/// 按钮正常状态颜色
pub const BUTTON_NORMAL_COLOR: Color = Color::srgba(1.0, 1.0, 1.0, 1.0);

/// 按钮悬停状态颜色
pub const BUTTON_HOVER_COLOR: Color = Color::srgba(0.9, 0.9, 0.9, 1.0);

/// 按钮按下状态颜色
pub const BUTTON_PRESSED_COLOR: Color = Color::srgba(0.8, 0.8, 0.8, 1.0);

/// 输入框边框正常状态颜色
pub const INPUT_BORDER_NORMAL: Color = Color::srgba(0.5, 0.5, 0.5, 1.0);

/// 输入框边框聚焦状态颜色
pub const INPUT_BORDER_FOCUSED: Color = Color::srgba(1.0, 1.0, 0.0, 1.0);

/// 输入框边框有效状态颜色
pub const INPUT_BORDER_VALID: Color = Color::srgba(0.0, 1.0, 0.0, 1.0);

/// 输入框边框无效状态颜色
pub const INPUT_BORDER_INVALID: Color = Color::srgba(1.0, 0.0, 0.0, 1.0);

/// 文本颜色
pub const TEXT_COLOR: Color = Color::srgba(1.0, 1.0, 1.0, 1.0);
