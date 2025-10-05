// Ggez Graphics Manager - 替代wgpu的DXManager
// 提供更高层次、更易用的2D渲染API
//
// 注意: ggez 0.10 使用 Canvas API - 所有渲染通过 Canvas 进行

use ggez::{Context, GameResult};
use ggez::graphics::{self, DrawParam, Image, Text, Color, Rect, Canvas, Mesh, DrawMode};
use std::collections::HashMap;

/// Ggez图形管理器
/// 
/// 替代原来的DXManager，提供简化的渲染API
/// 
/// **使用方式**:
/// ```rust
/// impl EventHandler for Game {
///     fn draw(&mut self, ctx: &mut Context) -> GameResult {
///         let mut canvas = Canvas::from_frame(ctx, Color::BLACK);
///         self.ggez_manager.draw_sprite_on_canvas(&mut canvas, &image, x, y)?;
///         canvas.finish(ctx)?;
///         Ok(())
///     }
/// }
/// ```
pub struct GgezManager {
    /// 纹理缓存 (key -> Image)
    textures: HashMap<String, Image>,
    
    /// 屏幕尺寸
    screen_width: f32,
    screen_height: f32,
    
    /// 统计信息
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
    
    /// 更新屏幕尺寸（窗口调整时）
    pub fn update_screen_size(&mut self, width: f32, height: f32) {
        self.screen_width = width;
        self.screen_height = height;
    }
    
    /// 开始新的一帧 (重置统计)
    pub fn begin_frame(&mut self) {
        self.draw_calls = 0;
    }
    
    /// 结束当前帧
    pub fn end_frame(&mut self) {
        tracing::trace!("Frame completed with {} draw calls", self.draw_calls);
    }
    
    /// 加载纹理（从文件路径）
    pub fn load_texture(&mut self, ctx: &mut Context, path: &str) -> GameResult<&Image> {
        // 如果已经加载，直接返回
        if !self.textures.contains_key(path) {
            tracing::debug!("Loading texture: {}", path);
            let image = Image::from_path(ctx, path)?;
            self.textures.insert(path.to_string(), image);
        }
        
        Ok(self.textures.get(path).unwrap())
    }
    
    /// 从原始RGBA像素数据创建纹理
    /// 
    /// # Arguments
    /// * `ctx` - ggez上下文
    /// * `width` - 图像宽度
    /// * `height` - 图像高度
    /// * `pixels` - RGBA格式像素数据 (长度必须为 width * height * 4)
    /// * `key` - 纹理缓存key（用于后续引用）
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
            let image = Image::from_rgba8(ctx, width, height, pixels)?;
            self.textures.insert(key.clone(), image);
        }
        
        Ok(self.textures.get(&key).unwrap())
    }
    
    /// 获取已加载的纹理
    pub fn get_texture(&self, key: &str) -> Option<&Image> {
        self.textures.get(key)
    }
    
    /// 清空纹理缓存
    pub fn clear_texture_cache(&mut self) {
        tracing::info!("Clearing texture cache ({} textures)", self.textures.len());
        self.textures.clear();
    }
    
    /// 移除特定纹理
    pub fn remove_texture(&mut self, key: &str) -> bool {
        self.textures.remove(key).is_some()
    }
    
    /// 开始新的一帧
    pub fn begin_frame(&mut self, ctx: &mut Context, clear_color: Color) {
        self.draw_calls = 0;
        graphics::clear(ctx, clear_color);
    }
    
    /// 结束当前帧并呈现到屏幕
    pub fn end_frame(&mut self, ctx: &mut Context) -> GameResult<()> {
        graphics::present(ctx)?;
        tracing::trace!("Frame completed with {} draw calls", self.draw_calls);
        Ok(())
    }
    
    /// 绘制精灵
    /// 
    /// # Arguments
    /// * `image` - 要绘制的图像
    /// * `x`, `y` - 屏幕坐标
    /// * `scale` - 缩放比例 (1.0 = 原始大小)
    /// * `rotation` - 旋转角度（弧度）
    /// * `color` - 颜色调制
    pub fn draw_sprite(
        &mut self,
        ctx: &mut Context,
        image: &Image,
        x: f32,
        y: f32,
        scale: f32,
        rotation: f32,
        color: Color,
    ) -> GameResult<()> {
        let params = DrawParam::default()
            .dest([x, y])
            .scale([scale, scale])
            .rotation(rotation)
            .color(color);
        
        graphics::draw(ctx, image, params)?;
        self.draw_calls += 1;
        Ok(())
    }
    
    /// 绘制精灵（简化版，使用默认参数）
    pub fn draw_sprite_simple(
        &mut self,
        ctx: &mut Context,
        image: &Image,
        x: f32,
        y: f32,
    ) -> GameResult<()> {
        self.draw_sprite(ctx, image, x, y, 1.0, 0.0, Color::WHITE)
    }
    
    /// 绘制精灵（带透明度）
    pub fn draw_sprite_alpha(
        &mut self,
        ctx: &mut Context,
        image: &Image,
        x: f32,
        y: f32,
        alpha: f32,
    ) -> GameResult<()> {
        let color = Color::new(1.0, 1.0, 1.0, alpha);
        self.draw_sprite(ctx, image, x, y, 1.0, 0.0, color)
    }
    
    /// 绘制文本
    pub fn draw_text(
        &mut self,
        ctx: &mut Context,
        text: &str,
        x: f32,
        y: f32,
        font_size: f32,
        color: Color,
    ) -> GameResult<()> {
        let mut text_obj = Text::new(text);
        text_obj.set_font(self.default_font);
        text_obj.set_scale(font_size);
        
        let params = DrawParam::default()
            .dest([x, y])
            .color(color);
        
        graphics::draw(ctx, &text_obj, params)?;
        self.draw_calls += 1;
        Ok(())
    }
    
    /// 绘制矩形（填充）
    pub fn draw_rect_filled(
        &mut self,
        ctx: &mut Context,
        x: f32,
        y: f32,
        width: f32,
        height: f32,
        color: Color,
    ) -> GameResult<()> {
        let rect = Rect::new(x, y, width, height);
        let mesh = graphics::Mesh::new_rectangle(
            ctx,
            graphics::DrawMode::fill(),
            rect,
            color,
        )?;
        
        graphics::draw(ctx, &mesh, DrawParam::default())?;
        self.draw_calls += 1;
        Ok(())
    }
    
    /// 绘制矩形（边框）
    pub fn draw_rect_outline(
        &mut self,
        ctx: &mut Context,
        x: f32,
        y: f32,
        width: f32,
        height: f32,
        line_width: f32,
        color: Color,
    ) -> GameResult<()> {
        let rect = Rect::new(x, y, width, height);
        let mesh = graphics::Mesh::new_rectangle(
            ctx,
            graphics::DrawMode::stroke(line_width),
            rect,
            color,
        )?;
        
        graphics::draw(ctx, &mesh, DrawParam::default())?;
        self.draw_calls += 1;
        Ok(())
    }
    
    /// 绘制线段
    pub fn draw_line(
        &mut self,
        ctx: &mut Context,
        x1: f32,
        y1: f32,
        x2: f32,
        y2: f32,
        line_width: f32,
        color: Color,
    ) -> GameResult<()> {
        let points = vec![[x1, y1].into(), [x2, y2].into()];
        let mesh = graphics::Mesh::new_line(
            ctx,
            &points,
            line_width,
            color,
        )?;
        
        graphics::draw(ctx, &mesh, DrawParam::default())?;
        self.draw_calls += 1;
        Ok(())
    }
    
    /// 获取当前帧的绘制调用次数
    pub fn draw_call_count(&self) -> u32 {
        self.draw_calls
    }
    
    /// 获取纹理缓存统计
    pub fn texture_cache_stats(&self) -> (usize, usize) {
        let count = self.textures.len();
        let memory = self.textures.values()
            .map(|img| {
                let (w, h) = (img.width(), img.height());
                (w * h * 4) as usize // RGBA估算
            })
            .sum();
        
        (count, memory)
    }
}

impl std::fmt::Debug for GgezManager {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("GgezManager")
            .field("screen_size", &(self.screen_width, self.screen_height))
            .field("texture_count", &self.textures.len())
            .field("draw_calls", &self.draw_calls)
            .finish()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    
    // 注意：ggez测试需要图形上下文，通常在集成测试中进行
    // 这里只做基本的结构测试
    
    #[test]
    fn test_texture_cache_stats() {
        // 创建模拟的GgezManager（无ctx）
        let manager = GgezManager {
            textures: HashMap::new(),
            default_font: graphics::Font::default(),
            screen_width: 1024.0,
            screen_height: 768.0,
            draw_calls: 0,
        };
        
        let (count, _memory) = manager.texture_cache_stats();
        assert_eq!(count, 0);
    }
}
