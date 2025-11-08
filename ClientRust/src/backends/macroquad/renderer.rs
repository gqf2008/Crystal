// ============================================================================
// Macroquad 渲染后端实现
// ============================================================================
//
// 基于 macroquad 的跨平台渲染器
// 支持 Web (WASM)、移动端 (iOS/Android)、桌面 (Windows/macOS/Linux)
//
// ============================================================================

use crate::backends::{
    Color, DrawParams, Rect, RenderError, Renderer, TextAlignment, TextParams, TextureId,
    TextureManager, Vec2,
};
use macroquad::prelude::*;
use std::collections::HashMap;

/// Macroquad 渲染器
pub struct MacroquadRenderer {
    /// 纹理缓存 (TextureId -> Texture2D)
    textures: HashMap<TextureId, Texture2D>,

    /// 下一个纹理 ID
    next_texture_id: u64,

    /// 字体缓存
    fonts: HashMap<String, Font>,
}

impl MacroquadRenderer {
    /// 创建新的渲染器
    pub fn new() -> Self {
        Self {
            textures: HashMap::new(),
            next_texture_id: 1,
            fonts: HashMap::new(),
        }
    }

    /// 加载字体
    pub async fn load_font(&mut self, name: &str, path: &str) -> Result<(), RenderError> {
        let font_data = load_file(path)
            .await
            .map_err(|e| RenderError::FontLoadFailed(format!("{}: {}", path, e)))?;

        let font = load_ttf_font_from_bytes(&font_data)
            .map_err(|e| RenderError::FontLoadFailed(format!("解析失败: {}", e)))?;

        self.fonts.insert(name.to_string(), font);
        Ok(())
    }
}

impl Renderer for MacroquadRenderer {
    fn clear(&mut self, color: Color) {
        let mq_color = to_macroquad_color(color);
        clear_background(mq_color);
    }

    fn draw_texture(&mut self, texture_id: TextureId, params: DrawParams) {
        let texture = match self.textures.get(&texture_id) {
            Some(tex) => tex,
            None => {
                tracing::warn!("⚠️ 纹理 ID {:?} 不存在", texture_id);
                return;
            }
        };

        let draw_params = DrawTextureParams {
            dest_size: Some(macroquad::math::vec2(
                texture.width() * params.scale.x,
                texture.height() * params.scale.y,
            )),
            source: params.src_rect.map(|r| to_macroquad_rect(r)),
            rotation: params.rotation,
            flip_x: params.flip_x,
            flip_y: params.flip_y,
            pivot: None,
        };

        draw_texture_ex(
            texture,
            params.position.x,
            params.position.y,
            WHITE,
            DrawTextureParams {
                dest_size: Some(macroquad::math::vec2(params.scale.x, params.scale.y)),
                rotation: params.rotation,
                ..Default::default()
            },
        );
    }

    fn draw_rect(&mut self, rect: Rect, color: Color) {
        draw_rectangle(rect.x, rect.y, rect.w, rect.h, to_macroquad_color(color));
    }

    fn draw_line(&mut self, start: Vec2, end: Vec2, thickness: f32, color: Color) {
        draw_line(
            start.x,
            start.y,
            end.x,
            end.y,
            thickness,
            to_macroquad_color(color),
        );
    }

    fn draw_text(&mut self, text: &str, pos: Vec2, params: TextParams) {
        let font = params
            .font_name
            .as_ref()
            .and_then(|name| self.fonts.get(name));

        let mq_text_params = macroquad::text::TextParams {
            font: font,                         // 源码显示是 Option<&Font>，不需要 .copied()
            font_size: params.font_size as u16, // 确认是 u16
            color: to_macroquad_color(params.color),
            ..Default::default()
        };

        // 简化对齐处理 (macroquad 不直接支持，需要手动计算)
        let final_x = match params.alignment {
            TextAlignment::Left => pos.x,
            TextAlignment::Center => {
                let dimensions = measure_text(text, font, params.font_size as u16, 1.0);
                pos.x - dimensions.width / 2.0
            }
            TextAlignment::Right => {
                let dimensions = measure_text(text, font, params.font_size as u16, 1.0);
                pos.x - dimensions.width
            }
        };

        draw_text_ex(text, final_x, pos.y, mq_text_params);
    }
    fn present(&mut self) -> Result<(), RenderError> {
        // macroquad 的渲染是自动的，不需要显式调用
        Ok(())
    }

    fn screen_size(&self) -> (f32, f32) {
        (screen_width(), screen_height())
    }
}

impl TextureManager for MacroquadRenderer {
    fn create_texture_from_rgba(
        &mut self,
        width: u16,
        height: u16,
        data: &[u8],
    ) -> Result<TextureId, RenderError> {
        if data.len() != (width as usize * height as usize * 4) {
            return Err(RenderError::TextureLoadFailed(format!(
                "数据大小不匹配: 期望 {} 字节，实际 {} 字节",
                width as usize * height as usize * 4,
                data.len()
            )));
        }

        let texture = Texture2D::from_rgba8(width, height, data);
        texture.set_filter(FilterMode::Nearest); // 像素艺术风格

        let id = TextureId::new(self.next_texture_id);
        self.next_texture_id += 1;

        self.textures.insert(id, texture);
        Ok(id)
    }

    fn delete_texture(&mut self, id: TextureId) {
        // macroquad 的 Texture2D 会自动清理，不需要显式删除
        self.textures.remove(&id);
    }

    fn texture_size(&self, id: TextureId) -> Option<(u16, u16)> {
        self.textures
            .get(&id)
            .map(|tex| (tex.width() as u16, tex.height() as u16))
    }
}

// ============================================================================
// 辅助函数：类型转换
// ============================================================================

fn to_macroquad_color(color: Color) -> macroquad::color::Color {
    macroquad::color::Color::new(color.r, color.g, color.b, color.a)
}

fn to_macroquad_rect(rect: Rect) -> macroquad::math::Rect {
    macroquad::math::Rect::new(rect.x, rect.y, rect.w, rect.h)
}
