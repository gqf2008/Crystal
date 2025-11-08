// ============================================================================
// Macroquad 字体管理系统
// ============================================================================
//
// 职责：
// - TTF 字体加载和缓存
// - 支持多种字体大小
// - 中文渲染支持
// - 文本测量和对齐
//
// ============================================================================

use macroquad::prelude::*;
use std::collections::HashMap;
use std::path::Path;

/// 字体大小类型
pub type FontSize = u16;

/// 文本对齐方式
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TextAlign {
    Left,
    Center,
    Right,
}

/// 字体数据
#[derive(Clone)]
pub struct FontData {
    /// macroquad 字体
    pub font: Font,

    /// 字体名称
    pub name: String,
}

/// 字体管理器
///
/// 管理字体的加载、缓存和渲染
pub struct FontManager {
    /// 字体缓存: 字体名 -> FontData
    fonts: HashMap<String, FontData>,

    /// 默认字体名称
    default_font: Option<String>,
}

impl FontManager {
    /// 创建新的字体管理器
    pub fn new() -> Self {
        Self {
            fonts: HashMap::new(),
            default_font: None,
        }
    }

    /// 加载 TTF 字体
    ///
    /// # 参数
    /// - `name`: 字体名称（用于后续引用）
    /// - `path`: 字体文件路径
    ///
    /// # 返回
    /// - `Ok(())`: 成功
    /// - `Err(String)`: 失败原因
    pub async fn load_font<P: AsRef<Path>>(&mut self, name: &str, path: P) -> Result<(), String> {
        let path_str = path.as_ref().to_string_lossy().to_string();

        match load_ttf_font(&path_str).await {
            Ok(font) => {
                println!("✅ 加载字体: {} ({})", name, path_str);

                let font_data = FontData {
                    font,
                    name: name.to_string(),
                };

                self.fonts.insert(name.to_string(), font_data);

                // 如果是第一个字体，设为默认
                if self.default_font.is_none() {
                    self.default_font = Some(name.to_string());
                }

                Ok(())
            }
            Err(e) => Err(format!("加载字体 {} 失败: {:?}", name, e)),
        }
    }

    /// 设置默认字体
    pub fn set_default_font(&mut self, name: &str) -> bool {
        if self.fonts.contains_key(name) {
            self.default_font = Some(name.to_string());
            true
        } else {
            false
        }
    }

    /// 获取字体
    pub fn get_font(&self, name: &str) -> Option<&Font> {
        self.fonts.get(name).map(|data| &data.font)
    }

    /// 获取默认字体
    pub fn get_default_font(&self) -> Option<&Font> {
        self.default_font
            .as_ref()
            .and_then(|name| self.get_font(name))
    }

    /// 绘制文本（使用指定字体）
    ///
    /// # 参数
    /// - `text`: 文本内容
    /// - `x`: X 坐标
    /// - `y`: Y 坐标（基线位置）
    /// - `font_size`: 字体大小
    /// - `color`: 文本颜色
    /// - `font_name`: 字体名称（None 使用默认字体）
    pub fn draw_text(
        &self,
        text: &str,
        x: f32,
        y: f32,
        font_size: FontSize,
        color: macroquad::color::Color,
        font_name: Option<&str>,
    ) {
        let font = if let Some(name) = font_name {
            self.get_font(name)
        } else {
            self.get_default_font()
        };

        if let Some(font) = font {
            draw_text_ex(
                text,
                x,
                y,
                TextParams {
                    font: Some(font),
                    font_size,
                    color,
                    ..Default::default()
                },
            );
        } else {
            // 使用 macroquad 默认字体
            draw_text(text, x, y, font_size as f32, color);
        }
    }

    /// 绘制文本（使用默认字体）
    pub fn draw_text_default(
        &self,
        text: &str,
        x: f32,
        y: f32,
        font_size: FontSize,
        color: macroquad::color::Color,
    ) {
        self.draw_text(text, x, y, font_size, color, None);
    }

    /// 绘制居中文本
    pub fn draw_text_centered(
        &self,
        text: &str,
        x: f32,
        y: f32,
        font_size: FontSize,
        color: macroquad::color::Color,
        font_name: Option<&str>,
    ) {
        let dims = self.measure_text(text, font_size, font_name);
        let centered_x = x - dims.width / 2.0;
        self.draw_text(text, centered_x, y, font_size, color, font_name);
    }

    /// 绘制右对齐文本
    pub fn draw_text_right_aligned(
        &self,
        text: &str,
        x: f32,
        y: f32,
        font_size: FontSize,
        color: macroquad::color::Color,
        font_name: Option<&str>,
    ) {
        let dims = self.measure_text(text, font_size, font_name);
        let aligned_x = x - dims.width;
        self.draw_text(text, aligned_x, y, font_size, color, font_name);
    }

    /// 绘制带对齐的文本
    pub fn draw_text_aligned(
        &self,
        text: &str,
        x: f32,
        y: f32,
        font_size: FontSize,
        color: macroquad::color::Color,
        align: TextAlign,
        font_name: Option<&str>,
    ) {
        match align {
            TextAlign::Left => self.draw_text(text, x, y, font_size, color, font_name),
            TextAlign::Center => self.draw_text_centered(text, x, y, font_size, color, font_name),
            TextAlign::Right => {
                self.draw_text_right_aligned(text, x, y, font_size, color, font_name)
            }
        }
    }

    /// 测量文本尺寸
    ///
    /// # 参数
    /// - `text`: 文本内容
    /// - `font_size`: 字体大小
    /// - `font_name`: 字体名称（None 使用默认字体）
    ///
    /// # 返回
    /// 文本尺寸（宽度和高度）
    pub fn measure_text(
        &self,
        text: &str,
        font_size: FontSize,
        font_name: Option<&str>,
    ) -> TextDimensions {
        let font = if let Some(name) = font_name {
            self.get_font(name)
        } else {
            self.get_default_font()
        };

        measure_text(text, font, font_size, 1.0)
    }

    /// 测量文本宽度
    pub fn measure_text_width(
        &self,
        text: &str,
        font_size: FontSize,
        font_name: Option<&str>,
    ) -> f32 {
        self.measure_text(text, font_size, font_name).width
    }

    /// 测量文本高度
    pub fn measure_text_height(
        &self,
        text: &str,
        font_size: FontSize,
        font_name: Option<&str>,
    ) -> f32 {
        self.measure_text(text, font_size, font_name).height
    }

    /// 绘制多行文本
    ///
    /// # 参数
    /// - `text`: 文本内容
    /// - `x`: X 坐标
    /// - `y`: Y 坐标（顶部）
    /// - `font_size`: 字体大小
    /// - `line_height`: 行高（像素）
    /// - `color`: 文本颜色
    /// - `font_name`: 字体名称
    pub fn draw_text_multiline(
        &self,
        text: &str,
        x: f32,
        y: f32,
        font_size: FontSize,
        line_height: f32,
        color: macroquad::color::Color,
        font_name: Option<&str>,
    ) {
        let mut current_y = y;

        for line in text.lines() {
            self.draw_text(line, x, current_y, font_size, color, font_name);
            current_y += line_height;
        }
    }

    /// 绘制带阴影的文本
    pub fn draw_text_with_shadow(
        &self,
        text: &str,
        x: f32,
        y: f32,
        font_size: FontSize,
        color: macroquad::color::Color,
        shadow_offset: f32,
        shadow_color: macroquad::color::Color,
        font_name: Option<&str>,
    ) {
        // 绘制阴影
        self.draw_text(
            text,
            x + shadow_offset,
            y + shadow_offset,
            font_size,
            shadow_color,
            font_name,
        );

        // 绘制文本
        self.draw_text(text, x, y, font_size, color, font_name);
    }

    /// 绘制带边框的文本
    pub fn draw_text_with_outline(
        &self,
        text: &str,
        x: f32,
        y: f32,
        font_size: FontSize,
        color: macroquad::color::Color,
        outline_color: macroquad::color::Color,
        font_name: Option<&str>,
    ) {
        // 绘制边框（8个方向）
        for dx in [-1.0, 0.0, 1.0] {
            for dy in [-1.0, 0.0, 1.0] {
                if dx == 0.0 && dy == 0.0 {
                    continue;
                }
                self.draw_text(text, x + dx, y + dy, font_size, outline_color, font_name);
            }
        }

        // 绘制主文本
        self.draw_text(text, x, y, font_size, color, font_name);
    }

    /// 清除所有字体
    pub fn clear(&mut self) {
        self.fonts.clear();
        self.default_font = None;
    }

    /// 获取已加载字体数量
    pub fn font_count(&self) -> usize {
        self.fonts.len()
    }

    /// 检查字体是否已加载
    pub fn has_font(&self, name: &str) -> bool {
        self.fonts.contains_key(name)
    }
}

impl Default for FontManager {
    fn default() -> Self {
        Self::new()
    }
}

/// 文本渲染参数构建器
pub struct TextBuilder<'a> {
    font_manager: &'a FontManager,
    text: String,
    x: f32,
    y: f32,
    font_size: FontSize,
    color: macroquad::color::Color,
    font_name: Option<String>,
    align: TextAlign,
    shadow: Option<(f32, macroquad::color::Color)>,
    outline: Option<macroquad::color::Color>,
}

impl<'a> TextBuilder<'a> {
    pub fn new(font_manager: &'a FontManager, text: &str) -> Self {
        Self {
            font_manager,
            text: text.to_string(),
            x: 0.0,
            y: 0.0,
            font_size: 20,
            color: WHITE,
            font_name: None,
            align: TextAlign::Left,
            shadow: None,
            outline: None,
        }
    }

    pub fn position(mut self, x: f32, y: f32) -> Self {
        self.x = x;
        self.y = y;
        self
    }

    pub fn font_size(mut self, size: FontSize) -> Self {
        self.font_size = size;
        self
    }

    pub fn color(mut self, color: macroquad::color::Color) -> Self {
        self.color = color;
        self
    }

    pub fn font(mut self, name: &str) -> Self {
        self.font_name = Some(name.to_string());
        self
    }

    pub fn align(mut self, align: TextAlign) -> Self {
        self.align = align;
        self
    }

    pub fn shadow(mut self, offset: f32, color: macroquad::color::Color) -> Self {
        self.shadow = Some((offset, color));
        self
    }

    pub fn outline(mut self, color: macroquad::color::Color) -> Self {
        self.outline = Some(color);
        self
    }

    /// 绘制文本
    pub fn draw(self) {
        let font_name = self.font_name.as_deref();

        if let Some((offset, shadow_color)) = self.shadow {
            self.font_manager.draw_text_with_shadow(
                &self.text,
                self.x,
                self.y,
                self.font_size,
                self.color,
                offset,
                shadow_color,
                font_name,
            );
        } else if let Some(outline_color) = self.outline {
            self.font_manager.draw_text_with_outline(
                &self.text,
                self.x,
                self.y,
                self.font_size,
                self.color,
                outline_color,
                font_name,
            );
        } else {
            self.font_manager.draw_text_aligned(
                &self.text,
                self.x,
                self.y,
                self.font_size,
                self.color,
                self.align,
                font_name,
            );
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_font_manager_creation() {
        let manager = FontManager::new();
        assert_eq!(manager.font_count(), 0);
        assert!(!manager.has_font("test"));
    }

    #[test]
    fn test_text_align() {
        assert_eq!(TextAlign::Left, TextAlign::Left);
        assert_ne!(TextAlign::Left, TextAlign::Center);
    }
}
