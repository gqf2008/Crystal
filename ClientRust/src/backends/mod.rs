// ============================================================================
// 渲染后端抽象层 - 支持多种渲染引擎
// ============================================================================
//
// 设计目标：
// 1. 提供统一的 Renderer trait，抽象不同渲染引擎的差异
// 2. 保持 ECS 系统不变，只需替换渲染后端
// 3. 支持 ggez 和 macroquad 两种后端
//
// 架构分层：
// - types.rs: 通用数据类型（Vec2, Color, DrawParams 等）
// - mod.rs: Renderer 和 TextureManager traits
// - ggez/: ggez 后端实现
// - macroquad/: macroquad 后端实现
//
// ============================================================================

pub mod types;

#[cfg(feature = "backend-ggez")]
pub mod ggez;

#[cfg(feature = "backend-macroquad")]
pub mod macroquad;

pub use types::*;

// 重新导出后端实现
#[cfg(feature = "backend-macroquad")]
pub use self::macroquad::MacroquadRenderer;

/// 通用渲染器 trait
///
/// 所有渲染后端都需要实现这个 trait
pub trait Renderer {
    /// 清空屏幕
    fn clear(&mut self, color: Color);

    /// 绘制纹理/精灵
    fn draw_texture(&mut self, texture_id: TextureId, params: DrawParams);

    /// 绘制矩形
    fn draw_rect(&mut self, rect: Rect, color: Color);

    /// 绘制线条
    fn draw_line(&mut self, start: Vec2, end: Vec2, thickness: f32, color: Color);

    /// 绘制文本
    fn draw_text(&mut self, text: &str, pos: Vec2, params: TextParams);

    /// 呈现到屏幕
    fn present(&mut self) -> Result<(), RenderError>;

    /// 获取屏幕尺寸
    fn screen_size(&self) -> (f32, f32);
}

/// 纹理管理器 trait
///
/// 用于加载和管理纹理资源
pub trait TextureManager {
    /// 从 RGBA 数据创建纹理
    fn create_texture_from_rgba(
        &mut self,
        width: u16,
        height: u16,
        data: &[u8],
    ) -> Result<TextureId, RenderError>;

    /// 删除纹理
    fn delete_texture(&mut self, id: TextureId);

    /// 获取纹理尺寸
    fn texture_size(&self, id: TextureId) -> Option<(u16, u16)>;
}

/// 渲染错误类型
#[derive(Debug, thiserror::Error)]
pub enum RenderError {
    #[error("纹理加载失败: {0}")]
    TextureLoadFailed(String),

    #[error("渲染失败: {0}")]
    RenderFailed(String),

    #[error("字体加载失败: {0}")]
    FontLoadFailed(String),

    #[error("无效的纹理 ID: {0:?}")]
    InvalidTextureId(TextureId),
}
