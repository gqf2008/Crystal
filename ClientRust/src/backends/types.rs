// ============================================================================
// 渲染后端通用类型定义
// ============================================================================
//
// 这些类型在所有渲染后端中共享，确保 API 的一致性
//
// ============================================================================

use serde::{Deserialize, Serialize};

/// 2D 向量
#[derive(Debug, Clone, Copy, Default, PartialEq, Serialize, Deserialize)]
pub struct Vec2 {
    pub x: f32,
    pub y: f32,
}

impl Vec2 {
    pub fn new(x: f32, y: f32) -> Self {
        Self { x, y }
    }

    pub fn zero() -> Self {
        Self { x: 0.0, y: 0.0 }
    }
}

/// 矩形区域
#[derive(Debug, Clone, Copy, Default, PartialEq, Serialize, Deserialize)]
pub struct Rect {
    pub x: f32,
    pub y: f32,
    pub w: f32,
    pub h: f32,
}

impl Rect {
    pub fn new(x: f32, y: f32, w: f32, h: f32) -> Self {
        Self { x, y, w, h }
    }
}

/// 颜色 (RGBA)
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct Color {
    pub r: f32,
    pub g: f32,
    pub b: f32,
    pub a: f32,
}

impl Color {
    pub fn new(r: f32, g: f32, b: f32, a: f32) -> Self {
        Self { r, g, b, a }
    }

    pub fn rgb(r: f32, g: f32, b: f32) -> Self {
        Self { r, g, b, a: 1.0 }
    }

    pub fn from_rgba_u8(r: u8, g: u8, b: u8, a: u8) -> Self {
        Self {
            r: r as f32 / 255.0,
            g: g as f32 / 255.0,
            b: b as f32 / 255.0,
            a: a as f32 / 255.0,
        }
    }

    // 预定义颜色
    pub const WHITE: Color = Color {
        r: 1.0,
        g: 1.0,
        b: 1.0,
        a: 1.0,
    };
    pub const BLACK: Color = Color {
        r: 0.0,
        g: 0.0,
        b: 0.0,
        a: 1.0,
    };
    pub const RED: Color = Color {
        r: 1.0,
        g: 0.0,
        b: 0.0,
        a: 1.0,
    };
    pub const GREEN: Color = Color {
        r: 0.0,
        g: 1.0,
        b: 0.0,
        a: 1.0,
    };
    pub const BLUE: Color = Color {
        r: 0.0,
        g: 0.0,
        b: 1.0,
        a: 1.0,
    };
    pub const TRANSPARENT: Color = Color {
        r: 0.0,
        g: 0.0,
        b: 0.0,
        a: 0.0,
    };
}

/// 纹理 ID (后端内部使用)
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct TextureId(pub u64);

impl TextureId {
    pub fn new(id: u64) -> Self {
        Self(id)
    }
}

/// 绘制参数
#[derive(Debug, Clone, Copy)]
pub struct DrawParams {
    /// 位置
    pub position: Vec2,

    /// 旋转角度 (弧度)
    pub rotation: f32,

    /// 缩放
    pub scale: Vec2,

    /// 颜色调制 (用于透明度和色调)
    pub color: Color,

    /// 源矩形 (用于精灵图集)
    pub src_rect: Option<Rect>,

    /// 翻转
    pub flip_x: bool,
    pub flip_y: bool,

    /// Z 层级 (用于排序)
    pub z_order: i32,
}

impl Default for DrawParams {
    fn default() -> Self {
        Self {
            position: Vec2::zero(),
            rotation: 0.0,
            scale: Vec2::new(1.0, 1.0),
            color: Color::WHITE,
            src_rect: None,
            flip_x: false,
            flip_y: false,
            z_order: 0,
        }
    }
}

/// 文本绘制参数
#[derive(Debug, Clone)]
pub struct TextParams {
    /// 字体大小
    pub font_size: f32,

    /// 颜色
    pub color: Color,

    /// 字体名称 (可选)
    pub font_name: Option<String>,

    /// 对齐方式
    pub alignment: TextAlignment,
}

impl Default for TextParams {
    fn default() -> Self {
        Self {
            font_size: 16.0,
            color: Color::WHITE,
            font_name: None,
            alignment: TextAlignment::Left,
        }
    }
}

/// 文本对齐方式
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TextAlignment {
    Left,
    Center,
    Right,
}
