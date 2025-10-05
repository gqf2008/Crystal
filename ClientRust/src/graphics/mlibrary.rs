// MLibrary - MIR2 图像库加载器
// 对应: Client/MirGraphics/MLibrary.cs
//
// 负责解析和加载 .lib 文件格式（MIR2 专有的图像库格式）

use std::collections::HashMap;
use std::fs::File;
use std::io::{self, Read, Seek, SeekFrom, BufReader};
use std::path::{Path, PathBuf};
use std::sync::Arc;
use flate2::read::GzDecoder;

// 移除 wgpu 依赖
// TextureHandle 已废弃,MLibrary 直接返回像素数据
// use super::dx_manager::TextureHandle;

// Dummy TextureHandle 用于编译兼容性 (实际使用 ggez Image)
#[derive(Debug, Clone)]
pub struct TextureHandle {
    pub width: u32,
    pub height: u32,
}

/// MIR2图像库文件头
#[derive(Debug, Clone)]
pub struct LibraryHeader {
    pub version: i32,
    pub count: i32,
    pub frame_seek: i32,
}

/// 图像索引项
#[derive(Debug, Clone)]
pub struct ImageIndex {
    pub offset: i32,  // 文件中的偏移量
}

/// 图像元数据(不包含纹理数据)
#[derive(Debug, Clone)]
pub struct ImageInfo {
    pub width: i16,
    pub height: i16,
    pub x: i16,  // 偏移量X
    pub y: i16,  // 偏移量Y
    pub shadow_x: i16,
    pub shadow_y: i16,
    pub shadow: u8,
    pub length: i32,  // 压缩数据长度
    pub has_mask: bool,  // 是否有第二层
}

/// MIR2图像库
pub struct MLibrary {
    path: PathBuf,
    header: LibraryHeader,
    indices: Vec<ImageIndex>,
    // 缓存已加载的图像信息
    cached_info: HashMap<usize, ImageInfo>,
    // 纹理缓存 - 避免重复加载
    texture_cache: HashMap<usize, Arc<TextureHandle>>,
    // 缓存清理时间戳
    cache_timestamps: HashMap<usize, i64>,
}

impl MLibrary {
    /// 打开.lib文件
    pub fn open<P: AsRef<Path>>(path: P) -> io::Result<Self> {
        let mut path_buf = path.as_ref().to_path_buf();
        
        // 尝试.Lib扩展名
        if !path_buf.exists() {
            path_buf.set_extension("Lib");
        }
        
        let mut file = File::open(&path_buf)?;
        let mut reader = BufReader::new(&mut file);
        
        // 读取文件头
        let version = read_i32(&mut reader)?;
        let count = read_i32(&mut reader)?;
        let frame_seek = read_i32(&mut reader)?;
        
        let header = LibraryHeader {
            version,
            count,
            frame_seek,
        };
        
        // 读取索引表
        let mut indices = Vec::with_capacity(count as usize);
        for _ in 0..count {
            let offset = read_i32(&mut reader)?;
            indices.push(ImageIndex { offset });
        }
        
        Ok(Self {
            path: path_buf,
            header,
            indices,
            cached_info: HashMap::new(),
            texture_cache: HashMap::new(),
            cache_timestamps: HashMap::new(),
        })
    }
    
    /// 获取图像数量
    pub fn count(&self) -> usize {
        self.indices.len()
    }
    
    /// 读取图像信息(不解压纹理数据)
    pub fn get_image_info(&mut self, index: usize) -> io::Result<ImageInfo> {
        if index >= self.indices.len() {
            return Err(io::Error::new(
                io::ErrorKind::InvalidInput,
                format!("Image index {} out of range (max: {})", index, self.indices.len())
            ));
        }
        
        // 检查缓存
        if let Some(info) = self.cached_info.get(&index) {
            return Ok(info.clone());
        }
        
        let offset = self.indices[index].offset as u64;
        let mut file = File::open(&self.path)?;
        file.seek(SeekFrom::Start(offset))?;
        
        let width = read_i16(&mut file)?;
        let height = read_i16(&mut file)?;
        let x = read_i16(&mut file)?;
        let y = read_i16(&mut file)?;
        let shadow_x = read_i16(&mut file)?;
        let shadow_y = read_i16(&mut file)?;
        let shadow = read_u8(&mut file)?;
        let length = read_i32(&mut file)?;
        
        let has_mask = (shadow >> 7) == 1;
        
        let info = ImageInfo {
            width,
            height,
            x,
            y,
            shadow_x,
            shadow_y,
            shadow,
            length,
            has_mask,
        };
        
        self.cached_info.insert(index, info.clone());
        Ok(info)
    }
    
    /// 加载图像数据为RGBA8像素数组
    pub fn load_image_data(&mut self, index: usize) -> io::Result<(ImageInfo, Vec<u8>)> {
        let info = self.get_image_info(index)?;
        
        let offset = self.indices[index].offset as u64;
        let mut file = File::open(&self.path)?;
        
        // 跳过图像信息头(17字节)
        file.seek(SeekFrom::Start(offset + 17))?;
        
        // 读取压缩数据
        let mut compressed = vec![0u8; info.length as usize];
        file.read_exact(&mut compressed)?;
        
        // 解压(GZip格式)
        let mut decompressor = GzDecoder::new(&compressed[..]);
        let mut decompressed = Vec::new();
        decompressor.read_to_end(&mut decompressed)?;
        
        // 验证解压后的大小
        let expected_size = (info.width as usize) * (info.height as usize) * 4;
        if decompressed.len() != expected_size {
            return Err(io::Error::new(
                io::ErrorKind::InvalidData,
                format!(
                    "Decompressed size mismatch: expected {}, got {}",
                    expected_size,
                    decompressed.len()
                )
            ));
        }
        
        Ok((info, decompressed))
    }
    
    /// 加载图像数据为 RGBA 格式
    /// 
    /// C# equivalent: MLibrary 内部解压逻辑
    /// 
    /// 返回: (ImageInfo, RGBA字节数据)
    pub fn load_rgba_data(&mut self, index: usize) -> io::Result<(ImageInfo, Vec<u8>)> {
        let (info, data) = self.load_image_data(index)?;
        
        // MIR2使用BGRA格式,需要转换为RGBA
        let mut rgba_data = Vec::with_capacity(data.len());
        for chunk in data.chunks_exact(4) {
            let b = chunk[0];
            let g = chunk[1];
            let r = chunk[2];
            let a = chunk[3];
            rgba_data.push(r);
            rgba_data.push(g);
            rgba_data.push(b);
            rgba_data.push(a);
        }
        
        Ok((info, rgba_data))
    }
    
    // ===== Ggez 渲染函数 =====
    
    /// 使用 ggez 渲染图像到 Canvas
    /// 
    /// # 参数
    /// - `ctx`: ggez Context
    /// - `canvas`: 目标 Canvas
    /// - `index`: 图像索引
    /// - `x`, `y`: 屏幕坐标
    /// - `blend`: 是否使用混合模式
    /// 
    /// # 返回
    /// - `Ok(())`: 成功
    /// - `Err`: 图像不存在或渲染失败
    pub fn draw_to_canvas(
        &mut self,
        ctx: &mut ggez::Context,
        canvas: &mut ggez::graphics::Canvas,
        index: usize,
        x: f32,
        y: f32,
        blend: bool,
    ) -> io::Result<()> {
        use ggez::graphics::{Image, DrawParam, BlendMode};
        
        // 加载 RGBA 数据
        let (info, rgba_data) = self.load_rgba_data(index)?;
        
        // 创建 ggez Image (ggez 0.10 API)
        let image = Image::from_pixels(
            ctx,
            &rgba_data,
            ggez::graphics::ImageFormat::Rgba8UnormSrgb,
            info.width as u32,
            info.height as u32,
        );
        
        // 设置绘制参数
        let draw_param = DrawParam::default()
            .dest([x, y]);
        
        // blend 模式在 ggez 0.10 中通过 Canvas 设置,这里暂时忽略
        let _ = blend; // 避免未使用警告
        
        // 绘制
        canvas.draw(&image, draw_param);
        
        Ok(())
    }
    
    /// 使用 ggez 渲染图像到 Canvas (带偏移)
    /// 
    /// 用于处理 ImageInfo 中的 offset_x/offset_y
    pub fn draw_to_canvas_with_offset(
        &mut self,
        ctx: &mut ggez::Context,
        canvas: &mut ggez::graphics::Canvas,
        index: usize,
        x: f32,
        y: f32,
        blend: bool,
    ) -> io::Result<()> {
        // 先加载 RGBA 数据以获取 ImageInfo
        let (info, rgba_data) = self.load_rgba_data(index)?;
        let offset_x = info.x as f32; // 使用正确的字段名
        let offset_y = info.y as f32;
        
        // 创建并绘制 Image
        use ggez::graphics::{Image, DrawParam};
        
        let image = Image::from_pixels(
            ctx,
            &rgba_data,
            ggez::graphics::ImageFormat::Rgba8UnormSrgb,
            info.width as u32,
            info.height as u32,
        );
        
        let draw_param = DrawParam::default()
            .dest([x + offset_x, y + offset_y]);
        
        let _ = blend; // blend 模式暂时忽略
        
        canvas.draw(&image, draw_param);
        
        Ok(())
    }
}

/// 纹理管理器 - 负责加载和缓存所有游戏纹理
/// 
/// C# equivalent: 部分对应 DXManager.TextureList + MLibrary 的组合使用
/// C# 中纹理管理分散在多个地方，Rust 统一到这里
pub struct TextureManager {
    libraries: HashMap<String, MLibrary>,
    textures: HashMap<TextureKey, Arc<TextureHandle>>,
}

#[derive(Debug, Clone, Hash, PartialEq, Eq)]
pub struct TextureKey {
    pub library: String,
    pub index: usize,
}

impl TextureManager {
    pub fn new() -> Self {
        Self {
            libraries: HashMap::new(),
            textures: HashMap::new(),
        }
    }
    
    /// 加载图像库
    /// 
    /// C# equivalent: 
    /// ```csharp
    /// Libraries.Add(LibraryType.UI, new MLibrary(Path.Combine(Settings.DataPath, "UI")));
    /// ```
    pub fn load_library(&mut self, name: &str, path: &Path) -> io::Result<()> {
        let lib = MLibrary::open(path)?;
        self.libraries.insert(name.to_string(), lib);
        Ok(())
    }
    
    /// 获取或加载纹理
    /// 
    /// C# equivalent: 内部逻辑类似 MLibrary.GetTexture() + DXManager caching
    /// 
    /// 参数:
    /// - dx_manager: DXManager 引用，用于上传纹理到 GPU
    /// - library: 库名称 (如 "UI", "Prguse", "Tiles" 等)
    /// - index: 图像索引
    /// 
    /// 返回: (ImageInfo, Arc<TextureHandle>)
    // 已废弃: 使用 ggez 替代
    #[allow(dead_code)]
    pub fn get_texture(
        &mut self,
        _dx_manager: &TextureHandle, // Dummy type to avoid compilation error
        library: &str,
        index: usize,
    ) -> io::Result<(ImageInfo, Arc<TextureHandle>)> {
        // 此函数已废弃,使用 get_image_data() 配合 ggez 渲染
        return Err(io::Error::new(
            io::ErrorKind::Unsupported,
            "get_texture() is deprecated, use get_image_data() with ggez instead"
        ));
        
        #[allow(unreachable_code)]
        {
        let key = TextureKey {
            library: library.to_string(),
            index,
        };
        
        // 检查缓存
        if let Some(handle) = self.textures.get(&key) {
            let lib = self.libraries.get_mut(library)
                .ok_or_else(|| io::Error::new(
                    io::ErrorKind::NotFound,
                    format!("Library '{}' not loaded", library)
                ))?;
            let info = lib.get_image_info(index)?;
            return Ok((info, handle.clone()));
        }
        
        // 加载纹理数据
        let lib = self.libraries.get_mut(library)
            .ok_or_else(|| io::Error::new(
                io::ErrorKind::NotFound,
                format!("Library '{}' not loaded", library)
            ))?;
        
        let (info, rgba_data) = lib.load_rgba_data(index)?;
        
        // 上传到 GPU (unreachable code - 保留用于参考)
        let texture_name = format!("{}_{}", library, index);
        let handle = _dx_manager;  // Dummy - this code is unreachable
        let handle = Arc::new(handle.clone());
        
        self.textures.insert(key, handle.clone());
        
        Ok((info, handle))
        } // unreachable_code block
    }
    
    /// 获取图像信息(不加载纹理)
    /// 
    /// C# equivalent: MLibrary.GetImageInfo()
    pub fn get_image_info(&mut self, library: &str, index: usize) -> io::Result<ImageInfo> {
        let lib = self.libraries.get_mut(library)
            .ok_or_else(|| io::Error::new(
                io::ErrorKind::NotFound,
                format!("Library '{}' not loaded", library)
            ))?;
        lib.get_image_info(index)
    }
    
    /// 清除所有纹理缓存
    /// 
    /// C# equivalent: DXManager.Clean()
    pub fn clear_cache(&mut self) {
        self.textures.clear();
    }
    
}

// ===== 辅助函数 =====

fn read_i32<R: Read>(reader: &mut R) -> io::Result<i32> {
    let mut buf = [0u8; 4];
    reader.read_exact(&mut buf)?;
    Ok(i32::from_le_bytes(buf))
}

fn read_i16<R: Read>(reader: &mut R) -> io::Result<i16> {
    let mut buf = [0u8; 2];
    reader.read_exact(&mut buf)?;
    Ok(i16::from_le_bytes(buf))
}

fn read_u8<R: Read>(reader: &mut R) -> io::Result<u8> {
    let mut buf = [0u8; 1];
    reader.read_exact(&mut buf)?;
    Ok(buf[0])
}

// ===== MLibrary draw functions (deprecated) =====
/*  // All draw functions commented out - depend on dx_manager

/// Drawing and helper methods for MLibrary
impl MLibrary {
    /// Draw image without blending
    /// 
    /// C# equivalent:
    /// ```csharp
    /// public void Draw(int index, Point point, Color colour, bool offSet, float opacity) {
    ///     if (!CheckImage(index)) return;
    ///     MImage mi = _images[index];
    ///     if (offSet) point.Offset(mi.X, mi.Y);
    ///     if (point.X >= Settings.ScreenWidth || point.Y >= Settings.ScreenHeight || 
    ///         point.X + mi.Width < 0 || point.Y + mi.Height < 0) return;
    ///     DXManager.DrawOpaque(mi.Image, new Rectangle(0, 0, mi.Width, mi.Height), 
    ///                          new Vector3((float)point.X, (float)point.Y, 0.0F), colour, opacity);
    /// }
    /// ```
    pub fn draw(
        &mut self,
        dx_manager: &mut super::dx_manager::DXManager,
        index: i32,
        point: (i32, i32),
        color: [f32; 4],
        use_offset: bool,
        opacity: f32,
        screen_width: i32,
        screen_height: i32,
    ) -> io::Result<()> {
        if !self.check_image(index) {
            return Ok(()); // C# 静默返回
        }
        
        // Step 1: 获取图像信息
        let info = self.get_image_info(index as usize)?;
        
        // Step 2: 应用偏移
        let (mut x, mut y) = point;
        if use_offset {
            x += info.x as i32;
            y += info.y as i32;
        }
        
        // Step 3: 屏幕裁剪检查 (照搬 C# 逻辑)
        if x >= screen_width || y >= screen_height || 
           (x + info.width as i32) < 0 || (y + info.height as i32) < 0 {
            return Ok(());
        }
        
        // Step 4: 加载/缓存纹理
        let texture = self.get_or_load_texture(dx_manager, index as usize)?;
        
        // Step 5: 调用 DXManager 渲染
        // 应用 opacity 到 color alpha 通道
        let mut render_color = color;
        render_color[3] *= opacity;
        
        dx_manager.draw_sprite(
            &texture,
            (x, y),
            (info.width as u32, info.height as u32),
            render_color,
        ).map_err(|e| io::Error::new(io::ErrorKind::Other, e.to_string()))?;
        
        // Step 6: 更新缓存时间戳 (对应 C# 的 mi.CleanTime)
        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_millis() as i64;
        self.cache_timestamps.insert(index as usize, now);
        
        Ok(())
    }
    
    /// Draw image with blending
    /// 
    /// C# equivalent:
    /// ```csharp
    /// public void DrawBlend(int index, Point point, Color colour, bool offSet = false, float rate = 1) {
    ///     if (!CheckImage(index)) return;
    ///     MImage mi = _images[index];
    ///     if (offSet) point.Offset(mi.X, mi.Y);
    ///     if (point.X >= Settings.ScreenWidth || point.Y >= Settings.ScreenHeight || 
    ///         point.X + mi.Width < 0 || point.Y + mi.Height < 0) return;
    ///     bool oldBlend = DXManager.Blending;
    ///     DXManager.SetBlend(true, rate);
    ///     DXManager.Draw(mi.Image, new Rectangle(0, 0, mi.Width, mi.Height), 
    ///                    new Vector3((float)point.X, (float)point.Y, 0.0F), colour);
    ///     DXManager.SetBlend(oldBlend);
    /// }
    /// ```
    pub fn draw_blend(
        &mut self,
        dx_manager: &mut super::dx_manager::DXManager,
        index: i32,
        point: (i32, i32),
        color: [f32; 4],
        use_offset: bool,
        rate: f32,
        screen_width: i32,
        screen_height: i32,
    ) -> io::Result<()> {
        if !self.check_image(index) {
            return Ok(()); // C# 静默返回
        }
        
        // Step 1: 获取图像信息
        let info = self.get_image_info(index as usize)?;
        
        // Step 2: 应用偏移
        let (mut x, mut y) = point;
        if use_offset {
            x += info.x as i32;
            y += info.y as i32;
        }
        
        // Step 3: 屏幕裁剪检查
        if x >= screen_width || y >= screen_height || 
           (x + info.width as i32) < 0 || (y + info.height as i32) < 0 {
            return Ok(());
        }
        
        // Step 4: 加载/缓存纹理
        let texture = self.get_or_load_texture(dx_manager, index as usize)?;
        
        // Step 5: 应用混合率到 color alpha 通道
        let mut render_color = color;
        render_color[3] *= rate;
        
        // Step 6: 使用混合模式渲染
        dx_manager.draw_sprite_blend(
            &texture,
            (x, y),
            (info.width as u32, info.height as u32),
            render_color,
        ).map_err(|e| io::Error::new(io::ErrorKind::Other, e.to_string()))?;
        
        // Step 7: 更新缓存时间戳
        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_millis() as i64;
        self.cache_timestamps.insert(index as usize, now);
        
        Ok(())
    }
    
    /// 批处理模式绘制(不立即提交GPU)
    /// 
    /// 用于粒子系统批量渲染优化
    pub fn draw_batched(
        &mut self,
        dx_manager: &super::dx_manager::DXManager,
        index: i32,
        point: (i32, i32),
        color: [f32; 4],
        use_offset: bool,
        opacity: f32,
        screen_width: i32,
        screen_height: i32,
    ) -> io::Result<()> {
        if !self.check_image(index) {
            return Ok(());
        }
        
        let info = self.get_image_info(index as usize)?;
        let (mut x, mut y) = point;
        if use_offset {
            x += info.x as i32;
            y += info.y as i32;
        }
        
        if x >= screen_width || y >= screen_height || 
           (x + info.width as i32) < 0 || (y + info.height as i32) < 0 {
            return Ok(());
        }
        
        let texture = self.get_or_load_texture_readonly(dx_manager, index as usize)?;
        
        let mut render_color = color;
        render_color[3] *= opacity;
        
        dx_manager.draw_sprite_batched(
            &texture,
            (x, y),
            (info.width as u32, info.height as u32),
            render_color,
        );
        
        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_millis() as i64;
        self.cache_timestamps.insert(index as usize, now);
        
        Ok(())
    }
    
    /// 批处理模式混合绘制(不立即提交GPU)
    pub fn draw_blend_batched(
        &mut self,
        dx_manager: &super::dx_manager::DXManager,
        index: i32,
        point: (i32, i32),
        color: [f32; 4],
        use_offset: bool,
        rate: f32,
        screen_width: i32,
        screen_height: i32,
    ) -> io::Result<()> {
        if !self.check_image(index) {
            return Ok(());
        }
        
        let info = self.get_image_info(index as usize)?;
        let (mut x, mut y) = point;
        if use_offset {
            x += info.x as i32;
            y += info.y as i32;
        }
        
        if x >= screen_width || y >= screen_height || 
           (x + info.width as i32) < 0 || (y + info.height as i32) < 0 {
            return Ok(());
        }
        
        let texture = self.get_or_load_texture_readonly(dx_manager, index as usize)?;
        
        let mut render_color = color;
        render_color[3] *= rate;
        
        dx_manager.draw_sprite_batched(
            &texture,
            (x, y),
            (info.width as u32, info.height as u32),
            render_color,
        );
        
        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_millis() as i64;
        self.cache_timestamps.insert(index as usize, now);
        
        Ok(())
    }
    
    /// GPU实例化渲染 (普通模式)
    pub fn draw_instanced(
        &mut self,
        dx_manager: &super::dx_manager::DXManager,
        index: i32,
        point: (i32, i32),
        color: [f32; 4],
        use_offset: bool,
        opacity: f32,
        screen_width: i32,
        screen_height: i32,
    ) -> io::Result<()> {
        if !self.check_image(index) {
            return Ok(());
        }
        
        let info = self.get_image_info(index as usize)?;
        let (mut x, mut y) = point;
        if use_offset {
            x += info.x as i32;
            y += info.y as i32;
        }
        
        if x >= screen_width || y >= screen_height || 
           (x + info.width as i32) < 0 || (y + info.height as i32) < 0 {
            return Ok(());
        }
        
        let texture = self.get_or_load_texture_readonly(dx_manager, index as usize)?;
        
        let mut render_color = color;
        render_color[3] *= opacity;
        
        dx_manager.draw_sprite_instanced(
            &texture,
            (x, y),
            (info.width as u32, info.height as u32),
            render_color,
        );
        
        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_millis() as i64;
        self.cache_timestamps.insert(index as usize, now);
        
        Ok(())
    }
    
    /// GPU实例化渲染 (混合模式)
    pub fn draw_blend_instanced(
        &mut self,
        dx_manager: &super::dx_manager::DXManager,
        index: i32,
        point: (i32, i32),
        color: [f32; 4],
        use_offset: bool,
        rate: f32,
        screen_width: i32,
        screen_height: i32,
    ) -> io::Result<()> {
        if !self.check_image(index) {
            return Ok(());
        }
        
        let info = self.get_image_info(index as usize)?;
        let (mut x, mut y) = point;
        if use_offset {
            x += info.x as i32;
            y += info.y as i32;
        }
        
        if x >= screen_width || y >= screen_height || 
           (x + info.width as i32) < 0 || (y + info.height as i32) < 0 {
            return Ok(());
        }
        
        let texture = self.get_or_load_texture_readonly(dx_manager, index as usize)?;
        
        let mut render_color = color;
        render_color[3] *= rate;
        
        dx_manager.draw_sprite_instanced(
            &texture,
            (x, y),
            (info.width as u32, info.height as u32),
            render_color,
        );
        
        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_millis() as i64;
        self.cache_timestamps.insert(index as usize, now);
        
        Ok(())
    }
    
    /// 获取已缓存的纹理(用于批处理)
    /// 
    /// 批处理模式要求纹理已预加载
    /// 如果纹理未缓存,会尝试即时加载
    fn get_or_load_texture_readonly(
        &mut self,
        dx_manager: &super::dx_manager::DXManager,
        index: usize,
    ) -> io::Result<Arc<TextureHandle>> {
        // 如果已缓存则直接返回
        if let Some(texture) = self.texture_cache.get(&index) {
            return Ok(Arc::clone(texture));
        }
        
        // 未缓存: 加载纹理
        // load_rgba_data不需要&mut self,只读取数据
        let (info, rgba_data) = self.load_rgba_data(index)?;
        
        // DXManager的load_texture使用内部可变性,&self即可
        let texture_name = format!("{}_{}", self.path.display(), index);
        let texture = dx_manager.load_texture(
            texture_name,
            info.width as u32,
            info.height as u32,
            &rgba_data,
        );
        
        // 缓存纹理
        self.texture_cache.insert(index, texture.clone());
        
        Ok(texture)
    }
    
    /// 获取或加载纹理 (内部方法)
    /// 
    /// 对应 C# 的纹理缓存逻辑
    fn get_or_load_texture(
        &mut self,
        dx_manager: &mut super::dx_manager::DXManager,
        index: usize,
    ) -> io::Result<Arc<TextureHandle>> {
        // 检查缓存
        if let Some(texture) = self.texture_cache.get(&index) {
            return Ok(texture.clone());
        }
        
        // 加载纹理数据
        let (info, rgba_data) = self.load_rgba_data(index)?;
        
        // 上传到 GPU
        let texture_name = format!("{}_{}", self.path.display(), index);
        let texture = dx_manager.load_texture(
            texture_name,
            info.width as u32,
            info.height as u32,
            &rgba_data,
        );
        
        // 缓存
        self.texture_cache.insert(index, texture.clone());
        
        Ok(texture)
    }
    
    /// 清理过期纹理缓存
    /// 
    /// C# equivalent: 定期清理 MImage.CleanTime
    pub fn clean_cache(&mut self, max_age_ms: i64) {
        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_millis() as i64;
        
        let mut to_remove = Vec::new();
        for (index, &timestamp) in &self.cache_timestamps {
            if now - timestamp > max_age_ms {
                to_remove.push(*index);
            }
        }
        
        for index in to_remove {
            self.texture_cache.remove(&index);
            self.cache_timestamps.remove(&index);
        }
    }
    
    /// Check if image index is valid
    /// 
    /// C# equivalent: MLibrary.CheckImage(int index)
    pub fn check_image(&self, index: i32) -> bool {
        index >= 0 && (index as usize) < self.indices.len()
    }
    
    /// Get image bounds for drawing
    /// 
    /// Used for screen clipping optimization.
    /// Returns None if image is invalid or info cannot be retrieved.
    /// 
    /// C# equivalent: Logic in MLibrary.Draw() for bounds checking
    pub fn get_image_bounds_mut(
        &mut self,
        index: i32,
        point: (i32, i32),
        use_offset: bool,
    ) -> Option<(i32, i32, i32, i32)> {
        if !self.check_image(index) {
            return None;
        }
        
        let info = self.get_image_info(index as usize).ok()?;
        
        let (mut x, mut y) = point;
        if use_offset {
            x += info.x as i32;
            y += info.y as i32;
        }
        
        let width = info.width as i32;
        let height = info.height as i32;
        
        Some((x, y, width, height))
    }
    
    */  // End of deprecated draw functions block

#[cfg(test)]
mod tests {
    // Tests require actual .lib files
    // TODO: Add integration tests with sample data
}
