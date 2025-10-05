// Ggez Graphics Manager - 简化版本
// 仅提供纹理管理功能，渲染由 Canvas 直接完成

use ggez::{Context, GameResult};
use ggez::graphics::{self, Image};
use std::collections::HashMap;

/// Ggez图形管理器（简化版）
/// 
/// 主要功能:
/// 1. 纹理缓存管理
/// 2. MLibrary 集成 (从像素数据创建纹理)
///  
/// 渲染直接使用 ggez Canvas API:
/// ```rust
/// let mut canvas = Canvas::from_frame(ctx, Color::BLACK);
/// canvas.draw(&image, DrawParam::default().dest([x, y]));
/// canvas.finish(ctx)?;
/// ```
pub struct GgezManager {
    /// 纹理缓存 (key -> Image)
    textures: HashMap<String, Image>,
    
    /// 屏幕尺寸
    screen_width: f32,
    screen_height: f32,
    
    /// 帧统计
    draw_calls: u32,
}

impl GgezManager {
    /// 创建新的图形管理器
    pub fn new(screen_width: f32, screen_height: f32) -> Self {
        tracing::info!("Initializing GgezManager ({}x{})", screen_width, screen_height);
        
        Self {
            textures: HashMap::new(),
            screen_width,
            screen_height,
            draw_calls: 0,
        }
    }
    
    /// 获取屏幕尺寸
    pub fn screen_size(&self) -> (f32, f32) {
        (self.screen_width, self.screen_height)
    }
    
    /// 更新屏幕尺寸
    pub fn update_screen_size(&mut self, width: f32, height: f32) {
        self.screen_width = width;
        self.screen_height = height;
    }
    
    /// 开始新的一帧
    pub fn begin_frame(&mut self) {
        self.draw_calls = 0;
    }
    
    /// 结束当前帧
    pub fn end_frame(&mut self) {
        tracing::trace!("Frame completed with {} draw calls", self.draw_calls);
    }
    
    /// 记录一次绘制调用
    pub fn inc_draw_call(&mut self) {
        self.draw_calls += 1;
    }
    
    /// 获取draw call计数
    pub fn draw_call_count(&self) -> u32 {
        self.draw_calls
    }
    
    //=== 纹理管理 ===
    
    /// 从文件加载纹理
    pub fn load_texture(&mut self, ctx: &mut Context, path: &str) -> GameResult<&Image> {
        if !self.textures.contains_key(path) {
            tracing::debug!("Loading texture: {}", path);
            let image = Image::from_path(ctx, path)?;
            self.textures.insert(path.to_string(), image);
        }
        
        Ok(self.textures.get(path).unwrap())
    }
    
    /// 从RGBA像素数据创建纹理 (用于MLibrary集成)
    /// 
    /// # Arguments
    /// * `ctx` - ggez上下文
    /// * `width` - 图像宽度
    /// * `height` - 图像高度
    /// * `pixels` - RGBA格式像素数据 (长度必须为 width * height * 4)
    /// * `key` - 纹理缓存key
    /// 
    /// # Example
    /// ```rust
    /// // 从 MLibrary 加载图片
    /// let (width, height, pixels) = mlibrary.get_image_data(index)?;
    /// let image = ggez_manager.create_texture_from_rgba(
    ///     ctx, width, height, &pixels, format!("lib_{}", index)
    /// )?;
    /// ```
    pub fn create_texture_from_rgba(
        &mut self,
        ctx: &mut Context,
        width: u16,
        height: u16,
        pixels: &[u8],
        key: String,
    ) -> GameResult<&Image> {
        if !self.textures.contains_key(&key) {
            tracing::debug!("Creating texture from RGBA: {} ({}x{})", key, width, height);
            
            // 创建 Image (ggez 0.10 API)
            let image = Image::from_pixels(
                ctx,
                pixels,
                ggez::graphics::ImageFormat::Rgba8UnormSrgb,
                width as u32,
                height as u32,
            );
            
            self.textures.insert(key.clone(), image);
        }
        
        Ok(self.textures.get(&key).unwrap())
    }
    
    /// 获取已加载的纹理
    pub fn get_texture(&self, key: &str) -> Option<&Image> {
        self.textures.get(key)
    }
    
    /// 移除特定纹理
    pub fn remove_texture(&mut self, key: &str) -> bool {
        self.textures.remove(key).is_some()
    }
    
    /// 清空纹理缓存
    pub fn clear_texture_cache(&mut self) {
        tracing::info!("Clearing texture cache ({} textures)", self.textures.len());
        self.textures.clear();
    }
    
    /// 获取纹理缓存统计
    pub fn texture_cache_stats(&self) -> (usize, usize) {
        let count = self.textures.len();
        // 估算内存占用 (假设平均每个纹理 256KB)
        let memory_bytes = count * 256 * 1024;
        (count, memory_bytes)
    }
}

// 重新导出 ggez graphics types (方便使用)
pub use ggez::graphics::{Canvas, DrawParam, Color, Rect, Text, Mesh, DrawMode};
