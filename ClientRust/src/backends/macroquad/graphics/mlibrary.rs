// MLibrary - MIR2 图像库加载器 (Macroquad 版本)
// 对应: Client/MirGraphics/MLibrary.cs
//
// 负责解析和加载 .lib 文件格式（MIR2 专有的图像库格式）
//
// 注意: 此模块依赖 objects::frames 和 macroquad
// 基于 ggez 版本移植，保持相同的逻辑但使用 macroquad 纹理

// use crate::objects::frames::{Frame, FrameSet};  // TODO: 修复编译器找不到 frames 模块的问题
use byteorder::LittleEndian;
use byteorder::ReadBytesExt;
use flate2::read::GzDecoder;

use macroquad::prelude::*;

use std::collections::HashMap;
use std::fs::File;
use std::io::{self, BufReader, Read, Seek, SeekFrom};
use std::path::{Path, PathBuf};
use std::time::Instant;

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
    pub offset: i32, // 文件中的偏移量
}

/// 图像元数据(不包含纹理数据)
#[derive(Debug, Clone)]
pub struct ImageInfo {
    pub width: i16,
    pub height: i16,
    pub x: i16, // 偏移量X
    pub y: i16, // 偏移量Y
    pub shadow_x: i16,
    pub shadow_y: i16,
    pub shadow: u8,
    pub length: i32,    // 压缩数据长度
    pub has_mask: bool, // 是否有第二层
    pub mask_width: i16,
    pub mask_height: i16,
    pub mask_x: i16,
    pub mask_y: i16,
    pub mask_length: i32,
    pub texture_valid: bool, // 纹理是否有效
    pub image: Option<Texture2D>, // 解压后的纹理数据 (RGBA格式)
    pub mask_image: Option<Texture2D>, // 解压后的遮罩纹理数据 (RGBA格式)
    pub last_access_time: Option<Instant>, // 最后访问时间 (用于缓存清理)
    bgra_data: Option<Vec<u8>>, // 原始解压数据 (RGBA格式)
}

impl ImageInfo {
    /// 从读取器中解析 ImageInfo (17字节)
    ///
    /// 注意: 这里不读取纹理数据,只读取元数据
    ///
    /// # 参数
    /// - `r`: 可读的字节流
    ///
    /// # 返回
    /// - `Ok(ImageInfo)`: 成功解析
    /// - `Err(io::Error)`: 读取失败
    pub fn from_reader<R: std::io::Read + Seek>(r: &mut R) -> Result<Self, std::io::Error> {
        let width = r.read_i16::<LittleEndian>()?;
        let height = r.read_i16::<LittleEndian>()?;
        let x = r.read_i16::<LittleEndian>()?;
        let y = r.read_i16::<LittleEndian>()?;
        let shadow_x = r.read_i16::<LittleEndian>()?;
        let shadow_y = r.read_i16::<LittleEndian>()?;
        let shadow = r.read_u8()?;
        let length = r.read_i32::<LittleEndian>()?;
        let has_mask = (shadow >> 7) == 1;
        let mut mask_width = 0;
        let mut mask_height = 0;
        let mut mask_x = 0;
        let mut mask_y = 0;
        let mut mask_length = 0;
        if has_mask {
            r.seek(SeekFrom::Current(length as i64))?;
            mask_width = r.read_i16::<LittleEndian>()?;
            mask_height = r.read_i16::<LittleEndian>()?;
            mask_x = r.read_i16::<LittleEndian>()?;
            mask_y = r.read_i16::<LittleEndian>()?;
            mask_length = r.read_i32::<LittleEndian>()?;
        }
        Ok(Self {
            width,
            height,
            x,
            y,
            shadow_x,
            shadow_y,
            shadow,
            length,
            has_mask,
            mask_width,
            mask_height,
            mask_x,
            mask_y,
            mask_length,
            texture_valid: false,
            image: None,
            mask_image: None,
            last_access_time: None,
            bgra_data: None,
        })
    }

    /// 创建纹理数据 - 从reader中读取并解压图像数据
    ///
    /// 对应 C# MImage.CreateTexture
    /// ```csharp
    /// public unsafe void CreateTexture(BinaryReader reader)
    /// {
    ///     int w = Width;
    ///     int h = Height;
    ///     Image = new Texture(DXManager.Device, w, h, 1, Usage.None, Format.A8R8G8B8, Pool.Managed);
    ///     DataRectangle stream = Image.LockRectangle(0, LockFlags.Discard);
    ///     Data = (byte*)stream.Data.DataPointer;
    ///     DecompressImage(reader.ReadBytes(Length), stream.Data);
    ///     stream.Data.Dispose();
    ///     Image.UnlockRectangle(0);
    ///     if (HasMask) {
    ///         reader.ReadBytes(12);
    ///         w = Width;
    ///         h = Height;
    ///         MaskImage = new Texture(DXManager.Device, w, h, 1, Usage.None, Format.A8R8G8B8, Pool.Managed);
    ///         stream = MaskImage.LockRectangle(0, LockFlags.Discard);
    ///         DecompressImage(reader.ReadBytes(Length), stream.Data);
    ///         stream.Data.Dispose();
    ///         MaskImage.UnlockRectangle(0);
    ///     }
    ///     DXManager.TextureList.Add(this);
    ///     TextureValid = true;
    ///     CleanTime = CMain.Time + Settings.CleanDelay;
    /// }
    /// ```
    ///
    /// # 参数
    /// - `reader`: 二进制读取器，当前位置应该在压缩数据的开始处
    ///
    /// # 返回
    /// - `Ok((main_image_data, mask_image_data))`: 主图像数据和可选的遮罩图像数据（RGBA格式）
    /// - `Err`: 读取或解压失败
    ///
    /// # 注意
    /// - 返回的数据是RGBA格式（4字节/像素）
    /// - 黑色像素(0,0,0)会被转换为透明(0,0,0,0)
    /// - 如果has_mask为true，会读取第二层图像数据
    pub fn create_texture<R: std::io::Read + Seek>(
        &mut self,
        reader: &mut R,
    ) -> Result<(), std::io::Error> {
        // 读取主图像的压缩数据
        let mut compressed_data = vec![0u8; self.length as usize];
        reader.read_exact(&mut compressed_data)?;

        // 解压主图像
        let mut main_image = Self::decompress_image(&compressed_data, self.width, self.height)?;

        // 🔧 BGRA -> RGBA 转换
        Self::bgra_to_rgba(&mut main_image);

        self.bgra_data = Some(main_image.clone()); // 保存原始数据副本

        // 🔧 使用 macroquad Texture2D 创建纹理（RGBA 格式数据）
        self.image = Some(Texture2D::from_rgba8(
            self.width as u16,
            self.height as u16,
            &main_image,
        ));

        // 处理遮罩层
        if self.has_mask {
            // 跳过12字节的遮罩头信息（C#: reader.ReadBytes(12)）
            // 这12字节包含: MaskWidth(2) + MaskHeight(2) + MaskX(2) + MaskY(2) + MaskLength(4)
            // 但这些信息在ImageInfo构造时已经读取，所以这里只是跳过
            // let mut skip_buffer = [0u8; 12];
            // reader.read_exact(&mut skip_buffer)?;
            reader.seek(SeekFrom::Current(12))?;
            // 读取遮罩层的压缩数据
            let mut mask_compressed = vec![0u8; self.mask_length as usize];
            reader.read_exact(&mut mask_compressed)?;

            // 解压遮罩层（使用主图像的宽高，因为C#代码中遮罩使用Width/Height）
            let mut mask_data = Self::decompress_image(&mask_compressed, self.width, self.height)?;

            // 🔧 BGRA -> RGBA 转换
            Self::bgra_to_rgba(&mut mask_data);

            // 🔧 使用 macroquad Texture2D 创建遮罩纹理（RGBA 格式数据）
            self.mask_image = Some(Texture2D::from_rgba8(
                self.width as u16,
                self.height as u16,
                &mask_data,
            ));
        }
        self.last_access_time = Some(Instant::now());
        self.texture_valid = true;
        Ok(())
    }

    pub fn dispose_texture(&mut self) {
        self.image.take();
        self.mask_image.take();
        self.last_access_time.take();
        self.bgra_data.take();
        self.texture_valid = false;
    }

    /// 获取 BGRA 原始数据的引用 (如果已加载)
    ///
    /// 用于 Bevy 等其他渲染引擎获取图像数据
    pub fn get_bgra_data(&self) -> Option<&Vec<u8>> {
        self.bgra_data.as_ref()
    }

    /// 解压图像数据并转换为RGBA格式
    ///
    /// 对应 C# MImage.DecompressImage
    /// ```csharp
    /// private static void DecompressImage(byte[] data, Stream destination)
    /// {
    ///     using (var stream = new GZipStream(new MemoryStream(data), CompressionMode.Decompress))
    ///     {
    ///         stream.CopyTo(destination);
    ///     }
    /// }
    /// ```
    ///
    /// # 参数
    /// - `compressed`: 压缩的图像数据（GZip格式）
    /// - `width`: 图像宽度
    /// - `height`: 图像高度
    ///
    /// # 返回
    /// - `Ok(Vec<u8>)`:BGRA格式的图像数据
    /// - `Err`: 解压失败
    fn decompress_image(
        compressed: &[u8],
        width: i16,
        height: i16,
    ) -> Result<Vec<u8>, std::io::Error> {
        // 使用GZip解压
        let mut decoder = GzDecoder::new(compressed);
        let mut decompressed = Vec::new();
        decoder.read_to_end(&mut decompressed)?;

        // 验证解压后的数据大小
        let expected_size = (width as usize) * (height as usize) * 4;
        if decompressed.len() != expected_size {
            if decompressed.len() > expected_size {
                // 数据过长，截断
                tracing::debug!(
                    "⚠️ 图像数据过长 ({} > {}), 截断",
                    decompressed.len(),
                    expected_size
                );
                decompressed.truncate(expected_size);
            } else {
                // 数据过短，填充透明像素
                tracing::debug!(
                    "⚠️ 图像数据过短 ({} < {}), 填充",
                    decompressed.len(),
                    expected_size
                );
                decompressed.resize(expected_size, 0);
            }
        }
        Ok(decompressed)
    }

    /// BGRA 转 RGBA
    /// 
    /// lib 文件存储的是 BGRA 格式（DirectX 格式），需要转换为 RGBA：
    /// - 交换 R 和 B 通道（BGRA -> RGBA）
    /// 
    /// 注意：不需要黑色背景透明化，渲染层会使用 ADD 混合模式处理
    fn bgra_to_rgba(data: &mut [u8]) {
        for chunk in data.chunks_exact_mut(4) {
            // BGRA: [B, G, R, A] -> RGBA: [R, G, B, A]
            chunk.swap(0, 2); // 交换 B 和 R 通道
        }
    }

    // fn apply_auto_alpha(rgba: &mut [u8]) {
    //     for chunk in rgba.chunks_exact_mut(4) {
    //         let alpha = chunk[3];
    //         if alpha == 0 {
    //             chunk.copy_from_slice(&[0, 0, 0, 0]);
    //             continue;
    //         }

    //         let max_rgb = chunk[2].max(chunk[1]).max(chunk[0]);
    //         if max_rgb == 0 {
    //             chunk.copy_from_slice(&[0, 0, 0, 0]);
    //             continue;
    //         }

    //         let scale = max_rgb as u16;
    //         chunk[0] = ((u16::from(chunk[0]) * 255 + scale / 2) / scale).min(255) as u8;
    //         chunk[1] = ((u16::from(chunk[1]) * 255 + scale / 2) / scale).min(255) as u8;
    //         chunk[2] = ((u16::from(chunk[2]) * 255 + scale / 2) / scale).min(255) as u8;
    //         chunk[3] = max_rgb;
    //     }
    // }

    /// 检查指定像素是否可见（非透明）
    ///
    /// 对应 C# MImage.VisiblePixel
    /// ```csharp
    /// public unsafe bool VisiblePixel(Point p)
    /// {
    ///     if (p.X < 0 || p.Y < 0 || p.X >= Width || p.Y >= Height)
    ///         return false;
    ///     int w = Width;
    ///     bool result = false;
    ///     if (Data != null)
    ///     {
    ///         int x = p.X;
    ///         int y = p.Y;
    ///         int index = (y * (w << 2)) + (x << 2) + 3;
    ///         byte col = Data[index];
    ///         if (col == 0) return false;
    ///         else return true;
    ///     }
    ///     return result;
    /// }
    /// ```
    ///
    /// # 参数
    /// - `x`: 像素X坐标
    /// - `y`: 像素Y坐标
    /// - `rgba_data`: RGBA格式的图像数据（可选）
    ///
    /// # 返回
    /// - `true`: 像素可见（alpha > 0）
    /// - `false`: 像素透明或坐标越界
    ///
    /// # 注意
    /// - 坐标越界返回 false
    /// - 检查 alpha 通道（第4字节）是否为0
    pub fn visible_pixel(&self, x: i32, y: i32) -> bool {
        if let Some(ref rgba_data) = self.bgra_data {
            // 边界检查
            if x < 0 || y < 0 || x >= self.width as i32 || y >= self.height as i32 {
                return false;
            }
            let w = self.width as usize;
            // 使用我们保存的数据副本
            // 计算索引: (y * width * 4) + (x * 4) + 3
            let index = ((y as usize) * (w << 2)) + ((x as usize) << 2) + 3;
            if index < rgba_data.len() {
                return rgba_data[index] != 0;
            }
        }
        false
    }

    /// 获取图像的实际显示尺寸（去除透明边缘）
    ///
    /// 对应 C# MImage.GetTrueSize
    /// ```csharp
    /// public Size GetTrueSize()
    /// {
    ///     if (TrueSize != Size.Empty) return TrueSize;
    ///     int l = 0, t = 0, r = Width, b = Height;
    ///     // ... 四个方向扫描找到非透明边界 ...
    ///     TrueSize = Rectangle.FromLTRB(l, t, r, b).Size;
    ///     return TrueSize;
    /// }
    /// ```
    ///
    /// # 参数
    /// - `rgba_data`: RGBA格式的图像数据
    ///
    /// # 返回
    /// - `(width, height)`: 实际显示尺寸（像素）
    ///
    /// # 算法
    /// 从四个方向扫描找到第一个非透明像素：
    /// 1. 从左到右 -> 找到左边界 (l)
    /// 2. 从上到下 -> 找到上边界 (t)
    /// 3. 从右到左 -> 找到右边界 (r)
    /// 4. 从下到上 -> 找到下边界 (b)
    ///
    /// # 性能
    /// - 最坏情况: O(width * height)
    /// - 会缓存结果，避免重复计算
    pub fn get_true_size(&self) -> (i16, i16) {
        let mut l = 0i32;
        let mut t = 0i32;
        let mut r = self.width as i32;
        let mut b = self.height as i32;

        // 1. 从左到右扫描，找到第一列包含可见像素
        let mut visible = false;
        for x in 0..r {
            for y in 0..b {
                if !self.visible_pixel(x, y) {
                    continue;
                }
                visible = true;
                break;
            }
            if !visible {
                continue;
            }
            l = x;
            break;
        }

        // 2. 从上到下扫描，找到第一行包含可见像素
        visible = false;
        for y in 0..b {
            for x in l..r {
                if !self.visible_pixel(x, y) {
                    continue;
                }
                visible = true;
                break;
            }
            if !visible {
                continue;
            }
            t = y;
            break;
        }

        // 3. 从右到左扫描，找到最后一列包含可见像素
        visible = false;
        for x in (l..r).rev() {
            for y in 0..b {
                if !self.visible_pixel(x, y) {
                    continue;
                }
                visible = true;
                break;
            }
            if !visible {
                continue;
            }
            r = x + 1;
            break;
        }

        // 4. 从下到上扫描，找到最后一行包含可见像素
        visible = false;
        for y in (t..b).rev() {
            for x in l..r {
                if !self.visible_pixel(x, y) {
                    continue;
                }
                visible = true;
                break;
            }
            if !visible {
                continue;
            }
            b = y + 1;
            break;
        }

        // 返回宽度和高度
        let width = (r - l) as i16;
        let height = (b - t) as i16;

        (width, height)
    }
}

/// MIR2图像库
#[derive(Debug)]
pub struct MLibrary {
    path: PathBuf,
    header: LibraryHeader,
    indices: Vec<ImageIndex>,
    // frames: FrameSet,  // TODO: 暂时注释掉frames功能（地图渲染不需要）
    cached_info: HashMap<usize, ImageInfo>,
    reader: BufReader<File>,
}

impl MLibrary {
    /// 打开.lib文件
    pub fn open<P: AsRef<Path>>(path: P) -> io::Result<Self> {
        let mut path_buf = path.as_ref().to_path_buf();
        // 尝试.Lib扩展名
        if !path_buf.exists() {
            path_buf.set_extension("Lib");
        }
        let mut reader = BufReader::new(File::open(&path_buf)?);
        let version = reader.read_i32::<LittleEndian>()?;
        if version < 2 {
            return Err(io::Error::new(
                io::ErrorKind::InvalidData,
                format!("Unsupported .lib version: {}", version),
            ));
        }
        let count = reader.read_i32::<LittleEndian>()?;
        let mut frame_seek = 0;
        if version >= 3 {
            frame_seek = reader.read_i32::<LittleEndian>()?;
        }
        let header = LibraryHeader {
            version,
            count,
            frame_seek,
        };
        let mut indices = Vec::with_capacity(count as usize);
        for _ in 0..count {
            let offset = reader.read_i32::<LittleEndian>()?;
            indices.push(ImageIndex { offset });
        }
        // TODO: 暂时注释掉 frames 加载（地图渲染不需要）
        /*
        let mut frames = FrameSet::new();
        if version >= 3 {
            reader.seek(SeekFrom::Start(frame_seek as u64))?;
            let frame_count = reader.read_i32::<LittleEndian>()?;

            for _ in 0..frame_count {
                let action_byte = reader.read_u8()?;
                let action = MirAction::try_from(action_byte).map_err(|_| {
                    io::Error::new(
                        io::ErrorKind::InvalidData,
                        format!("Invalid MirAction value: {}", action_byte),
                    )
                })?;
                let frame = Frame::from_reader(&mut reader)?;
                frames.insert(action, frame);
            }
        }
        */

        // 使用 HashMap 缓存图像信息，稀疏访问模式更高效
        // 预分配容量避免动态扩容
        let cached_info = HashMap::with_capacity(count as usize);

        Ok(Self {
            path: path_buf,
            header,
            indices,
            // frames,  // TODO: 暂时注释掉
            cached_info,
            reader,
        })
    }

    /// 获取图像数量
    pub fn count(&self) -> usize {
        self.indices.len()
    }

    /// 获取库文件路径
    pub fn path(&self) -> &Path {
        &self.path
    }

    // TODO: 暂时注释掉 frames 相关方法
    /*
    pub fn frames(&self) -> &FrameSet {
        &self.frames
    }
    */

    /// 读取图像信息(不解压纹理数据)
    pub fn get_image_info(&mut self, index: usize) -> io::Result<ImageInfo> {
        if index >= self.indices.len() {
            return Err(io::Error::new(
                io::ErrorKind::InvalidInput,
                format!(
                    "Image index {} out of range (max: {})",
                    index,
                    self.indices.len()
                ),
            ));
        }

        // 检查缓存 (如果已缓存则直接返回)
        if let Some(cached) = self.cached_info.get(&index) {
            return Ok(cached.clone());
        }

        // 读取图像信息
        let offset = self.indices[index].offset as u64;
        self.reader.seek(SeekFrom::Start(offset))?;
        let info = ImageInfo::from_reader(&mut self.reader)?;

        // 缓存结果
        self.cached_info.insert(index, info.clone());
        Ok(info)
    }

    /// 获取图像并解压 BGRA 数据 (用于非 ggez 渲染引擎,如 Bevy)
    ///
    /// 返回 (ImageInfo, BGRA数据)
    pub fn get_image_with_data(&mut self, index: usize) -> io::Result<(ImageInfo, Vec<u8>)> {
        if index >= self.indices.len() {
            return Err(io::Error::new(
                io::ErrorKind::InvalidInput,
                format!("Image index {} out of range", index),
            ));
        }

        // 定位到图像数据
        let offset = self.indices[index].offset as u64;
        self.reader.seek(SeekFrom::Start(offset))?;

        // 读取图像信息
        let info = ImageInfo::from_reader(&mut self.reader)?;

        // 读取并解压主图像数据
        let mut compressed_data = vec![0u8; info.length as usize];
        self.reader.read_exact(&mut compressed_data)?;
        let bgra_data = ImageInfo::decompress_image(&compressed_data, info.width, info.height)?;

        Ok((info, bgra_data))
    }

    /// 获取或创建缓存的 ggez 纹理
    ///
    /// 对应 C# 实现:
    /// ```csharp
    /// // MLibrary.cs line 898-927
    /// public unsafe void CreateTexture(BinaryReader reader) {
    ///     // ... create texture once ...
    ///     DXManager.TextureList.Add(this); // Cache for reuse
    /// }
    /// ```
    ///
    /// # 参数
    /// - `ctx`: ggez Context
    /// - `index`: 图像索引
    ///
    /// # 返回
    /// - `Ok(&ImageInfo)`: 缓存的纹理引用（零拷贝）
    /// - `Err`: 加载失败
    ///
    /// # 性能优化
    /// - ✅ 返回引用而非克隆，避免大块内存拷贝
    /// - ✅ 自动缓存纹理，重复调用零开销
    pub fn get_or_create_texture(
        &mut self,
        
        index: usize,
    ) -> io::Result<&ImageInfo> {
        // 检查索引范围
        if index >= self.indices.len() {
            return Err(io::Error::new(
                io::ErrorKind::InvalidInput,
                format!("图像索引 {} 超出范围 (max: {})", index, self.indices.len()),
            ));
        }

        // 使用 entry API 统一处理
        use std::collections::hash_map::Entry;
        let offset = self.indices[index].offset as u64;

        // 先处理 Entry，确保数据在 HashMap 中
        match self.cached_info.entry(index) {
            Entry::Occupied(mut e) => {
                let cached = e.get_mut();
                // 检查是否已有纹理
                if !cached.texture_valid {
                    // 已有 info 但没有纹理，创建纹理
                    self.reader.seek(SeekFrom::Start(offset + 17))?;
                    cached.create_texture( &mut self.reader)?;
                }
                // 更新访问时间
                cached.last_access_time = Some(Instant::now());
            }
            Entry::Vacant(e) => {
                // 缓存中没有，读取图像元数据
                self.reader.seek(SeekFrom::Start(offset))?;
                let mut info = ImageInfo::from_reader(&mut self.reader)?;

                // 验证图像有效性
                if info.width == 0 || info.height == 0 {
                    return Err(io::Error::new(
                        io::ErrorKind::InvalidData,
                        format!(
                            "图像 {} 无效: width={}, height={}",
                            index, info.width, info.height
                        ),
                    ));
                }

                // 创建纹理
                self.reader.seek(SeekFrom::Start(offset + 17))?;
                info.create_texture( &mut self.reader)?;

                // 插入
                e.insert(info);
            }
        }

        // 现在返回引用（Entry 已经释放）
        Ok(self.cached_info.get_mut(&index).unwrap())
    }

    /// 清理长时间未使用的纹理缓存
    ///
    /// 对应 C# 实现:
    /// ```csharp
    /// // DXManager.cs - CleanUp()
    /// for (int i = TextureList.Count - 1; i >= 0; i--) {
    ///     if (CMain.Time >= TextureList[i].CleanTime)
    ///         TextureList[i].DisposeTexture();
    /// }
    /// ```
    ///
    /// # 参数
    /// - `max_age`: 最大未使用时间 (超过此时间的纹理将被清理)
    pub fn cleanup_old_textures(&mut self, max_age: std::time::Duration) {
        use std::time::Instant;
        let now = Instant::now();
        let mut removed = 0;
        self.cached_info.iter_mut().for_each(|(_index, image)| {
            if let Some(access_time) = image.last_access_time {
                let age = now.duration_since(access_time);
                if age > max_age {
                    image.dispose_texture();
                    removed += 1;
                }
            }
        });

        if removed > 0 {
            tracing::info!("🧹 Cleaned {} old textures from cache", removed);
        }
    }

    /// 获取缓存统计信息
    pub fn get_cache_stats(&self) -> (usize, usize) {
        let total = self.cached_info.len();
        let used = self
            .cached_info
            .values()
            .filter(|info| info.image.is_some())
            .count();
        (total, used)
    }

    // ===== 图像属性访问方法 (对应 C# GetOffSet/GetSize/GetTrueSize) =====

    /// 获取图像偏移量
    ///
    /// 对应 C# 实现:
    /// ```csharp
    /// // MLibrary.cs line 641-654
    /// public Point GetOffSet(int index) {
    ///     if (!_initialized) Initialize();
    ///     if (_images == null || index < 0 || index >= _images.Length)
    ///         return Point.Empty;
    ///     if (_images[index] == null) {
    ///         _fStream.Seek(_indexList[index], SeekOrigin.Begin);
    ///         _images[index] = new MImage(_reader);
    ///     }
    ///     return new Point(_images[index].X, _images[index].Y);
    /// }
    /// ```
    ///
    /// # 参数
    /// - `index`: 图像索引
    ///
    /// # 返回
    /// - `Ok((x, y))`: 图像偏移量 (x, y)
    /// - `Err`: 索引无效或读取失败
    pub fn get_offset(&mut self, index: usize) -> io::Result<(i16, i16)> {
        // 检查索引范围
        if index >= self.indices.len() {
            return Ok((0, 0)); // 对应 C# 的 Point.Empty
        }

        // 获取或读取图像信息
        let info = self.get_image_info(index)?;

        Ok((info.x, info.y))
    }

    /// 获取图像尺寸
    ///
    /// 对应 C# 实现:
    /// ```csharp
    /// // MLibrary.cs line 655-667
    /// public Size GetSize(int index) {
    ///     if (!_initialized) Initialize();
    ///     if (_images == null || index < 0 || index >= _images.Length)
    ///         return Size.Empty;
    ///     if (_images[index] == null) {
    ///         _fStream.Seek(_indexList[index], SeekOrigin.Begin);
    ///         _images[index] = new MImage(_reader);
    ///     }
    ///     return new Size(_images[index].Width, _images[index].Height);
    /// }
    /// ```
    ///
    /// # 参数
    /// - `index`: 图像索引
    ///
    /// # 返回
    /// - `Ok((width, height))`: 图像尺寸 (width, height)
    /// - `Err`: 索引无效或读取失败
    pub fn get_size(&mut self, index: usize) -> io::Result<(i16, i16)> {
        // 检查索引范围
        if index >= self.indices.len() {
            return Ok((0, 0)); // 对应 C# 的 Size.Empty
        }

        // 获取或读取图像信息
        let info = self.get_image_info(index)?;

        Ok((info.width, info.height))
    }

    /// 获取图像实际尺寸（排除透明边缘）
    ///
    /// 对应 C# 实现:
    /// ```csharp
    /// // MLibrary.cs line 668-699
    /// public Size GetTrueSize(int index) {
    ///     if (!_initialized) Initialize();
    ///     if (_images == null || index < 0 || index >= _images.Length)
    ///         return Size.Empty;
    ///     if (_images[index] == null) {
    ///         _fStream.Position = _indexList[index];
    ///         _images[index] = new MImage(_reader);
    ///     }
    ///     MImage mi = _images[index];
    ///     if (mi.TrueSize.IsEmpty) {
    ///         if (!mi.TextureValid) {
    ///             if ((mi.Width == 0) || (mi.Height == 0))
    ///                 return Size.Empty;
    ///             _fStream.Seek(_indexList[index] + 17, SeekOrigin.Begin);
    ///             mi.CreateTexture(_reader);
    ///         }
    ///         return mi.GetTrueSize();
    ///     }
    ///     return mi.TrueSize;
    /// }
    /// ```
    ///
    /// # 参数
    /// - `ctx`: ggez Context (用于创建纹理)
    /// - `index`: 图像索引
    ///
    /// # 返回
    /// - `Ok((width, height))`: 实际尺寸 (width, height)
    /// - `Err`: 索引无效、图像无效或读取失败
    ///
    /// # 说明
    /// - 如果已经计算过，直接返回缓存的值
    /// - 如果还没有纹理数据，会先加载纹理
    /// - 然后调用 `ImageInfo::get_true_size()` 计算实际边界
    pub fn get_true_size(
        &mut self,
        
        index: usize,
    ) -> io::Result<(i16, i16)> {
        // 检查索引范围
        if index >= self.indices.len() {
            return Ok((0, 0)); // 对应 C# 的 Size.Empty
        }
        // 获取或读取图像信息
        let info = self.get_or_create_texture( index)?;

        // 检查图像是否有效
        if info.width == 0 || info.height == 0 {
            return Ok((0, 0)); // 对应 C# 的 Size.Empty
        }
        // 计算实际尺寸
        let true_size = info.get_true_size();

        Ok(true_size)
    }
} // impl MLibrary 结束

// ============================================================================
// 绘制函数 (macroquad 版本不需要 - 直接在渲染器中使用纹理)
// ============================================================================
// 注意：macroquad 版本在渲染器中直接使用 draw_texture_ex 绘制，
// 因此不需要这些 ggez 风格的绘制辅助函数。
// 如果需要，可以后续添加 macroquad 版本的绘制辅助函数。
// ============================================================================

/*
    // ===== Ggez 渲染函数 =====

    /// 基础绘制方法 - Draw(index, x, y)
    ///
    /// 对应 C# 实现:
    /// ```csharp
    /// // MLibrary.cs line 701-716
    /// public void Draw(int index, int x, int y) {
    ///     if (x >= Settings.ScreenWidth || y >= Settings.ScreenHeight) return;
    ///     if (!CheckImage(index)) return;
    ///     MImage mi = _images[index];
    ///     if (x + mi.Width < 0 || y + mi.Height < 0) return;
    ///     DXManager.Draw(mi.Image, new Rectangle(0, 0, mi.Width, mi.Height),
    ///                    new Vector3((float)x, (float)y, 0.0F), Color.White);
    ///     mi.CleanTime = CMain.Time + Settings.CleanDelay;
    /// }
    /// ```
    pub fn draw(
        &mut self,
        
        canvas: &mut ggez::graphics::Canvas,
        index: usize,
        x: f32,
        y: f32,
    ) -> io::Result<()> {
        let (screen_width, screen_height) = ctx.drawable_size();
        // 屏幕裁剪检查
        if x >= screen_width || y >= screen_height {
            return Ok(());
        }

        // 获取或创建纹理
        let info = self.get_or_create_texture( index)?;

        // 边界检查
        if x + (info.width as f32) < 0.0 || y + (info.height as f32) < 0.0 {
            return Ok(());
        }

        // 获取纹理
        if let Some(ref image) = info.image {
            canvas.draw(image, DrawParam::default().dest([x, y]).color(Color::WHITE));
        }

        Ok(())
    }

    /// 带颜色和偏移的绘制 - Draw(index, point, colour, offSet)
    ///
    /// 对应 C# 实现:
    /// ```csharp
    /// // MLibrary.cs line 717-730
    /// public void Draw(int index, Point point, Color colour, bool offSet = false) {
    ///     if (!CheckImage(index)) return;
    ///     MImage mi = _images[index];
    ///     if (offSet) point.Offset(mi.X, mi.Y);
    ///     if (point.X >= Settings.ScreenWidth || ...) return;
    ///     DXManager.Draw(mi.Image, new Rectangle(0, 0, mi.Width, mi.Height),
    ///                    new Vector3((float)point.X, (float)point.Y, 0.0F), colour);
    ///     mi.CleanTime = CMain.Time + Settings.CleanDelay;
    /// }
    /// ```
    pub fn draw_with_color(
        &mut self,
        
        canvas: &mut ggez::graphics::Canvas,
        index: usize,
        x: f32,
        y: f32,
        color: ggez::graphics::Color,
        offset: bool,
    ) -> io::Result<()> {
        // 🔧 使用 canvas 的屏幕坐标系,而不是物理像素
        // canvas.screen_coordinates() 返回当前设置的逻辑坐标系 (如 1024×768)
        let screen_rect = canvas.screen_coordinates().unwrap_or_else(|| {
            // 如果没有设置,使用物理像素作为回退
            let (w, h) = ctx.drawable_size();
            ggez::graphics::Rect::new(0.0, 0.0, w, h)
        });
        let (screen_width, screen_height) = (screen_rect.w, screen_rect.h);

        // 获取或创建纹理
        let info = self.get_or_create_texture( index)?;

        // 应用偏移
        let (draw_x, draw_y) = if offset {
            (x + info.x as f32, y + info.y as f32)
        } else {
            (x, y)
        };

        // 调试输出 (只对index=1即主面板背景)
        static mut DEBUG_COUNT: u32 = 0;
        unsafe {
            if index == 1 && DEBUG_COUNT < 3 {
                // println!("🔍 [mlibrary] index={}, pos=({}, {}), screen={}x{}",
                //          index, draw_x, draw_y, screen_width, screen_height);
                // println!("🔍 [mlibrary] texture size: {}x{}", info.width, info.height);
                DEBUG_COUNT += 1;
            }
        }

        // 屏幕裁剪检查
        if draw_x >= screen_width
            || draw_y >= screen_height
            || draw_x + (info.width as f32) < 0.0
            || draw_y + (info.height as f32) < 0.0
        {
            unsafe {
                if index == 1 && DEBUG_COUNT <= 3 {
                    println!("❌ [mlibrary] 裁剪检查失败! 元素在屏幕外");
                }
            }
            return Ok(());
        }

        // unsafe {
        //     if index == 1 && DEBUG_COUNT <= 3 {
        //         println!("✅ [mlibrary] 裁剪检查通过,即将调用 canvas.draw()");
        //     }
        // }

        // 绘制
        if let Some(ref image) = info.image {
            canvas.draw(
                image,
                DrawParam::default().dest([draw_x, draw_y]).color(color),
            );
            // unsafe {
            //     if index == 1 && DEBUG_COUNT <= 3 {
            //         println!("✅ [mlibrary] canvas.draw() 调用完成");
            //     }
            // }
        } else {
            unsafe {
                if index == 1 && DEBUG_COUNT <= 3 {
                    println!("❌ [mlibrary] 纹理图像不存在!");
                }
            }
        }

        Ok(())
    }

    /// 带缩放的绘制 - 支持摄像机缩放
    pub fn draw_with_scale(
        &mut self,
        
        canvas: &mut ggez::graphics::Canvas,
        index: usize,
        x: f32,
        y: f32,
        color: ggez::graphics::Color,
        offset: bool,
        scale: f32,
    ) -> io::Result<()> {
        use ggez::graphics::DrawParam;

        // 获取或创建纹理
        let info = self.get_or_create_texture( index)?;

        // 应用偏移 (偏移也需要缩放)
        let (draw_x, draw_y) = if offset {
            (x + info.x as f32 * scale, y + info.y as f32 * scale)
        } else {
            (x, y)
        };

        // 绘制
        if let Some(ref image) = info.image {
            canvas.draw(
                image,
                DrawParam::default()
                    .dest([draw_x, draw_y])
                    .color(color)
                    .scale([scale, scale]),
            );
        }

        Ok(())
    }

    /// 带透明度的绘制 - Draw(index, point, colour, offSet, opacity)
    ///
    /// 对应 C# 实现:
    /// ```csharp
    /// // MLibrary.cs line 735-750
    /// public void Draw(int index, Point point, Color colour, bool offSet, float opacity) {
    ///     if (!CheckImage(index)) return;
    ///     MImage mi = _images[index];
    ///     if (offSet) point.Offset(mi.X, mi.Y);
    ///     if (point.X >= Settings.ScreenWidth || ...) return;
    ///     DXManager.DrawOpaque(mi.Image, ..., colour, opacity);
    ///     mi.CleanTime = CMain.Time + Settings.CleanDelay;
    /// }
    /// ```
    pub fn draw_with_opacity(
        &mut self,
        
        canvas: &mut ggez::graphics::Canvas,
        index: usize,
        x: f32,
        y: f32,
        color: ggez::graphics::Color,
        offset: bool,
        opacity: f32,
    ) -> io::Result<()> {
        let (screen_width, screen_height) = ctx.drawable_size();

        // 获取或创建纹理
        let info = self.get_or_create_texture( index)?;

        // 应用偏移
        let (draw_x, draw_y) = if offset {
            (x + info.x as f32, y + info.y as f32)
        } else {
            (x, y)
        };

        // 屏幕裁剪检查
        if draw_x >= screen_width
            || draw_y >= screen_height
            || draw_x + (info.width as f32) < 0.0
            || draw_y + (info.height as f32) < 0.0
        {
            return Ok(());
        }

        // 应用透明度到颜色
        let mut color_with_opacity = color;
        color_with_opacity.a *= opacity;

        // 绘制
        if let Some(ref image) = info.image {
            canvas.draw(
                image,
                DrawParam::default()
                    .dest([draw_x, draw_y])
                    .color(color_with_opacity),
            );
        }

        Ok(())
    }

    /// 混合模式绘制 - DrawBlend(index, point, colour, offSet, rate)
    ///
    /// 对应 C# 实现:
    /// ```csharp
    /// // MLibrary.cs line 752-768
    /// public void DrawBlend(int index, Point point, Color colour, bool offSet = false, float rate = 1) {
    ///     if (!CheckImage(index)) return;
    ///     MImage mi = _images[index];
    ///     if (offSet) point.Offset(mi.X, mi.Y);
    ///     if (point.X >= Settings.ScreenWidth || ...) return;
    ///     bool oldBlend = DXManager.Blending;
    ///     DXManager.SetBlend(true, rate);
    ///     DXManager.Draw(mi.Image, ..., colour);
    ///     DXManager.SetBlend(oldBlend);
    ///     mi.CleanTime = CMain.Time + Settings.CleanDelay;
    /// }
    /// ```
    pub fn draw_blend(
        &mut self,
        
        canvas: &mut ggez::graphics::Canvas,
        index: usize,
        x: f32,
        y: f32,
        color: ggez::graphics::Color,
        offset: bool,
        rate: f32,
    ) -> io::Result<()> {
        let (screen_width, screen_height) = ctx.drawable_size();

        // 获取或创建纹理
        let info = self.get_or_create_texture( index)?;

        // 应用偏移
        let (draw_x, draw_y) = if offset {
            (x + info.x as f32, y + info.y as f32)
        } else {
            (x, y)
        };

        // 屏幕裁剪检查
        if draw_x >= screen_width
            || draw_y >= screen_height
            || draw_x + (info.width as f32) < 0.0
            || draw_y + (info.height as f32) < 0.0
        {
            return Ok(());
        }

        // 应用混合率到颜色
        let mut blend_color = color;
        blend_color.a *= rate;

        // 绘制 (ggez 默认使用 alpha blending)
        if let Some(ref image) = info.image {
            canvas.draw(
                image,
                DrawParam::default()
                    .dest([draw_x, draw_y])
                    .color(blend_color),
            );
        }

        Ok(())
    }

    /// 部分区域绘制 - Draw(index, section, point, colour, offSet)
    ///
    /// 对应 C# 实现:
    /// ```csharp
    /// // MLibrary.cs line 769-789
    /// public void Draw(int index, Rectangle section, Point point, Color colour, bool offSet) {
    ///     if (!CheckImage(index)) return;
    ///     MImage mi = _images[index];
    ///     if (offSet) point.Offset(mi.X, mi.Y);
    ///     if (point.X >= Settings.ScreenWidth || ...) return;
    ///     if (section.Right > mi.Width) section.Width -= section.Right - mi.Width;
    ///     if (section.Bottom > mi.Height) section.Height -= section.Bottom - mi.Height;
    ///     DXManager.Draw(mi.Image, section, new Vector3(...), colour);
    ///     mi.CleanTime = CMain.Time + Settings.CleanDelay;
    /// }
    /// ```
    pub fn draw_section(
        &mut self,
        
        canvas: &mut ggez::graphics::Canvas,
        index: usize,
        section_x: f32,
        section_y: f32,
        section_width: f32,
        section_height: f32,
        x: f32,
        y: f32,
        color: ggez::graphics::Color,
        offset: bool,
    ) -> io::Result<()> {
        let (screen_width, screen_height) = ctx.drawable_size();

        // 获取或创建纹理
        let info = self.get_or_create_texture( index)?;

        // 应用偏移
        let (draw_x, draw_y) = if offset {
            (x + info.x as f32, y + info.y as f32)
        } else {
            (x, y)
        };

        // 屏幕裁剪检查
        if draw_x >= screen_width
            || draw_y >= screen_height
            || draw_x + (info.width as f32) < 0.0
            || draw_y + (info.height as f32) < 0.0
        {
            return Ok(());
        }

        // 修正区域大小
        let mut adj_width = section_width;
        let mut adj_height = section_height;

        if section_x + section_width > info.width as f32 {
            adj_width = info.width as f32 - section_x;
        }
        if section_y + section_height > info.height as f32 {
            adj_height = info.height as f32 - section_y;
        }

        // 绘制
        if let Some(ref image) = info.image {
            canvas.draw(
                image,
                DrawParam::default()
                    .src(Rect::new(
                        section_x / info.width as f32,
                        section_y / info.height as f32,
                        adj_width / info.width as f32,
                        adj_height / info.height as f32,
                    ))
                    .dest([draw_x, draw_y])
                    .color(color),
            );
        }

        Ok(())
    }

    /// 部分区域带透明度绘制 - Draw(index, section, point, colour, opacity)
    ///
    /// 对应 C# 实现:
    /// ```csharp
    /// // MLibrary.cs line 790-807
    /// public void Draw(int index, Rectangle section, Point point, Color colour, float opacity) {
    ///     if (!CheckImage(index)) return;
    ///     MImage mi = _images[index];
    ///     if (point.X >= Settings.ScreenWidth || ...) return;
    ///     if (section.Right > mi.Width) section.Width -= section.Right - mi.Width;
    ///     if (section.Bottom > mi.Height) section.Height -= section.Bottom - mi.Height;
    ///     DXManager.DrawOpaque(mi.Image, section, ..., colour, opacity);
    ///     mi.CleanTime = CMain.Time + Settings.CleanDelay;
    /// }
    /// ```
    pub fn draw_section_with_opacity(
        &mut self,
        
        canvas: &mut ggez::graphics::Canvas,
        index: usize,
        section_x: f32,
        section_y: f32,
        section_width: f32,
        section_height: f32,
        x: f32,
        y: f32,
        color: ggez::graphics::Color,
        opacity: f32,
    ) -> io::Result<()> {
        let (screen_width, screen_height) = ctx.drawable_size();

        // 获取或创建纹理
        let info = self.get_or_create_texture( index)?;

        // 屏幕裁剪检查
        if x >= screen_width
            || y >= screen_height
            || x + (info.width as f32) < 0.0
            || y + (info.height as f32) < 0.0
        {
            return Ok(());
        }

        // 修正区域大小
        let mut adj_width = section_width;
        let mut adj_height = section_height;

        if section_x + section_width > info.width as f32 {
            adj_width = info.width as f32 - section_x;
        }
        if section_y + section_height > info.height as f32 {
            adj_height = info.height as f32 - section_y;
        }

        // 应用透明度
        let mut color_with_opacity = color;
        color_with_opacity.a *= opacity;

        // 绘制
        if let Some(ref image) = info.image {
            canvas.draw(
                image,
                DrawParam::default()
                    .src(Rect::new(
                        section_x / info.width as f32,
                        section_y / info.height as f32,
                        adj_width / info.width as f32,
                        adj_height / info.height as f32,
                    ))
                    .dest([x, y])
                    .color(color_with_opacity),
            );
        }

        Ok(())
    }

    /// 缩放绘制 - Draw(index, point, size, colour)
    ///
    /// 对应 C# 实现:
    /// ```csharp
    /// // MLibrary.cs line 808-827
    /// public void Draw(int index, Point point, Size size, Color colour) {
    ///     if (!CheckImage(index)) return;
    ///     MImage mi = _images[index];
    ///     if (point.X >= Settings.ScreenWidth || ...) return;
    ///     float scaleX = (float)size.Width / mi.Width;
    ///     float scaleY = (float)size.Height / mi.Height;
    ///     Matrix matrix = Matrix.Scaling(scaleX, scaleY, 0);
    ///     DXManager.Sprite.Transform = matrix;
    ///     DXManager.Draw(mi.Image, ..., Color.White);
    ///     DXManager.Sprite.Transform = Matrix.Identity;
    ///     mi.CleanTime = CMain.Time + Settings.CleanDelay;
    /// }
    /// ```
    pub fn draw_scaled(
        &mut self,
        
        canvas: &mut ggez::graphics::Canvas,
        index: usize,
        x: f32,
        y: f32,
        width: f32,
        height: f32,
        color: ggez::graphics::Color,
    ) -> io::Result<()> {
        let (screen_width, screen_height) = ctx.drawable_size();

        // 获取或创建纹理
        let info = self.get_or_create_texture( index)?;

        // 屏幕裁剪检查
        if x >= screen_width || y >= screen_height || x + width < 0.0 || y + height < 0.0 {
            return Ok(());
        }

        // 计算缩放比例
        let scale_x = width / info.width as f32;
        let scale_y = height / info.height as f32;

        // 绘制
        if let Some(ref image) = info.image {
            canvas.draw(
                image,
                DrawParam::default()
                    .dest([x, y])
                    .scale([scale_x, scale_y])
                    .color(color),
            );
        }

        Ok(())
    }

    /// 着色绘制（双层） - DrawTinted(index, point, colour, Tint, offSet)
    ///
    /// 对应 C# 实现:
    /// ```csharp
    /// // MLibrary.cs line 829-845
    /// public void DrawTinted(int index, Point point, Color colour, Color Tint, bool offSet = false) {
    ///     if (!CheckImage(index)) return;
    ///     MImage mi = _images[index];
    ///     if (offSet) point.Offset(mi.X, mi.Y);
    ///     if (point.X >= Settings.ScreenWidth || ...) return;
    ///     DXManager.Draw(mi.Image, ..., colour);
    ///     if (mi.HasMask) {
    ///         DXManager.Draw(mi.MaskImage, ..., Tint);
    ///     }
    ///     mi.CleanTime = CMain.Time + Settings.CleanDelay;
    /// }
    /// ```
    pub fn draw_tinted(
        &mut self,
        
        canvas: &mut ggez::graphics::Canvas,
        index: usize,
        x: f32,
        y: f32,
        color: ggez::graphics::Color,
        tint: ggez::graphics::Color,
        offset: bool,
    ) -> io::Result<()> {
        let (screen_width, screen_height) = ctx.drawable_size();

        // 获取或创建纹理
        let info = self.get_or_create_texture( index)?;

        // 应用偏移
        let (draw_x, draw_y) = if offset {
            (x + info.x as f32, y + info.y as f32)
        } else {
            (x, y)
        };

        // 屏幕裁剪检查
        if draw_x >= screen_width
            || draw_y >= screen_height
            || draw_x + (info.width as f32) < 0.0
            || draw_y + (info.height as f32) < 0.0
        {
            return Ok(());
        }

        // 绘制主图像
        if let Some(ref image) = info.image {
            canvas.draw(
                image,
                DrawParam::default().dest([draw_x, draw_y]).color(color),
            );
        }

        // 如果有遮罩层，绘制着色层
        if info.has_mask {
            if let Some(ref mask_image) = info.mask_image {
                canvas.draw(
                    mask_image,
                    DrawParam::default().dest([draw_x, draw_y]).color(tint),
                );
            }
        }

        Ok(())
    }

    /// 向上绘制（Y坐标减去高度） - DrawUp(index, x, y)
    ///
    /// 对应 C# 实现:
    /// ```csharp
    /// // MLibrary.cs line 847-862
    /// public void DrawUp(int index, int x, int y) {
    ///     if (x >= Settings.ScreenWidth) return;
    ///     if (!CheckImage(index)) return;
    ///     MImage mi = _images[index];
    ///     y -= mi.Height;
    ///     if (y >= Settings.ScreenHeight) return;
    ///     if (x + mi.Width < 0 || y + mi.Height < 0) return;
    ///     DXManager.Draw(mi.Image, ..., Color.White);
    ///     mi.CleanTime = CMain.Time + Settings.CleanDelay;
    /// }
    /// ```
    pub fn draw_up(
        &mut self,
        
        canvas: &mut ggez::graphics::Canvas,
        index: usize,
        x: f32,
        y: f32,
    ) -> io::Result<()> {
        let (screen_width, screen_height) = ctx.drawable_size();

        // 屏幕裁剪检查
        if x >= screen_width {
            return Ok(());
        }

        // 获取或创建纹理
        let info = self.get_or_create_texture( index)?;

        // Y坐标减去高度
        let draw_y = y - info.height as f32;

        // 边界检查
        if draw_y >= screen_height
            || x + (info.width as f32) < 0.0
            || draw_y + (info.height as f32) < 0.0
        {
            return Ok(());
        }

        // 绘制
        if let Some(ref image) = info.image {
            canvas.draw(
                image,
                DrawParam::default().dest([x, draw_y]).color(Color::WHITE),
            );
        }

        Ok(())
    }

    /// 向上混合绘制 - DrawUpBlend(index, point)
    ///
    /// 对应 C# 实现:
    /// ```csharp
    /// // MLibrary.cs line 863-880
    /// public void DrawUpBlend(int index, Point point) {
    ///     if (!CheckImage(index)) return;
    ///     MImage mi = _images[index];
    ///     point.Y -= mi.Height;
    ///     if (point.X >= Settings.ScreenWidth || ...) return;
    ///     bool oldBlend = DXManager.Blending;
    ///     DXManager.SetBlend(true, 1);
    ///     DXManager.Draw(mi.Image, ..., Color.White);
    ///     DXManager.SetBlend(oldBlend);
    ///     mi.CleanTime = CMain.Time + Settings.CleanDelay;
    /// }
    /// ```
    pub fn draw_up_blend(
        &mut self,
        
        canvas: &mut ggez::graphics::Canvas,
        index: usize,
        x: f32,
        y: f32,
    ) -> io::Result<()> {
        let (screen_width, screen_height) = ctx.drawable_size();

        // 获取或创建纹理
        let info = self.get_or_create_texture( index)?;

        // Y坐标减去高度
        let draw_y = y - info.height as f32;

        // 屏幕裁剪检查
        if x >= screen_width
            || draw_y >= screen_height
            || x + (info.width as f32) < 0.0
            || draw_y + (info.height as f32) < 0.0
        {
            return Ok(());
        }

        // 绘制 (ggez 默认使用 alpha blending)
        if let Some(ref image) = info.image {
            canvas.draw(
                image,
                DrawParam::default().dest([x, draw_y]).color(Color::WHITE),
            );
        }

        Ok(())
    }

    /// 像素可见性检测（带精度控制） - VisiblePixel(index, point, accurate)
    ///
    /// 对应 C# 实现:
    /// ```csharp
    /// // MLibrary.cs line 882-897
    /// public bool VisiblePixel(int index, Point point, bool accuate) {
    ///     if (!CheckImage(index)) return false;
    ///     if (accuate)
    ///         return _images[index].VisiblePixel(point);
    ///     int accuracy = 2;
    ///     for (int x = -accuracy; x <= accuracy; x++)
    ///         for (int y = -accuracy; y <= accuracy; y++)
    ///             if (_images[index].VisiblePixel(new Point(point.X + x, point.Y + y)))
    ///                 return true;
    ///     return false;
    /// }
    /// ```
    pub fn visible_pixel(
        &mut self,
        
        index: usize,
        x: i32,
        y: i32,
        accurate: bool,
    ) -> io::Result<bool> {
        // 获取或创建纹理（确保 rgba_data 已加载）
        let info = self.get_or_create_texture( index)?;

        if accurate {
            // 精确模式：直接检测指定像素
            Ok(info.visible_pixel(x, y))
        } else {
            // 模糊模式：检测周围 5x5 区域
            let accuracy = 2;
            for dx in -accuracy..=accuracy {
                for dy in -accuracy..=accuracy {
                    if info.visible_pixel(x + dx, y + dy) {
                        return Ok(true);
                    }
                }
            }
            Ok(false)
        }
    }
}

// ============================================================================
// 单元测试
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    // ============================================================================
    // 测试 ImageInfo 结构
    // ============================================================================
    #[test]
    fn test_image_info_creation() {
        let info = ImageInfo {
            width: 48,
            height: 32,
            x: -24,
            y: -16,
            shadow_x: 0,
            shadow_y: 0,
            shadow: 0,
            length: 0,
            mask_width: 48,
            mask_height: 32,
            mask_x: -24,
            mask_y: -16,
            mask_length: 0,
            texture_valid: false,
            image: None,
            mask_image: None,
            has_mask: false,
            last_access_time: None,
            bgra_data: None,
        };

        assert_eq!(info.width, 48);
        assert_eq!(info.height, 32);
        assert_eq!(info.x, -24);
        assert_eq!(info.y, -16);
        assert!(!info.has_mask);
        assert!(!info.texture_valid);
    }

    // ============================================================================
    // 测试偏移量应用逻辑
    // ============================================================================
    #[test]
    fn test_offset_calculation() {
        // 模拟 C# 的偏移逻辑
        let info_x = -24i16;
        let info_y = -16i16;

        let base_x = 100f32;
        let base_y = 200f32;

        // 不使用 offset (C#: 直接使用 point)
        let (x1, y1) = (base_x, base_y);
        assert_eq!(x1, 100.0);
        assert_eq!(y1, 200.0);

        // 使用 offset (C#: if (offSet) point.Offset(mi.X, mi.Y))
        let (x2, y2) = (base_x + info_x as f32, base_y + info_y as f32);
        assert_eq!(x2, 76.0); // 100 + (-24)
        assert_eq!(y2, 184.0); // 200 + (-16)
    }

    // ============================================================================
    // 测试屏幕裁剪逻辑
    // ============================================================================
    #[test]
    fn test_screen_clipping() {
        let screen_width = 800.0;
        let screen_height = 600.0;

        // C# 逻辑:
        // if (x >= ScreenWidth || y >= ScreenHeight ||
        //     x + width < 0 || y + height < 0) return;

        let test_cases = [
            // (x, y, width, height, should_cull, description)
            (850.0, 300.0, 48, 32, true, "x >= screen_width"),
            (400.0, 650.0, 48, 32, true, "y >= screen_height"),
            (-50.0, 300.0, 48, 32, true, "x + width < 0"),
            (400.0, -40.0, 48, 32, true, "y + height < 0"),
            (100.0, 100.0, 48, 32, false, "正常范围内"),
            (770.0, 580.0, 48, 32, false, "边界情况（部分可见）"),
            (-10.0, 100.0, 48, 32, false, "左边缘部分可见"),
            (100.0, -10.0, 48, 32, false, "上边缘部分可见"),
        ];

        for (x, y, width, height, should_cull, desc) in test_cases {
            let culled = x >= screen_width
                || y >= screen_height
                || x + (width as f32) < 0.0
                || y + (height as f32) < 0.0;
            assert_eq!(
                culled, should_cull,
                "Failed for case '{}': ({}, {}) with size {}x{}",
                desc, x, y, width, height
            );
        }
    }

    // ============================================================================
    // 测试图像索引边界检查
    // ============================================================================
    #[test]
    fn test_index_bounds_check() {
        // 模拟 C# 的边界检查逻辑
        let image_count = 100;

        // C#: if (index < 0 || index >= _images.Length) return false;
        let test_cases = [
            (-1, true, "负数索引应该被拒绝"),
            (0, false, "索引 0 应该通过"),
            (50, false, "中间索引应该通过"),
            (99, false, "最大有效索引应该通过"),
            (100, true, "等于长度的索引应该被拒绝"),
            (200, true, "超大索引应该被拒绝"),
        ];

        for (index, should_fail, desc) in test_cases {
            let failed = index < 0 || index >= image_count;
            assert_eq!(
                failed, should_fail,
                "Failed for case '{}': index {}",
                desc, index
            );
        }
    }

    // ============================================================================
    // 测试 BackImage 标记处理（与 MapCode 配合）
    // ============================================================================
    #[test]
    fn test_back_image_masking() {
        // C# 中 BackImage 的高3位用于标记
        // 绘制时需要屏蔽: index = (BackImage & 0x1FFFFFFF) - 1

        let test_cases = [
            (0x00000001, 0),           // 普通图像索引 1 -> 0
            (0x00000064, 99),          // 普通图像索引 100 -> 99
            (0x20000001, 0),           // 带标记的索引 1 -> 0
            (0x20000064, 99),          // 带标记的索引 100 -> 99
            (0xE0000001u32 as i32, 0), // 多个标记位 -> 0
        ];

        for (back_image, expected_index) in test_cases {
            let index = ((back_image & 0x1FFFFFFF) - 1) as usize;
            assert_eq!(
                index, expected_index,
                "BackImage 0x{:08X} should yield index {}",
                back_image, expected_index
            );
        }
    }

    // ============================================================================
    // 测试 FrontImage 标记处理
    // ============================================================================
    #[test]
    fn test_front_image_masking() {
        // C# 中 FrontImage 的高位用于标记
        // 绘制时需要屏蔽: index = (FrontImage & 0x7FFF) - 1

        let test_cases = [
            (0x0001, 0),  // 普通索引 1 -> 0
            (0x0064, 99), // 普通索引 100 -> 99
            (0x8001, 0),  // 带标记的索引 1 -> 0
            (0x8064, 99), // 带标记的索引 100 -> 99
        ];

        for (front_image, expected_index) in test_cases {
            let index = ((front_image & 0x7FFF) - 1) as usize;
            assert_eq!(
                index, expected_index,
                "FrontImage 0x{:04X} should yield index {}",
                front_image, expected_index
            );
        }
    }

    // ============================================================================
    // 测试瓦片动画计算
    // ============================================================================
    #[test]
    fn test_tile_animation_calculation() {
        // C# 逻辑:
        // int animationoffset = M2CellInfo[x, y].TileAnimationOffset ^ 0x2000;
        // index += animationoffset * (AnimationCount % animation);

        let base_index = 100;
        let animation_offset = 0x2000i16 ^ 0x2000; // 结果为 0
        let animation_frames = 8u8;
        let animation_count = 15u32;

        // 计算当前帧
        let current_frame = animation_count % animation_frames as u32;
        let final_index = base_index + (animation_offset as i32) * (current_frame as i32);

        assert_eq!(final_index, 100); // offset=0 时索引不变

        // 测试非零偏移
        let animation_offset2 = 0x2100i16 ^ 0x2000; // 0x0100 = 256
        let final_index2 = base_index + (animation_offset2 as i32) * (current_frame as i32);
        // current_frame = 15 % 8 = 7
        // final_index = 100 + 256 * 7 = 1892
        assert_eq!(final_index2, 1892);
    }

    // ============================================================================
    // 测试混合模式标记
    // ============================================================================
    #[test]
    fn test_animation_blend_flag() {
        // C# 中动画帧数可能包含混合标记
        // if ((animation & 0x80) > 0) blend = true;
        // animation &= 0x7F;

        let test_cases = [
            (0x00, false, 0x00, "无混合，帧数 0"),
            (0x08, false, 0x08, "无混合，帧数 8"),
            (0x80, true, 0x00, "有混合，帧数 0"),
            (0x88, true, 0x08, "有混合，帧数 8"),
            (0xFF, true, 0x7F, "有混合，帧数 127"),
        ];

        for (raw_value, expected_blend, expected_frames, desc) in test_cases {
            let blend = (raw_value & 0x80) > 0;
            let frames = raw_value & 0x7F;

            assert_eq!(
                blend, expected_blend,
                "Blend flag mismatch for case '{}'",
                desc
            );
            assert_eq!(
                frames, expected_frames,
                "Frame count mismatch for case '{}'",
                desc
            );
        }
    }

    // ============================================================================
    // 测试门动画索引计算
    // ============================================================================
    #[test]
    fn test_door_animation_calculation() {
        // C# 逻辑:
        // if (DoorInfo.DoorState != 0) {
        //     index += (DoorInfo.ImageIndex + 1) * M2CellInfo[x, y].DoorOffset;
        // }

        let base_index = 1000;
        let door_image_index = 3; // 门动画第3帧
        let door_offset = 10; // 每帧偏移10

        // 门关闭状态（DoorState = 0）
        let closed_index = base_index;
        assert_eq!(closed_index, 1000);

        // 门打开状态（DoorState != 0）
        let open_index = base_index + (door_image_index + 1) * door_offset;
        assert_eq!(open_index, 1040); // 1000 + 4 * 10
    }
}
*/
