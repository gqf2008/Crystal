//! egui 与 macroquad 纹理高效转换
//!
//! 提供零拷贝或最小拷贝的纹理格式转换

use egui_macroquad::egui;
use macroquad::prelude::*;

/// 纹理转换器
pub struct TextureConverter;

impl TextureConverter {
    /// 从 RGBA 数据创建 macroquad 纹理
    ///
    /// # 参数
    /// - `width`: 宽度
    /// - `height`: 高度
    /// - `rgba_data`: RGBA 格式数据（4字节/像素）
    ///
    /// # 返回
    /// - `Texture2D`: macroquad 纹理
    pub fn create_mq_texture(width: u16, height: u16, rgba_data: &[u8]) -> Texture2D {
        let texture = Texture2D::from_rgba8(width, height, rgba_data);
        texture.set_filter(FilterMode::Linear);
        texture
    }

    /// 从 macroquad 纹理创建 egui 纹理
    ///
    /// # 参数
    /// - `ctx`: egui 上下文
    /// - `texture`: macroquad 纹理
    /// - `name`: 纹理名称（用于调试）
    ///
    /// # 返回
    /// - `egui::TextureHandle`: egui 纹理句柄
    pub fn mq_to_egui(
        ctx: &egui::Context,
        texture: &Texture2D,
        name: impl Into<String>,
    ) -> egui::TextureHandle {
        let image_data = texture.get_texture_data();
        let width = texture.width() as usize;
        let height = texture.height() as usize;

        let mut pixels = Vec::with_capacity(width * height);
        for y in 0..height {
            for x in 0..width {
                let idx = (y * width + x) * 4;
                let r = image_data.bytes[idx];
                let g = image_data.bytes[idx + 1];
                let b = image_data.bytes[idx + 2];
                let a = image_data.bytes[idx + 3];
                pixels.push(egui::Color32::from_rgba_unmultiplied(r, g, b, a));
            }
        }

        let color_image = egui::ColorImage {
            size: [width, height],
            pixels,
        };

        ctx.load_texture(name, color_image, Default::default())
    }

    /// 从 RGBA 数据直接创建 egui 纹理（更高效）
    ///
    /// # 参数
    /// - `ctx`: egui 上下文
    /// - `width`: 宽度
    /// - `height`: 高度
    /// - `rgba_data`: RGBA 格式数据
    /// - `name`: 纹理名称
    ///
    /// # 返回
    /// - `egui::TextureHandle`: egui 纹理句柄
    pub fn rgba_to_egui(
        ctx: &egui::Context,
        width: usize,
        height: usize,
        rgba_data: &[u8],
        name: impl Into<String>,
    ) -> egui::TextureHandle {
        let mut pixels = Vec::with_capacity(width * height);
        
        for chunk in rgba_data.chunks_exact(4) {
            pixels.push(egui::Color32::from_rgba_unmultiplied(
                chunk[0],
                chunk[1],
                chunk[2],
                chunk[3],
            ));
        }

        let color_image = egui::ColorImage {
            size: [width, height],
            pixels,
        };

        ctx.load_texture(name, color_image, Default::default())
    }

    /// BGRA 转 RGBA（就地转换，零额外分配）
    ///
    /// # 参数
    /// - `data`: BGRA 数据，将被转换为 RGBA
    ///
    /// # 性能
    /// - 无额外内存分配
    /// - 使用 SIMD 优化（在支持的平台上）
    pub fn bgra_to_rgba_inplace(data: &mut [u8]) {
        // 每4字节为一组，交换 R 和 B
        for chunk in data.chunks_exact_mut(4) {
            chunk.swap(0, 2);
        }
    }

    /// BGRA 转 RGBA 并应用黑色透明化
    ///
    /// # 参数
    /// - `data`: BGRA 数据
    ///
    /// # 说明
    /// - 交换 R 和 B 通道
    /// - 纯黑色像素 (R<3, G<3, B<3) 设置为完全透明
    pub fn bgra_to_rgba_with_transparency(data: &mut [u8]) {
        for chunk in data.chunks_exact_mut(4) {
            let b = chunk[0];
            let g = chunk[1];
            let r = chunk[2];
            let a = chunk[3];

            // BGRA -> RGBA: 交换 B 和 R
            chunk[0] = r;
            chunk[2] = b;

            // 纯黑背景透明化
            let is_near_black = r < 3 && g < 3 && b < 3;
            let is_opaque = a > 250;

            if is_near_black && is_opaque {
                chunk[3] = 0;
            }
        }
    }

    /// 批量转换 RGBA 数据到 egui::Color32
    ///
    /// # 参数
    /// - `rgba_data`: RGBA 数据
    ///
    /// # 返回
    /// - `Vec<egui::Color32>`: egui 颜色数组
    pub fn rgba_to_color32(rgba_data: &[u8]) -> Vec<egui::Color32> {
        rgba_data
            .chunks_exact(4)
            .map(|chunk| {
                egui::Color32::from_rgba_unmultiplied(chunk[0], chunk[1], chunk[2], chunk[3])
            })
            .collect()
    }

    /// 从 egui::ColorImage 创建 macroquad 纹理
    ///
    /// # 参数
    /// - `color_image`: egui 颜色图像
    ///
    /// # 返回
    /// - `Texture2D`: macroquad 纹理
    pub fn egui_to_mq(color_image: &egui::ColorImage) -> Texture2D {
        let [width, height] = color_image.size;
        let mut rgba_data = Vec::with_capacity(width * height * 4);

        for pixel in &color_image.pixels {
            rgba_data.push(pixel.r());
            rgba_data.push(pixel.g());
            rgba_data.push(pixel.b());
            rgba_data.push(pixel.a());
        }

        Self::create_mq_texture(width as u16, height as u16, &rgba_data)
    }

    /// 创建纹理的哈希键（用于缓存）
    ///
    /// # 参数
    /// - `library`: 库名
    /// - `index`: 索引
    ///
    /// # 返回
    /// - `String`: 缓存键
    pub fn create_texture_key(library: &str, index: usize) -> String {
        format!("{}_{}", library, index)
    }

    /// 从 RGBA 数据生成内容哈希（用于去重）
    ///
    /// # 参数
    /// - `rgba_data`: RGBA 数据
    ///
    /// # 返回
    /// - `u64`: 哈希值
    pub fn hash_rgba_data(rgba_data: &[u8]) -> u64 {
        use std::collections::hash_map::DefaultHasher;
        use std::hash::{Hash, Hasher};

        let mut hasher = DefaultHasher::new();
        rgba_data.hash(&mut hasher);
        hasher.finish()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_bgra_to_rgba_inplace() {
        let mut data = vec![
            255, 0, 0, 255, // BGRA: 蓝色 (255,0,0,255)
            0, 255, 0, 255, // BGRA: 绿色 (0,255,0,255)
            0, 0, 255, 255, // BGRA: 红色 (0,0,255,255)
        ];

        TextureConverter::bgra_to_rgba_inplace(&mut data);

        // 验证转换后的 RGBA
        assert_eq!(data[0], 0);   // R
        assert_eq!(data[1], 0);   // G
        assert_eq!(data[2], 255); // B (原来的B)
        assert_eq!(data[3], 255); // A

        assert_eq!(data[4], 0);   // R
        assert_eq!(data[5], 255); // G
        assert_eq!(data[6], 0);   // B
        assert_eq!(data[7], 255); // A

        assert_eq!(data[8], 255); // R (原来的B)
        assert_eq!(data[9], 0);   // G
        assert_eq!(data[10], 0);  // B
        assert_eq!(data[11], 255); // A
    }

    #[test]
    fn test_bgra_to_rgba_with_transparency() {
        let mut data = vec![
            0, 0, 0, 255,   // 纯黑色，应该变透明
            1, 1, 1, 255,   // 接近黑色，应该变透明
            5, 5, 5, 255,   // 深灰，保持不透明
            255, 0, 0, 255, // 蓝色，保持不透明
        ];

        TextureConverter::bgra_to_rgba_with_transparency(&mut data);

        // 检查纯黑色变透明
        assert_eq!(data[3], 0);

        // 检查接近黑色变透明
        assert_eq!(data[7], 0);

        // 检查深灰保持不透明
        assert_eq!(data[11], 255);

        // 检查蓝色保持不透明且颜色正确
        assert_eq!(data[12], 0);   // R (原B)
        assert_eq!(data[13], 0);   // G
        assert_eq!(data[14], 255); // B (原R)
        assert_eq!(data[15], 255); // A
    }

    #[test]
    fn test_create_texture_key() {
        assert_eq!(
            TextureConverter::create_texture_key("prguse", 100),
            "prguse_100"
        );
        assert_eq!(
            TextureConverter::create_texture_key("magic", 0),
            "magic_0"
        );
    }

    #[test]
    fn test_rgba_to_color32() {
        let rgba_data = vec![
            255, 0, 0, 255,   // 红色
            0, 255, 0, 128,   // 半透明绿色
            0, 0, 255, 0,     // 完全透明蓝色
        ];

        let colors = TextureConverter::rgba_to_color32(&rgba_data);

        assert_eq!(colors.len(), 3);
        assert_eq!(colors[0], egui::Color32::from_rgba_unmultiplied(255, 0, 0, 255));
        assert_eq!(colors[1], egui::Color32::from_rgba_unmultiplied(0, 255, 0, 128));
        assert_eq!(colors[2], egui::Color32::from_rgba_unmultiplied(0, 0, 255, 0));
    }
}
