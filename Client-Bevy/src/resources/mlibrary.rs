// MLibrary - MIR2 图像库加载器 (Bevy 版本)
// 对应: Client/MirGraphics/MLibrary.cs
//
// 与 Client-Macroquad/src/resources/mlibrary.rs 保持相同解析逻辑，
// 但去掉 macroquad 纹理耦合：只保存原始 RGBA 数据，由 Bevy Asset 层创建 Image。

use byteorder::LittleEndian;
use byteorder::ReadBytesExt;
use flate2::read::GzDecoder;

use std::collections::HashMap;
use std::fs::File;
use std::io::{self, BufReader, Read, Seek, SeekFrom};
use std::path::{Path, PathBuf};

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

/// 图像元数据 + 原始像素数据（不依赖任何渲染引擎）
#[derive(Clone)]
pub struct ImageInfo {
    pub width: i16,
    pub height: i16,
    pub offset_x: i16, // 偏移量X
    pub offset_y: i16, // 偏移量Y
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
    /// 主图像 RGBA 数据（未解压时为 None）
    pub rgba: Option<Vec<u8>>,
    /// 遮罩层 RGBA 数据（未解压/无遮罩时为 None）
    pub mask_rgba: Option<Vec<u8>>,
}

impl ImageInfo {
    /// 从读取器中解析 ImageInfo (17字节)
    ///
    /// 注意: 这里不读取纹理数据,只读取元数据
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
            offset_x: x,
            offset_y: y,
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
            rgba: None,
            mask_rgba: None,
        })
    }

    /// 从当前读取器位置读取并解压主图像 + 遮罩层数据（RGBA）
    ///
    /// 对应 C# MImage.CreateTexture 的数据读取部分（不含渲染纹理创建）。
    pub fn load_data<R: std::io::Read + Seek>(
        &mut self,
        reader: &mut R,
    ) -> Result<(), std::io::Error> {
        // 读取主图像的压缩数据
        let mut compressed_data = vec![0u8; self.length as usize];
        reader.read_exact(&mut compressed_data)?;

        // 解压主图像
        let mut main_image = Self::decompress_image(&compressed_data, self.width, self.height)?;

        // BGRA -> RGBA
        Self::bgra_to_rgba(&mut main_image);
        self.rgba = Some(main_image);

        // 遮罩层
        if self.has_mask {
            // 跳过12字节的遮罩头信息（C#: reader.ReadBytes(12)）
            reader.seek(SeekFrom::Current(12))?;
            let mut mask_compressed = vec![0u8; self.mask_length as usize];
            reader.read_exact(&mut mask_compressed)?;

            let mut mask_data = Self::decompress_image(&mask_compressed, self.width, self.height)?;
            Self::bgra_to_rgba(&mut mask_data);
            self.mask_rgba = Some(mask_data);
        }
        Ok(())
    }

    /// 解压图像数据并转换为RGBA格式
    ///
    /// 对应 C# MImage.DecompressImage
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
                tracing::debug!(
                    "⚠️ 图像数据过长 ({} > {}), 截断",
                    decompressed.len(),
                    expected_size
                );
                decompressed.truncate(expected_size);
            } else {
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

    /// BGRA 转 RGBA + 黑色背景透明化
    fn bgra_to_rgba(data: &mut [u8]) {
        for chunk in data.chunks_exact_mut(4) {
            let b = chunk[0];
            let g = chunk[1];
            let r = chunk[2];

            // 纯黑背景 → 完全透明（严格纯黑，避免误杀深色描边）
            let is_pure_black = r == 0 && g == 0 && b == 0;

            // BGRA -> RGBA: 交换 B 和 R 通道
            chunk[0] = r;
            chunk[2] = b;

            if is_pure_black {
                chunk[3] = 0;
            }
        }
    }

    /// 检查指定像素是否可见（非透明）
    pub fn visible_pixel(&self, x: i32, y: i32) -> bool {
        if let Some(ref rgba_data) = self.rgba {
            if x < 0 || y < 0 || x >= self.width as i32 || y >= self.height as i32 {
                return false;
            }
            let w = self.width as usize;
            let index = ((y as usize) * (w << 2)) + ((x as usize) << 2) + 3;
            if index < rgba_data.len() {
                return rgba_data[index] != 0;
            }
        }
        false
    }

    pub fn get_size(&self) -> (i16, i16) {
        (self.width, self.height)
    }

    /// 获取图像的实际显示尺寸（去除透明边缘）
    pub fn get_true_size(&self) -> (i16, i16) {
        let mut l = 0i32;
        let mut t = 0i32;
        let mut r = self.width as i32;
        let mut b = self.height as i32;

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

        let width = (r - l) as i16;
        let height = (b - t) as i16;
        (width, height)
    }
}

/// MIR2图像库
pub struct MLibrary {
    path: PathBuf,
    indices: Vec<ImageIndex>,
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

        // Version 3的.Lib文件有frame_seek字段
        if version >= 3 {
            let _frame_seek = reader.read_i32::<LittleEndian>()?;
        }

        let mut indices = Vec::with_capacity(count as usize);
        for _ in 0..count {
            let offset = reader.read_i32::<LittleEndian>()?;
            indices.push(ImageIndex { offset });
        }

        let cached_info = HashMap::with_capacity(count as usize);

        Ok(Self {
            path: path_buf,
            indices,
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

        if let Some(cached) = self.cached_info.get(&index) {
            return Ok(cached.clone());
        }

        let offset = self.indices[index].offset as u64;
        self.reader.seek(SeekFrom::Start(offset))?;
        let info = ImageInfo::from_reader(&mut self.reader)?;

        self.cached_info.insert(index, info.clone());
        Ok(info)
    }

    /// 获取图像并解压 RGBA 数据（不修改缓存）
    pub fn get_image_with_data(&mut self, index: usize) -> io::Result<(ImageInfo, Vec<u8>)> {
        if index >= self.indices.len() {
            return Err(io::Error::new(
                io::ErrorKind::InvalidInput,
                format!("Image index {} out of range", index),
            ));
        }

        let offset = self.indices[index].offset as u64;
        self.reader.seek(SeekFrom::Start(offset))?;
        let info = ImageInfo::from_reader(&mut self.reader)?;

        let mut compressed_data = vec![0u8; info.length as usize];
        self.reader.read_exact(&mut compressed_data)?;
        let bgra_data = ImageInfo::decompress_image(&compressed_data, info.width, info.height)?;
        let mut rgba = bgra_data;
        ImageInfo::bgra_to_rgba(&mut rgba);

        Ok((info, rgba))
    }

    /// 获取或加载图像（解压 RGBA 数据并缓存），返回引用
    pub fn get_or_load_image(&mut self, index: usize) -> io::Result<&ImageInfo> {
        if index >= self.indices.len() {
            return Err(io::Error::new(
                io::ErrorKind::InvalidInput,
                format!("图像索引 {} 超出范围 (max: {})", index, self.indices.len()),
            ));
        }

        use std::collections::hash_map::Entry;
        let offset = self.indices[index].offset as u64;

        match self.cached_info.entry(index) {
            Entry::Occupied(mut e) => {
                let cached = e.get_mut();
                if cached.rgba.is_none() {
                    self.reader.seek(SeekFrom::Start(offset + 17))?;
                    cached.load_data(&mut self.reader)?;
                }
            }
            Entry::Vacant(e) => {
                self.reader.seek(SeekFrom::Start(offset))?;
                let mut info = ImageInfo::from_reader(&mut self.reader)?;

                if info.width == 0 || info.height == 0 {
                    return Err(io::Error::new(
                        io::ErrorKind::InvalidData,
                        format!(
                            "图像 {} 无效: width={}, height={}",
                            index, info.width, info.height
                        ),
                    ));
                }

                self.reader.seek(SeekFrom::Start(offset + 17))?;
                info.load_data(&mut self.reader)?;
                e.insert(info);
            }
        }

        Ok(self.cached_info.get_mut(&index).unwrap())
    }

    // ===== 图像属性访问方法 =====

    /// 获取图像偏移量
    pub fn get_offset(&mut self, index: usize) -> io::Result<(i16, i16)> {
        if index >= self.indices.len() {
            return Ok((0, 0));
        }
        let info = self.get_image_info(index)?;
        Ok((info.offset_x, info.offset_y))
    }

    /// 获取图像尺寸
    pub fn get_size(&mut self, index: usize) -> io::Result<(i16, i16)> {
        if index >= self.indices.len() {
            return Ok((0, 0));
        }
        let info = self.get_image_info(index)?;
        Ok((info.width, info.height))
    }

    /// 获取图像实际尺寸（排除透明边缘）
    pub fn get_true_size(&mut self, index: usize) -> io::Result<(i16, i16)> {
        if index >= self.indices.len() {
            return Ok((0, 0));
        }
        let info = self.get_or_load_image(index)?;
        if info.width == 0 || info.height == 0 {
            return Ok((0, 0));
        }
        Ok(info.get_true_size())
    }
}
