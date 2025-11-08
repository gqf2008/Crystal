// ============================================================================
// Macroquad + MLibrary 演示
// ============================================================================
//
// 展示使用 MLibraryData 加载图像并在 macroquad 中渲染
//
// 用法：
//   cargo run --bin demo_mlibrary_macroquad --no-default-features --features backend-macroquad
//
// ============================================================================

use macroquad::prelude::*;
use std::collections::HashMap;

// 内联 MLibraryData 实现（简化版）
mod mlibrary_data {
    use byteorder::{LittleEndian, ReadBytesExt};
    use flate2::read::ZlibDecoder;
    use std::collections::HashMap;
    use std::fs::File;
    use std::io::{self, Read, Seek, SeekFrom};
    use std::path::Path;

    #[derive(Debug, Clone)]
    pub struct LibraryHeader {
        pub count: i32,
        pub version: i32,
    }

    #[derive(Debug, Clone, Copy)]
    pub struct ImageIndex {
        pub offset: u32,
    }

    #[derive(Debug, Clone)]
    pub struct MaskData {
        pub width: i16,
        pub height: i16,
        pub data: Vec<u8>,
    }

    #[derive(Debug, Clone)]
    pub struct ImageData {
        pub width: i16,
        pub height: i16,
        pub offset_x: i16,
        pub offset_y: i16,
        pub rgba_data: Vec<u8>,
        pub mask: Option<MaskData>,
    }

    pub struct MLibraryData {
        file_path: String,
        header: LibraryHeader,
        indices: Vec<ImageIndex>,
        cache: HashMap<usize, ImageData>,
        max_cache_size: usize,
        lru_order: Vec<usize>,
    }

    impl MLibraryData {
        pub fn load<P: AsRef<Path>>(path: P) -> io::Result<Self> {
            let path_str = path.as_ref().to_string_lossy().to_string();
            let mut file = File::open(&path)?;

            // 读取头部 (12 字节)
            let count = file.read_i32::<LittleEndian>()?;
            file.read_i32::<LittleEndian>()?; // 跳过 4 字节
            let version = file.read_i32::<LittleEndian>()?;

            let header = LibraryHeader { count, version };

            // 读取索引
            let mut indices = Vec::with_capacity(count as usize);
            for _ in 0..count {
                let offset = file.read_u32::<LittleEndian>()?;
                indices.push(ImageIndex { offset });
            }

            Ok(Self {
                file_path: path_str,
                header,
                indices,
                cache: HashMap::new(),
                max_cache_size: 100, // 较小的缓存用于演示
                lru_order: Vec::new(),
            })
        }

        pub fn get_image(&mut self, index: usize) -> io::Result<Option<ImageData>> {
            if index >= self.indices.len() {
                return Ok(None);
            }

            // 检查缓存
            if let Some(cached) = self.cache.get(&index).cloned() {
                self.update_lru(index);
                return Ok(Some(cached));
            }

            let mut file = File::open(&self.file_path)?;
            let idx = &self.indices[index];

            if idx.offset == 0 {
                return Ok(None);
            }

            file.seek(SeekFrom::Start(idx.offset as u64))?;

            // 读取图像元数据 (17 字节)
            let width = file.read_i16::<LittleEndian>()?;
            let height = file.read_i16::<LittleEndian>()?;
            let offset_x = file.read_i16::<LittleEndian>()?;
            let offset_y = file.read_i16::<LittleEndian>()?;
            let shadow_x = file.read_u8()?;
            let shadow_y = file.read_u8()?;
            let shadow = file.read_u8()?;
            let length = file.read_i32::<LittleEndian>()?;

            if width <= 0 || height <= 0 || length <= 0 {
                return Ok(None);
            }

            // 读取压缩数据
            let mut compressed = vec![0u8; length as usize];
            file.read_exact(&mut compressed)?;

            // 解压
            let mut decoder = ZlibDecoder::new(&compressed[..]);
            let mut decompressed = Vec::new();
            decoder.read_to_end(&mut decompressed)?;

            // 转换 555 BGR -> RGBA
            let rgba_data =
                self.convert_555_to_rgba(&decompressed, width as usize, height as usize);

            // 处理阴影蒙版（如果存在）
            let mask = if shadow != 0 {
                if let Ok(mask_length) = file.read_i32::<LittleEndian>() {
                    if mask_length > 0 {
                        let mut mask_compressed = vec![0u8; mask_length as usize];
                        if file.read_exact(&mut mask_compressed).is_ok() {
                            let mut mask_decoder = ZlibDecoder::new(&mask_compressed[..]);
                            let mut mask_data = Vec::new();
                            if mask_decoder.read_to_end(&mut mask_data).is_ok() {
                                Some(MaskData {
                                    width: shadow_x as i16,
                                    height: shadow_y as i16,
                                    data: mask_data,
                                })
                            } else {
                                None
                            }
                        } else {
                            None
                        }
                    } else {
                        None
                    }
                } else {
                    None
                }
            } else {
                None
            };

            let image_data = ImageData {
                width,
                height,
                offset_x,
                offset_y,
                rgba_data,
                mask,
            };

            // 加入缓存
            self.cache.insert(index, image_data.clone());
            self.lru_order.push(index);

            // LRU 淘汰
            while self.cache.len() > self.max_cache_size && !self.lru_order.is_empty() {
                let oldest = self.lru_order.remove(0);
                self.cache.remove(&oldest);
            }

            Ok(Some(image_data))
        }

        fn update_lru(&mut self, index: usize) {
            self.lru_order.retain(|&x| x != index);
            self.lru_order.push(index);
        }

        fn convert_555_to_rgba(&self, data: &[u8], width: usize, height: usize) -> Vec<u8> {
            let mut rgba = Vec::with_capacity(width * height * 4);

            for chunk in data.chunks_exact(2) {
                let pixel = u16::from_le_bytes([chunk[0], chunk[1]]);

                // 555 格式: 0RRRRRGGGGGBBBBB
                let r = ((pixel >> 10) & 0x1F) as u8;
                let g = ((pixel >> 5) & 0x1F) as u8;
                let b = (pixel & 0x1F) as u8;

                // 扩展到 8 位
                let r8 = (r << 3) | (r >> 2);
                let g8 = (g << 3) | (g >> 2);
                let b8 = (b << 3) | (b >> 2);

                // 黑色 = 透明
                let a = if r == 0 && g == 0 && b == 0 { 0 } else { 255 };

                rgba.push(r8);
                rgba.push(g8);
                rgba.push(b8);
                rgba.push(a);
            }

            rgba
        }

        pub fn image_count(&self) -> usize {
            self.indices.len()
        }

        pub fn clear_cache(&mut self) {
            self.cache.clear();
            self.lru_order.clear();
        }
    }
}

use mlibrary_data::{ImageData, MLibraryData};

/// 纹理缓存
struct TextureCache {
    textures: HashMap<usize, Texture2D>,
}

impl TextureCache {
    fn new() -> Self {
        Self {
            textures: HashMap::new(),
        }
    }

    fn get_or_create(&mut self, index: usize, image_data: &ImageData) -> Texture2D {
        // 检查缓存，如果存在则返回克隆
        if let Some(texture) = self.textures.get(&index) {
            return texture.clone();
        }

        // 从 RGBA 数据创建纹理
        let texture = Texture2D::from_rgba8(
            image_data.width as u16,
            image_data.height as u16,
            &image_data.rgba_data,
        );

        // 插入缓存并返回克隆
        self.textures.insert(index, texture.clone());
        texture
    }
}

/// macroquad 窗口配置
fn window_conf() -> Conf {
    Conf {
        window_title: "Macroquad + MLibrary 演示".to_owned(),
        window_width: 1024,
        window_height: 768,
        window_resizable: true,
        ..Default::default()
    }
}

#[macroquad::main(window_conf)]
async fn main() {
    println!("🚀 Macroquad + MLibrary 演示启动");

    // 尝试加载 .lib 文件
    let lib_path = "Data/ChrSel.lib";
    let mut library = match MLibraryData::load(lib_path) {
        Ok(lib) => {
            println!("✅ 成功加载: {}", lib_path);
            println!("   - 图像数量: {}", lib.image_count());
            Some(lib)
        }
        Err(e) => {
            println!("❌ 无法加载 {}: {}", lib_path, e);
            println!("   继续运行演示（无图像）");
            None
        }
    };

    let mut texture_cache = TextureCache::new();
    let mut current_image_index = 0usize;
    let mut loaded_image: Option<ImageData> = None;

    // 尝试加载第一张图像
    if let Some(ref mut lib) = library {
        match lib.get_image(current_image_index) {
            Ok(Some(img)) => {
                println!(
                    "✅ 加载图像 #{}: {}x{}",
                    current_image_index, img.width, img.height
                );
                loaded_image = Some(img);
            }
            Ok(None) => println!("⚠️  图像 #{} 为空", current_image_index),
            Err(e) => println!("❌ 加载图像失败: {}", e),
        }
    }

    println!("✅ 初始化完成，进入主循环");
    println!("   - 方向键: 切换图像");
    println!("   - 空格: 清除缓存");
    println!("   - ESC: 退出");

    loop {
        let dt = get_frame_time();

        // ========== 输入处理 ==========
        if is_key_pressed(KeyCode::Escape) {
            break;
        }

        if is_key_pressed(KeyCode::Right) {
            if let Some(ref mut lib) = library {
                current_image_index = (current_image_index + 1) % lib.image_count();
                match lib.get_image(current_image_index) {
                    Ok(Some(img)) => {
                        println!(
                            "➡️  图像 #{}: {}x{}",
                            current_image_index, img.width, img.height
                        );
                        loaded_image = Some(img);
                    }
                    Ok(None) => {
                        println!("⚠️  图像 #{} 为空", current_image_index);
                        loaded_image = None;
                    }
                    Err(e) => println!("❌ 加载失败: {}", e),
                }
            }
        }

        if is_key_pressed(KeyCode::Left) {
            if let Some(ref mut lib) = library {
                current_image_index = if current_image_index == 0 {
                    lib.image_count() - 1
                } else {
                    current_image_index - 1
                };
                match lib.get_image(current_image_index) {
                    Ok(Some(img)) => {
                        println!(
                            "⬅️  图像 #{}: {}x{}",
                            current_image_index, img.width, img.height
                        );
                        loaded_image = Some(img);
                    }
                    Ok(None) => {
                        println!("⚠️  图像 #{} 为空", current_image_index);
                        loaded_image = None;
                    }
                    Err(e) => println!("❌ 加载失败: {}", e),
                }
            }
        }

        if is_key_pressed(KeyCode::Space) {
            if let Some(ref mut lib) = library {
                lib.clear_cache();
                texture_cache.textures.clear();
                println!("🗑️  缓存已清除");
            }
        }

        // ========== 渲染 ==========
        clear_background(Color::from_rgba(40, 40, 50, 255));

        let screen_w = screen_width();
        let screen_h = screen_height();

        // 绘制背景网格
        for x in (0..screen_w as i32).step_by(32) {
            draw_line(
                x as f32,
                0.0,
                x as f32,
                screen_h,
                1.0,
                Color::from_rgba(60, 60, 70, 255),
            );
        }
        for y in (0..screen_h as i32).step_by(32) {
            draw_line(
                0.0,
                y as f32,
                screen_w,
                y as f32,
                1.0,
                Color::from_rgba(60, 60, 70, 255),
            );
        }

        // 绘制加载的图像
        if let Some(ref img_data) = loaded_image {
            let texture = texture_cache.get_or_create(current_image_index, img_data);

            let x = (screen_w - img_data.width as f32) * 0.5 + img_data.offset_x as f32;
            let y = (screen_h - img_data.height as f32) * 0.5 + img_data.offset_y as f32;

            draw_texture(&texture, x, y, WHITE);

            // 显示边框
            draw_rectangle_lines(
                x,
                y,
                img_data.width as f32,
                img_data.height as f32,
                2.0,
                GREEN,
            );
        } else {
            // 无图像时显示提示
            let text = if library.is_none() {
                "未加载库文件"
            } else {
                "空图像"
            };
            let font_size = 30.0;
            let text_dim = measure_text(text, None, font_size as u16, 1.0);
            draw_text(
                text,
                (screen_w - text_dim.width) * 0.5,
                screen_h * 0.5,
                font_size,
                RED,
            );
        }

        // UI 信息
        let info_text = if let Some(ref lib) = library {
            format!(
                "图像: {}/{} | 缓存: {}  | FPS: {:.0}",
                current_image_index,
                lib.image_count(),
                texture_cache.textures.len(),
                get_fps()
            )
        } else {
            format!("无库文件 | FPS: {:.0}", get_fps())
        };

        draw_text(&info_text, 10.0, 30.0, 20.0, WHITE);

        draw_text(
            "← → : 切换图像 | 空格: 清除缓存 | ESC: 退出",
            10.0,
            screen_h - 10.0,
            16.0,
            Color::from_rgba(200, 200, 200, 255),
        );

        next_frame().await;
    }

    println!("👋 演示结束");
}
