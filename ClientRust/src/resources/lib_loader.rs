// ============================================================================
// MLibrary Data - 纯数据版本的 MIR2 图像库加载器
// ============================================================================
//
// 设计目标：
// 1. 不依赖任何渲染后端（ggez/macroquad）
// 2. 只负责解析 .lib 文件和解压图像数据
// 3. 返回原始 RGBA 数据，由渲染器负责创建纹理
// 4. 支持缓存和延迟加载
//
// ============================================================================

use byteorder::{LittleEndian, ReadBytesExt};
use flate2::read::GzDecoder;
use std::collections::HashMap;
use std::fs::File;
use std::io::{self, BufReader, Read, Seek, SeekFrom};
use std::path::{Path, PathBuf};
use std::time::Instant;

/// MIR2 图像库文件头
#[derive(Debug, Clone)]
pub struct LibraryHeader {
    pub version: i32,
    pub count: i32,
    pub frame_seek: i32,
}

impl LibraryHeader {
    /// 从读取器中读取库头信息（12字节）
    pub fn read_from<R: Read>(reader: &mut R) -> io::Result<Self> {
        let version = reader.read_i32::<LittleEndian>()?;
        let count = reader.read_i32::<LittleEndian>()?;
        let frame_seek = reader.read_i32::<LittleEndian>()?;
        
        Ok(LibraryHeader {
            version,
            count,
            frame_seek,
        })
    }
}

/// 图像索引项
#[derive(Debug, Clone)]
pub struct ImageIndex {
    pub offset: i32,
}

impl ImageIndex {
    /// 从读取器中读取索引（4字节）
    pub fn read_from<R: Read>(reader: &mut R) -> io::Result<Self> {
        let offset = reader.read_i32::<LittleEndian>()?;
        Ok(ImageIndex { offset })
    }
}

/// 图像数据（纯数据，不包含 GPU 纹理）
#[derive(Debug, Clone)]
pub struct ImageData {
    /// 图像尺寸
    pub width: u16,
    pub height: u16,
    
    /// 偏移量（用于定位）
    pub offset_x: i16,
    pub offset_y: i16,
    
    /// 阴影偏移
    pub shadow_x: i16,
    pub shadow_y: i16,
    pub shadow: u8,
    
    /// RGBA 数据（解压后）
    pub rgba_data: Vec<u8>,
    
    /// 遮罩层（可选）
    pub mask: Option<MaskData>,
    
    /// 最后访问时间（用于缓存管理）
    pub last_access_time: Instant,
}

/// 遮罩层数据
#[derive(Debug, Clone)]
pub struct MaskData {
    pub width: u16,
    pub height: u16,
    pub offset_x: i16,
    pub offset_y: i16,
    pub rgba_data: Vec<u8>,
}

/// 图像元数据（不包含实际像素数据）
#[derive(Debug, Clone)]
pub struct ImageMetadata {
    pub width: i16,
    pub height: i16,
    pub offset_x: i16,
    pub offset_y: i16,
    pub shadow_x: i16,
    pub shadow_y: i16,
    pub shadow: u8,
    pub data_length: i32,
    pub has_mask: bool,
    pub mask_width: i16,
    pub mask_height: i16,
    pub mask_offset_x: i16,
    pub mask_offset_y: i16,
    pub mask_length: i32,
}

impl ImageMetadata {
    /// 从读取器中解析元数据（17字节）
    pub fn read_from<R: Read>(reader: &mut R) -> io::Result<Self> {
        let width = reader.read_i16::<LittleEndian>()?;
        let height = reader.read_i16::<LittleEndian>()?;
        let offset_x = reader.read_i16::<LittleEndian>()?;
        let offset_y = reader.read_i16::<LittleEndian>()?;
        let shadow_x = reader.read_i16::<LittleEndian>()?;
        let shadow_y = reader.read_i16::<LittleEndian>()?;
        let shadow = reader.read_u8()?;
        let data_length = reader.read_i32::<LittleEndian>()?;
        
        // 遮罩标志在 shadow 字节的最高位
        let has_mask = (shadow >> 7) == 1;
        
        let (mask_width, mask_height, mask_offset_x, mask_offset_y, mask_length) = if has_mask {
            (
                reader.read_i16::<LittleEndian>()?,
                reader.read_i16::<LittleEndian>()?,
                reader.read_i16::<LittleEndian>()?,
                reader.read_i16::<LittleEndian>()?,
                reader.read_i32::<LittleEndian>()?,
            )
        } else {
            (0, 0, 0, 0, 0)
        };
        
        Ok(ImageMetadata {
            width,
            height,
            offset_x,
            offset_y,
            shadow_x,
            shadow_y,
            shadow,
            data_length,
            has_mask,
            mask_width,
            mask_height,
            mask_offset_x,
            mask_offset_y,
            mask_length,
        })
    }
}

/// MLibrary 数据加载器（纯数据版本）
pub struct MLibraryData {
    /// 库文件路径
    path: PathBuf,
    
    /// 文件头
    header: LibraryHeader,
    
    /// 图像索引
    indices: Vec<ImageIndex>,
    
    /// 图像缓存（索引 -> 图像数据）
    cache: HashMap<usize, ImageData>,
    
    /// 最大缓存大小（图像数量）
    max_cache_size: usize,
}

impl MLibraryData {
    /// 创建新的库加载器
    pub fn new() -> Self {
        Self {
            path: PathBuf::new(),
            header: LibraryHeader {
                version: 0,
                count: 0,
                frame_seek: 0,
            },
            indices: Vec::new(),
            cache: HashMap::new(),
            max_cache_size: 1000, // 默认缓存 1000 张图片
        }
    }
    
    /// 设置最大缓存大小
    pub fn set_max_cache_size(&mut self, size: usize) {
        self.max_cache_size = size;
    }
    
    /// 加载库文件
    pub fn load<P: AsRef<Path>>(&mut self, path: P) -> io::Result<()> {
        let path = path.as_ref();
        self.path = path.to_path_buf();
        
        let file = File::open(path)?;
        let mut reader = BufReader::new(file);
        
        // 读取文件头
        self.header = LibraryHeader::read_from(&mut reader)?;
        
        tracing::info!(
            "📚 加载库文件: {:?}, 版本: {}, 图像数量: {}",
            path.file_name().unwrap_or_default(),
            self.header.version,
            self.header.count
        );
        
        // 读取所有索引
        self.indices = Vec::with_capacity(self.header.count as usize);
        for _ in 0..self.header.count {
            let index = ImageIndex::read_from(&mut reader)?;
            self.indices.push(index);
        }
        
        Ok(())
    }
    
    /// 获取图像数量
    pub fn count(&self) -> usize {
        self.header.count as usize
    }
    
    /// 检查索引是否有效
    pub fn is_valid_index(&self, index: usize) -> bool {
        index < self.indices.len() && self.indices[index].offset > 0
    }
    
    /// 获取图像数据（带缓存）
    pub fn get_image(&mut self, index: usize) -> io::Result<Option<&ImageData>> {
        // 检查索引有效性
        if !self.is_valid_index(index) {
            return Ok(None);
        }
        
        // 检查缓存
        if self.cache.contains_key(&index) {
            // 更新访问时间
            if let Some(img) = self.cache.get_mut(&index) {
                img.last_access_time = Instant::now();
            }
            return Ok(self.cache.get(&index));
        }
        
        // 加载图像数据
        let image_data = self.load_image_from_file(index)?;
        
        // 缓存管理：如果缓存已满，删除最旧的
        if self.cache.len() >= self.max_cache_size {
            self.evict_oldest();
        }
        
        self.cache.insert(index, image_data);
        Ok(self.cache.get(&index))
    }
    
    /// 从文件加载图像数据
    fn load_image_from_file(&self, index: usize) -> io::Result<ImageData> {
        let offset = self.indices[index].offset;
        let file = File::open(&self.path)
            .map_err(|e| io::Error::new(
                e.kind(),
                format!("无法打开库文件: {:?} - {}", self.path.display(), e)
            ))?;
        let mut reader = BufReader::new(file);
        
        // 定位到图像数据位置
        reader.seek(SeekFrom::Start(offset as u64))?;
        
        // 读取元数据
        let metadata = ImageMetadata::read_from(&mut reader)?;
        
        // 检查图像是否有效（宽度和高度不为0）
        // 零尺寸图像是库文件的占位符,静默跳过,不输出错误
        if metadata.width == 0 || metadata.height == 0 || metadata.data_length == 0 {
            #[cfg(debug_assertions)]
            log::debug!(
                "跳过占位符图像 - File: {:?}, Index: {}, Width: {}, Height: {}, DataLength: {}",
                self.path.display(),
                index,
                metadata.width,
                metadata.height,
                metadata.data_length
            );
            return Err(io::Error::new(
                io::ErrorKind::InvalidData,
                "占位符图像" // 简化错误消息,不包含详细信息
            ));
        }
        
        // 重新定位到图像数据开始位置(跳过元数据)
        // 基础元数据: 17字节 (width, height, offset_x, offset_y, shadow_x, shadow_y, shadow, data_length)
        // 如果有mask,再加12字节 (mask_width, mask_height, mask_offset_x, mask_offset_y, mask_length)
        let metadata_size = if metadata.has_mask { 17 + 12 } else { 17 };
        reader.seek(SeekFrom::Start((offset + metadata_size) as u64))?;
        
        // 解压主图像数据
        let rgba_data = Self::decompress_image_data(
            &mut reader,
            metadata.width as u16,
            metadata.height as u16,
            metadata.data_length as usize,
        )?;
        
        // 解压遮罩数据（如果有）
        let mask = if metadata.has_mask {
            let mask_rgba = Self::decompress_image_data(
                &mut reader,
                metadata.mask_width as u16,
                metadata.mask_height as u16,
                metadata.mask_length as usize,
            )?;
            
            Some(MaskData {
                width: metadata.mask_width as u16,
                height: metadata.mask_height as u16,
                offset_x: metadata.mask_offset_x,
                offset_y: metadata.mask_offset_y,
                rgba_data: mask_rgba,
            })
        } else {
            None
        };
        
        Ok(ImageData {
            width: metadata.width as u16,
            height: metadata.height as u16,
            offset_x: metadata.offset_x,
            offset_y: metadata.offset_y,
            shadow_x: metadata.shadow_x,
            shadow_y: metadata.shadow_y,
            shadow: metadata.shadow,
            rgba_data,
            mask,
            last_access_time: Instant::now(),
        })
    }
    
    /// 解压图像数据（从 555 BGR 转换为 RGBA）
    fn decompress_image_data<R: Read>(
        reader: &mut R,
        width: u16,
        height: u16,
        compressed_length: usize,
    ) -> io::Result<Vec<u8>> {
        // 读取压缩数据
        let mut compressed_data = vec![0u8; compressed_length];
        let bytes_read = reader.read(&mut compressed_data)?;
        
        if bytes_read != compressed_length {
            eprintln!("⚠️ 压缩数据读取不完整: 期望={} 实际={}", compressed_length, bytes_read);
            compressed_data.truncate(bytes_read);
        }
        
        // 解压
        let mut decoder = GzDecoder::new(&compressed_data[..]);
        let mut decompressed = Vec::new();
        match decoder.read_to_end(&mut decompressed) {
            Ok(_) => {},
            Err(e) => {
                eprintln!("⚠️ 解压失败: width={} height={} compressed_len={} error={:?}", 
                    width, height, compressed_length, e);
                return Err(e);
            }
        }
        
        // 解压后的数据是BGRA8888格式,需要转换为RGBA8888
        // 验证数据大小
        let expected_size = (width as usize) * (height as usize) * 4;
        if decompressed.len() != expected_size {
            eprintln!("⚠️ 解压后数据大小不匹配: 期望={} 实际={}", expected_size, decompressed.len());
            if decompressed.len() < expected_size {
                decompressed.resize(expected_size, 0);
            } else {
                decompressed.truncate(expected_size);
            }
        }
        
        // 转换 BGRA 到 RGBA (交换 R 和 B 通道)
        let mut rgba_data = decompressed;
        for chunk in rgba_data.chunks_exact_mut(4) {
            // BGRA: [B, G, R, A] -> RGBA: [R, G, B, A]
            chunk.swap(0, 2); // 交换 B 和 R
            
            // 黑色背景透明化 (匹配 ggez 版本逻辑)
            let r = chunk[0];
            let g = chunk[1];
            let b = chunk[2];
            let a = chunk[3];
            
            // 检查是否为接近黑色且不透明的像素
            // 这些像素通常是背景色,需要转为透明
            let is_near_black = r < 3 && g < 3 && b < 3;
            let is_opaque = a > 250;
            
            // 将接近黑色且不透明的背景色转为透明
            if is_near_black && is_opaque {
                chunk[3] = 0; // 设置为透明
            }
        }
        
        Ok(rgba_data)
    }
    
    /// 清除最旧的缓存项
    fn evict_oldest(&mut self) {
        if let Some((&oldest_index, _)) = self
            .cache
            .iter()
            .min_by_key(|(_, img)| img.last_access_time)
        {
            self.cache.remove(&oldest_index);
        }
    }
    
    /// 清空缓存
    pub fn clear_cache(&mut self) {
        self.cache.clear();
    }
    
    /// 预加载一批图像
    pub fn preload_range(&mut self, start: usize, end: usize) -> io::Result<usize> {
        let mut loaded = 0;
        for i in start..end.min(self.count()) {
            if self.get_image(i)?.is_some() {
                loaded += 1;
            }
        }
        Ok(loaded)
    }
}

impl Default for MLibraryData {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_library_creation() {
        let lib = MLibraryData::new();
        assert_eq!(lib.count(), 0);
    }
    
    #[test]
    fn test_cache_management() {
        let mut lib = MLibraryData::new();
        lib.set_max_cache_size(10);
        assert_eq!(lib.max_cache_size, 10);
    }
}
