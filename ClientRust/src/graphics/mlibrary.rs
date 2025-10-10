// MLibrary - MIR2 图像库加载器
// 对应: Client/MirGraphics/MLibrary.cs
//
// 负责解析和加载 .lib 文件格式（MIR2 专有的图像库格式）

use crate::objects::frames::{Frame, FrameSet};
use byteorder::LittleEndian;
use byteorder::{ByteOrder, ReadBytesExt};
use flate2::read::GzDecoder;
use ggez::graphics::{Image, ImageFormat};
use ggez::Context;
use mir2_shared::MirAction;
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
    pub texture_valid: bool,                       // 纹理是否有效
    pub image: Option<ggez::graphics::Image>,      // 解压后的纹理数据 (RGBA格式)
    pub mask_image: Option<ggez::graphics::Image>, // 解压后的遮罩纹理数据 (RGBA格式)
    pub last_access_time: Option<Instant>,         // 最后访问时间 (用于缓存清理)
    rgba_data: Option<Vec<u8>>,                    // 原始解压数据 (RGBA格式)
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
            rgba_data: None,
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
        ctx: &mut ggez::Context,
        reader: &mut R,
    ) -> Result<(), std::io::Error> {
        // 读取主图像的压缩数据
        let mut compressed_data = vec![0u8; self.length as usize];
        reader.read_exact(&mut compressed_data)?;

        // 解压主图像
        let main_image = Self::decompress_image(&compressed_data, self.width, self.height)?;
        self.rgba_data = Some(main_image.clone()); // 保存原始数据副本
        self.image = Some(ggez::graphics::Image::from_pixels(
            ctx,
            &main_image,
            ImageFormat::Rgba8Unorm,
            self.width as u32,
            self.height as u32,
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
            let mask_data = Self::decompress_image(&mask_compressed, self.width, self.height)?;
            self.mask_image = Some(ggez::graphics::Image::from_pixels(
                ctx,
                &mask_data,
                ImageFormat::Rgba8Unorm,
                self.width as u32,
                self.height as u32,
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
        self.rgba_data.take();
        self.texture_valid = false;
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
    /// - `Ok(Vec<u8>)`: RGBA格式的图像数据
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

        // 转换 BGRA -> RGBA，并处理透明色
        // MIR2 格式: B G R A (每个像素4字节)
        // 目标格式: R G B A
        let mut rgba_data = Vec::with_capacity(decompressed.len());

        for chunk in decompressed.chunks_exact(4) {
            let b = chunk[0];
            let g = chunk[1];
            let r = chunk[2];
            let mut a = chunk[3];

            // 🔧 传奇2关键特性: 黑色被视为透明色
            // 对应 C# 中的隐式行为
            if r == 0 && g == 0 && b == 0 {
                a = 0;
            }

            rgba_data.push(r);
            rgba_data.push(g);
            rgba_data.push(b);
            rgba_data.push(a);
        }

        Ok(rgba_data)
    }

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
        if let Some(ref rgba_data) = self.rgba_data {
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
    frames: FrameSet,
    cached_info: Vec<ImageInfo>,
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
        Ok(Self {
            path: path_buf,
            header,
            indices,
            frames,
            cached_info: Vec::new(),
            reader,
        })
    }

    /// 获取图像数量
    pub fn count(&self) -> usize {
        self.indices.len()
    }

    pub fn frames(&self) -> &FrameSet {
        &self.frames
    }

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

        // 检查缓存
        if let Some(info) = self.cached_info.get(index) {
            return Ok(info.clone());
        }
        let offset = self.indices[index].offset as u64;
        self.reader.seek(SeekFrom::Start(offset))?;
        let info = ImageInfo::from_reader(&mut self.reader)?;
        self.cached_info.insert(index, info.clone());
        Ok(info)
    }

    // /// 加载图像数据为RGBA8像素数组
    // pub fn load_image_data(&mut self, index: usize) -> io::Result<(ImageInfo, Vec<u8>)> {
    //     let info = self.get_image_info(index)?;

    //     // ✅ C# CheckImage 检查: if (Width == 0 || Height == 0) return false;
    //     if info.width == 0 || info.height == 0 {
    //         return Err(io::Error::new(
    //             io::ErrorKind::InvalidData,
    //             format!(
    //                 "图像 {} 无效: width={}, height={}",
    //                 index, info.width, info.height
    //             ),
    //         ));
    //     }

    //     // ✅ 检查 Length 是否有效
    //     if info.length <= 0 {
    //         return Err(io::Error::new(
    //             io::ErrorKind::InvalidData,
    //             format!("图像 {} 无效: length={}", index, info.length),
    //         ));
    //     }

    //     let offset = self.indices[index].offset as u64;
    //     let mut file = File::open(&self.path)?;

    //     // 跳过图像信息头(17字节)
    //     file.seek(SeekFrom::Start(offset + 17))?;

    //     // Length 字段就是压缩数据的实际长度 (对应 C# 的 reader.ReadBytes(Length))
    //     let compressed_size = info.length as usize;

    //     // 读取压缩数据
    //     let mut compressed = vec![0u8; compressed_size];
    //     file.read_exact(&mut compressed)?;

    //     // 解压(GZip格式)
    //     let mut decompressor = GzDecoder::new(&compressed[..]);
    //     let mut decompressed = Vec::new();
    //     match decompressor.read_to_end(&mut decompressed) {
    //         Ok(_) => {}
    //         Err(e) => {
    //             tracing::error!(
    //                 "⚠️ 图像 {} 解压失败: width={}, height={}, length={}, offset={}, error={:?}",
    //                 index,
    //                 info.width,
    //                 info.height,
    //                 info.length,
    //                 offset,
    //                 e
    //             );
    //             return Err(e);
    //         }
    //     }

    //     // 验证解压后的大小
    //     let expected_size = (info.width as usize) * (info.height as usize) * 4;
    //     if decompressed.len() != expected_size {
    //         // 尝试修正数据大小
    //         if decompressed.len() > expected_size {
    //             // 数据过长,截断
    //             // 降级到DEBUG,避免每帧重复警告
    //             tracing::debug!(
    //                 "⚠️ 图像 {} 数据过长 ({} > {}), 截断",
    //                 index,
    //                 decompressed.len(),
    //                 expected_size
    //             );
    //             decompressed.truncate(expected_size);
    //         } else {
    //             // 数据过短,填充透明像素
    //             tracing::debug!(
    //                 "⚠️ 图像 {} 数据过短 ({} < {}), 填充",
    //                 index,
    //                 decompressed.len(),
    //                 expected_size
    //             );
    //             decompressed.resize(expected_size, 0);
    //         }
    //     }

    //     Ok((info, decompressed))
    // }

    // /// 加载图像数据为 RGBA 格式
    // ///
    // /// C# equivalent: MLibrary 内部解压逻辑
    // ///
    // /// 返回: (ImageInfo, RGBA字节数据)
    // pub fn load_rgba_data(&mut self, index: usize) -> io::Result<(ImageInfo, Vec<u8>)> {
    //     let (info, data) = self.load_image_data(index)?;

    //     let width = info.width as usize;
    //     let height = info.height as usize;

    //     // MIR2使用BGRA格式,需要转换为RGBA
    //     // 关键修复: 将黑色像素(0,0,0)转换为透明色(0,0,0,0)
    //     let mut rgba_data = Vec::with_capacity(data.len());

    //     for chunk in data.chunks_exact(4) {
    //         let b = chunk[0];
    //         let g = chunk[1];
    //         let r = chunk[2];
    //         let mut a = chunk[3];

    //         // 🔧 传奇2关键特性: 黑色被视为透明色
    //         if r == 0 && g == 0 && b == 0 {
    //             a = 0;
    //         }

    //         rgba_data.push(r);
    //         rgba_data.push(g);
    //         rgba_data.push(b);
    //         rgba_data.push(a);
    //     }

    //     Ok((info, rgba_data))
    // }

    // ===== 纹理缓存系统 (对应 C# MImage.CreateTexture + DXManager.TextureList) =====

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
    /// - `Ok(&Image)`: 缓存的纹理引用
    /// - `Err`: 加载失败
    pub fn get_or_create_texture(
        &mut self,
        ctx: &mut ggez::Context,
        index: usize,
    ) -> io::Result<ImageInfo> {
        // // 尝试从缓存读取（仅共享借用，避免与后续可变借用冲突）
        // 读取图像信息（必要时从文件解析）
        let mut info = self.get_image_info(index)?;
        if info.width == 0 || info.height == 0 {
            return Err(io::Error::new(
                io::ErrorKind::InvalidData,
                format!(
                    "图像 {} 无效: width={}, height={}",
                    index, info.width, info.height
                ),
            ));
        }
        if info.image.is_some() {
            // 已有纹理，更新访问时间并返回
            info.last_access_time = Some(Instant::now());
            self.cached_info[index] = info.clone();
            return Ok(info);
        }
        // 将文件指针定位到压缩数据起始位置（跳过17字节的头）
        let offset = self.indices[index].offset as u64;
        self.reader.seek(SeekFrom::Start(offset + 17))?;
        // 创建纹理（内部会更新 last_access_time）
        info.create_texture(ctx, &mut self.reader)?;
        // 写回缓存
        self.cached_info.insert(index, info.clone());
        Ok(info)
    }

    // /// 垂直翻转纹理数据
    // ///
    // /// 用于在 DirectX (Y轴向下) 和 OpenGL (Y轴向上) 之间转换
    // #[allow(dead_code)]
    // fn flip_texture_vertically(data: &mut [u8], width: usize, height: usize) {
    //     let row_size = width * 4; // RGBA, 4 bytes per pixel
    //     let mut row_buffer = vec![0u8; row_size];

    //     for y in 0..(height / 2) {
    //         let top_offset = y * row_size;
    //         let bottom_offset = (height - 1 - y) * row_size;

    //         // 交换两行
    //         row_buffer.copy_from_slice(&data[top_offset..top_offset + row_size]);
    //         data.copy_within(bottom_offset..bottom_offset + row_size, top_offset);
    //         data[bottom_offset..bottom_offset + row_size].copy_from_slice(&row_buffer);
    //     }
    // }

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
        self.cached_info.iter_mut().for_each(|image| {
            if let Some(access_time) = image.last_access_time {
                let age = now.duration_since(access_time);
                if age > max_age {
                    image.image = None; // 释放主纹理
                    image.mask_image = None; // 释放遮罩纹理
                    image.last_access_time = None; // 清除访问时间
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
            .iter()
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
        ctx: &mut ggez::Context,
        index: usize,
    ) -> io::Result<(i16, i16)> {
        // 检查索引范围
        if index >= self.indices.len() {
            return Ok((0, 0)); // 对应 C# 的 Size.Empty
        }
        // 获取或读取图像信息
        let  info = self.get_or_create_texture(ctx,index)?;

        // 检查图像是否有效
        if info.width == 0 || info.height == 0 {
            return Ok((0, 0)); // 对应 C# 的 Size.Empty
        }
        // 计算实际尺寸
        let true_size = info.get_true_size();

        Ok(true_size)
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
        Ok(())
    }
}
