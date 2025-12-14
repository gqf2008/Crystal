// 统一的文本渲染模块 - 支持中文字体

use macroquad::prelude::*;
use once_cell::sync::OnceCell;

/// 全局中文字体实例
static CHINESE_FONT: OnceCell<Option<Font>> = OnceCell::new();

/// 初始化中文字体
/// 
/// 应该在程序启动时调用一次
pub async fn init_chinese_font() {
    let font = if let Ok(font) = load_ttf_font("assets/fonts/AlibabaPuHuiTi-3-55-Regular.ttf").await {
        Some(font)
    } else if let Ok(font) = load_ttf_font("assets/fonts/Chinese.ttc").await {
        Some(font)
    } else if let Ok(font) = load_ttf_font("C:\\Windows\\Fonts\\msyh.ttc").await {
        Some(font)
    } else {
        None
    };
    
    if font.is_some() {
        println!("✅ 文本渲染器：已加载中文字体");
    } else {
        println!("⚠️  文本渲染器：无法加载中文字体，将使用默认字体");
    }
    
    CHINESE_FONT.set(font).ok();
}

/// 获取中文字体引用
pub fn get_chinese_font() -> Option<&'static Font> {
    CHINESE_FONT.get().and_then(|f| f.as_ref())
}

/// 绘制文本（自动使用中文字体）
/// 
/// # 参数
/// - `text`: 要绘制的文本
/// - `x`, `y`: 文本位置
/// - `font_size`: 字体大小
/// - `color`: 文本颜色
pub fn draw_text_cn(text: &str, x: f32, y: f32, font_size: f32, color: Color) {
    let params = TextParams {
        font: get_chinese_font(),
        font_size: font_size as u16,
        color,
        ..Default::default()
    };
    
    draw_text_ex(text, x, y, params);
}

/// 绘制居中文本（自动使用中文字体）
/// 
/// # 参数
/// - `text`: 要绘制的文本
/// - `center_x`, `y`: 文本中心X坐标和Y坐标
/// - `font_size`: 字体大小
/// - `color`: 文本颜色
pub fn draw_text_centered(text: &str, center_x: f32, y: f32, font_size: f32, color: Color) {
    let text_size = measure_text_cn(text, font_size);
    let x = center_x - text_size.width / 2.0;
    draw_text_cn(text, x, y, font_size, color);
}

/// 绘制右对齐文本（自动使用中文字体）
/// 
/// # 参数
/// - `text`: 要绘制的文本
/// - `right_x`, `y`: 文本右侧X坐标和Y坐标
/// - `font_size`: 字体大小
/// - `color`: 文本颜色
pub fn draw_text_right_aligned(text: &str, right_x: f32, y: f32, font_size: f32, color: Color) {
    let text_size = measure_text_cn(text, font_size);
    let x = right_x - text_size.width;
    draw_text_cn(text, x, y, font_size, color);
}

/// 测量文本尺寸（使用中文字体）
/// 
/// # 参数
/// - `text`: 要测量的文本
/// - `font_size`: 字体大小
/// 
/// # 返回
/// TextDimensions 包含宽度、高度和偏移信息
pub fn measure_text_cn(text: &str, font_size: f32) -> TextDimensions {
    measure_text(text, get_chinese_font(), font_size as u16, 1.0)
}

/// 绘制文本（带阴影效果）
/// 
/// # 参数
/// - `text`: 要绘制的文本
/// - `x`, `y`: 文本位置
/// - `font_size`: 字体大小
/// - `color`: 文本颜色
/// - `shadow_color`: 阴影颜色
/// - `shadow_offset`: 阴影偏移量
pub fn draw_text_with_shadow(
    text: &str, 
    x: f32, 
    y: f32, 
    font_size: f32, 
    color: Color,
    shadow_color: Color,
    shadow_offset: f32,
) {
    // 先绘制阴影
    draw_text_cn(text, x + shadow_offset, y + shadow_offset, font_size, shadow_color);
    // 再绘制文本
    draw_text_cn(text, x, y, font_size, color);
}

/// 绘制描边文本
/// 
/// # 参数
/// - `text`: 要绘制的文本
/// - `x`, `y`: 文本位置
/// - `font_size`: 字体大小
/// - `color`: 文本颜色
/// - `outline_color`: 描边颜色
pub fn draw_text_with_outline(
    text: &str,
    x: f32,
    y: f32,
    font_size: f32,
    color: Color,
    outline_color: Color,
) {
    // 绘制描边（8个方向）
    let offset = 1.0;
    for dx in [-offset, 0.0, offset] {
        for dy in [-offset, 0.0, offset] {
            if dx != 0.0 || dy != 0.0 {
                draw_text_cn(text, x + dx, y + dy, font_size, outline_color);
            }
        }
    }
    // 绘制文本
    draw_text_cn(text, x, y, font_size, color);
}

#[cfg(test)]
mod tests {
    #[test]
    fn test_measure_text() {
        // 注意：这个测试需要在运行时环境中才能正确执行
        // 在单元测试中可能无法加载字体
    }
}
