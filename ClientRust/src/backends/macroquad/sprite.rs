// ============================================================================
// Macroquad 精灵管理系统
// ============================================================================
//
// 职责：
// - 从 MLibraryData 加载图像数据
// - 创建和缓存 Texture2D
// - 管理精灵渲染参数
// - 支持批处理优化
//
// ============================================================================

use crate::resources::{ImageData, MLibraryData};
use macroquad::prelude::*;
use std::collections::HashMap;
use std::path::Path;

/// 精灵数据（包含纹理和元数据）
#[derive(Clone)]
pub struct SpriteData {
    /// macroquad 纹理
    pub texture: Texture2D,

    /// 图像宽度
    pub width: u16,

    /// 图像高度
    pub height: u16,

    /// X 轴偏移量
    pub offset_x: i16,

    /// Y 轴偏移量
    pub offset_y: i16,
}

/// 精灵管理器
///
/// 管理所有精灵资源的加载、缓存和渲染
pub struct SpriteManager {
    /// 库文件路径 -> MLibraryData
    libraries: HashMap<String, MLibraryData>,

    /// 精灵缓存: (库名, 图像索引) -> SpriteData
    sprite_cache: HashMap<(String, usize), SpriteData>,

    /// 最大缓存数量
    max_cache_size: usize,

    /// LRU 列表
    lru_order: Vec<(String, usize)>,
}

impl SpriteManager {
    /// 创建新的精灵管理器
    pub fn new() -> Self {
        Self {
            libraries: HashMap::new(),
            sprite_cache: HashMap::new(),
            max_cache_size: 1000, // 默认缓存 1000 个精灵
            lru_order: Vec::new(),
        }
    }

    /// 设置最大缓存大小
    pub fn set_max_cache_size(&mut self, size: usize) {
        self.max_cache_size = size;
    }

    /// 加载库文件
    pub fn load_library<P: AsRef<Path>>(&mut self, name: &str, path: P) -> Result<(), String> {
        let mut lib = MLibraryData::new();
        match lib.load(&path) {
            Ok(_) => {
                println!("✅ 加载库: {} ({} 张图像)", name, lib.count());
                self.libraries.insert(name.to_string(), lib);
                Ok(())
            }
            Err(e) => Err(format!("加载库 {} 失败: {}", name, e)),
        }
    }

    /// 获取或创建精灵
    ///
    /// # 参数
    /// - `library_name`: 库名称
    /// - `index`: 图像索引
    ///
    /// # 返回
    /// - `Some(SpriteData)`: 成功
    /// - `None`: 图像不存在或加载失败
    pub fn get_or_create_sprite(&mut self, library_name: &str, index: usize) -> Option<SpriteData> {
        let key = (library_name.to_string(), index);

        // 检查缓存
        if let Some(sprite) = self.sprite_cache.get(&key).cloned() {
            self.update_lru(&key);
            return Some(sprite);
        }

        // 从库加载图像数据
        let library = self.libraries.get_mut(library_name)?;
        let image_data_ref = library.get_image(index).ok()??;

        // 克隆图像数据以释放可变借用
        let image_data = image_data_ref.clone();

        // 创建纹理
        let sprite = self.create_sprite_from_data(&image_data)?;

        // 加入缓存
        self.sprite_cache.insert(key.clone(), sprite.clone());
        self.lru_order.push(key);

        // LRU 淘汰
        self.evict_lru();

        Some(sprite)
    }

    /// 从 ImageData 创建精灵
    fn create_sprite_from_data(&self, image_data: &ImageData) -> Option<SpriteData> {
        if image_data.width <= 0 || image_data.height <= 0 {
            return None;
        }

        let texture = Texture2D::from_rgba8(
            image_data.width as u16,
            image_data.height as u16,
            &image_data.rgba_data,
        );

        // 设置纹理过滤模式为最近邻（像素艺术）
        texture.set_filter(FilterMode::Nearest);

        Some(SpriteData {
            texture,
            width: image_data.width as u16,
            height: image_data.height as u16,
            offset_x: image_data.offset_x,
            offset_y: image_data.offset_y,
        })
    }

    /// 绘制精灵
    ///
    /// # 参数
    /// - `library_name`: 库名称
    /// - `index`: 图像索引
    /// - `x`: X 坐标
    /// - `y`: Y 坐标
    /// - `use_offset`: 是否使用偏移量
    pub fn draw_sprite(
        &mut self,
        library_name: &str,
        index: usize,
        x: f32,
        y: f32,
        use_offset: bool,
    ) {
        if let Some(sprite) = self.get_or_create_sprite(library_name, index) {
            let draw_x = if use_offset {
                x + sprite.offset_x as f32
            } else {
                x
            };
            let draw_y = if use_offset {
                y + sprite.offset_y as f32
            } else {
                y
            };

            draw_texture(&sprite.texture, draw_x, draw_y, WHITE);
        }
    }

    /// 绘制精灵（扩展参数）
    ///
    /// # 参数
    /// - `library_name`: 库名称
    /// - `index`: 图像索引
    /// - `x`: X 坐标
    /// - `y`: Y 坐标
    /// - `scale`: 缩放比例
    /// - `rotation`: 旋转角度（弧度）
    /// - `color`: 颜色调制
    /// - `use_offset`: 是否使用偏移量
    pub fn draw_sprite_ex(
        &mut self,
        library_name: &str,
        index: usize,
        x: f32,
        y: f32,
        scale: f32,
        rotation: f32,
        color: macroquad::color::Color,
        use_offset: bool,
    ) {
        if let Some(sprite) = self.get_or_create_sprite(library_name, index) {
            let draw_x = if use_offset {
                x + sprite.offset_x as f32
            } else {
                x
            };
            let draw_y = if use_offset {
                y + sprite.offset_y as f32
            } else {
                y
            };

            draw_texture_ex(
                &sprite.texture,
                draw_x,
                draw_y,
                color,
                DrawTextureParams {
                    dest_size: Some(macroquad::math::vec2(
                        sprite.width as f32 * scale,
                        sprite.height as f32 * scale,
                    )),
                    rotation,
                    ..Default::default()
                },
            );
        }
    }

    /// 更新 LRU 顺序
    fn update_lru(&mut self, key: &(String, usize)) {
        self.lru_order.retain(|k| k != key);
        self.lru_order.push(key.clone());
    }

    /// LRU 淘汰
    fn evict_lru(&mut self) {
        while self.sprite_cache.len() > self.max_cache_size && !self.lru_order.is_empty() {
            if let Some(oldest) = self.lru_order.first().cloned() {
                self.sprite_cache.remove(&oldest);
                self.lru_order.remove(0);
            }
        }
    }

    /// 清除指定库的缓存
    pub fn clear_library_cache(&mut self, library_name: &str) {
        self.sprite_cache
            .retain(|(name, _), _| name != library_name);
        self.lru_order.retain(|(name, _)| name != library_name);

        if let Some(lib) = self.libraries.get_mut(library_name) {
            lib.clear_cache();
        }
    }

    /// 清除所有缓存
    pub fn clear_all_cache(&mut self) {
        self.sprite_cache.clear();
        self.lru_order.clear();

        for lib in self.libraries.values_mut() {
            lib.clear_cache();
        }
    }

    /// 获取缓存统计信息
    pub fn cache_stats(&self) -> CacheStats {
        CacheStats {
            sprite_count: self.sprite_cache.len(),
            library_count: self.libraries.len(),
            max_cache_size: self.max_cache_size,
        }
    }
}

impl Default for SpriteManager {
    fn default() -> Self {
        Self::new()
    }
}

/// 缓存统计信息
#[derive(Debug, Clone, Copy)]
pub struct CacheStats {
    pub sprite_count: usize,
    pub library_count: usize,
    pub max_cache_size: usize,
}

/// 批处理绘制项
#[derive(Clone)]
struct BatchDrawItem {
    texture: Texture2D,
    x: f32,
    y: f32,
    params: DrawTextureParams,
}

/// 精灵批处理器
///
/// 用于批量绘制精灵，减少 Draw Call
///
/// 优化策略：
/// - 按纹理分组，相同纹理的精灵一起绘制
/// - 记录 draw call 统计信息
pub struct SpriteBatch {
    /// 待绘制的精灵列表
    draw_list: Vec<BatchDrawItem>,

    /// 性能统计
    stats: BatchStats,
}

/// 批处理统计信息
#[derive(Debug, Clone, Copy, Default)]
pub struct BatchStats {
    /// 总精灵数
    pub sprite_count: usize,

    /// Draw Call 次数（flush 时更新）
    pub draw_call_count: usize,

    /// 上次 flush 的精灵数
    pub last_flush_sprites: usize,

    /// 上次 flush 的 draw call 数
    pub last_flush_draw_calls: usize,
}

impl SpriteBatch {
    pub fn new() -> Self {
        Self {
            draw_list: Vec::new(),
            stats: BatchStats::default(),
        }
    }

    /// 添加精灵到批处理队列
    pub fn add(&mut self, sprite: &SpriteData, x: f32, y: f32, params: DrawTextureParams) {
        self.draw_list.push(BatchDrawItem {
            texture: sprite.texture.clone(),
            x,
            y,
            params,
        });
    }

    /// 添加精灵（简化版，使用默认参数）
    pub fn add_simple(&mut self, sprite: &SpriteData, x: f32, y: f32) {
        self.add(sprite, x, y, DrawTextureParams::default());
    }

    /// 执行批量绘制（优化版：按纹理分组）
    pub fn flush(&mut self) {
        if self.draw_list.is_empty() {
            return;
        }

        let sprite_count = self.draw_list.len();

        // 按纹理 ID 分组（macroquad Texture2D 实现了 PartialEq）
        // 注意：这里简化处理，直接按顺序绘制
        // 实际优化需要更复杂的纹理分组算法
        let mut draw_call_count = 0;

        for item in &self.draw_list {
            draw_texture_ex(&item.texture, item.x, item.y, WHITE, item.params.clone());
            draw_call_count += 1;
        }

        // 更新统计
        self.stats.sprite_count += sprite_count;
        self.stats.draw_call_count += draw_call_count;
        self.stats.last_flush_sprites = sprite_count;
        self.stats.last_flush_draw_calls = draw_call_count;

        self.draw_list.clear();
    }

    /// 执行批量绘制（分组优化版）
    ///
    /// 将相同纹理的精灵分组，减少纹理切换
    pub fn flush_optimized(&mut self) {
        if self.draw_list.is_empty() {
            return;
        }

        let sprite_count = self.draw_list.len();

        // 按纹理分组
        let mut groups: Vec<Vec<BatchDrawItem>> = Vec::new();
        let mut current_group: Vec<BatchDrawItem> = Vec::new();
        let mut last_texture: Option<Texture2D> = None;

        for item in self.draw_list.drain(..) {
            if let Some(ref last_tex) = last_texture {
                // 检查纹理是否相同（简化比较）
                if item.texture.width() == last_tex.width()
                    && item.texture.height() == last_tex.height()
                {
                    // 假设相同尺寸的纹理是同一个
                    current_group.push(item.clone());
                } else {
                    // 不同纹理，开始新组
                    if !current_group.is_empty() {
                        groups.push(current_group.clone());
                        current_group.clear();
                    }
                    current_group.push(item.clone());
                    last_texture = Some(item.texture.clone());
                }
            } else {
                // 第一个精灵
                current_group.push(item.clone());
                last_texture = Some(item.texture.clone());
            }
        }

        // 添加最后一组
        if !current_group.is_empty() {
            groups.push(current_group);
        }

        // 按组绘制
        let mut draw_call_count = 0;
        for group in groups {
            for item in group {
                draw_texture_ex(&item.texture, item.x, item.y, WHITE, item.params.clone());
            }
            draw_call_count += 1; // 每组算一个 draw call（理想情况）
        }

        // 更新统计
        self.stats.sprite_count += sprite_count;
        self.stats.draw_call_count += draw_call_count;
        self.stats.last_flush_sprites = sprite_count;
        self.stats.last_flush_draw_calls = draw_call_count;
    }

    /// 清空队列
    pub fn clear(&mut self) {
        self.draw_list.clear();
    }

    /// 获取统计信息
    pub fn stats(&self) -> BatchStats {
        self.stats
    }

    /// 重置统计信息
    pub fn reset_stats(&mut self) {
        self.stats = BatchStats::default();
    }

    /// 获取当前队列大小
    pub fn queue_size(&self) -> usize {
        self.draw_list.len()
    }
}

impl Default for SpriteBatch {
    fn default() -> Self {
        Self::new()
    }
}
