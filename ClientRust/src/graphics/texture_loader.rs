// 纹理加载器 - 解析MIR2 .lib文件格式
// 参考: Client/MirGraphics/MLibrary.cs

use std::collections::HashMap;
use std::fs::File;
use std::io::{self, Read, Seek, SeekFrom, BufReader};
use std::path::{Path, PathBuf};
use flate2::read::GzDecoder;
use egui::{ColorImage, TextureHandle, Context as EguiContext};

/// Library trait - generic interface for image libraries
/// 
/// This trait provides a common interface for different types of
/// image libraries (MLibrary, WZL, etc.)
/// 
/// **Note**: This trait is a Rust design improvement over the original C# code.
/// The C# version uses a concrete `MLibrary` sealed class without any interface.
/// This trait-based design allows for:
/// - Better abstraction and extensibility
/// - Support for multiple library formats (e.g., future WZL support)
/// - More idiomatic Rust code with trait bounds
/// 
/// C# equivalent: `MLibrary` class (no interface exists in original)
pub trait Library {
    /// Get the total number of frames in the library
    /// 
    /// C# equivalent: `MLibrary._count` field
    fn frame_count(&self) -> usize;
    
    /// Check if a frame exists at the given index
    /// 
    /// C# equivalent: `MLibrary.CheckImage(int index)`
    fn has_frame(&self, frame_index: i32) -> bool;
    
    /// Get the size of a frame (width, height)
    /// 
    /// Requires mutable access to load image info if not cached.
    /// 
    /// C# equivalent: `MLibrary.GetSize(int index)`
    fn frame_size(&mut self, frame_index: i32) -> Option<(u32, u32)>;
    
    /// Get the offset of a frame (x, y)
    /// 
    /// Requires mutable access to load image info if not cached.
    /// 
    /// C# equivalent: `MLibrary.GetOffSet(int index)`
    fn frame_offset(&mut self, frame_index: i32) -> Option<(i32, i32)>;
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
    
    /// 加载图像为egui ColorImage
    pub fn load_color_image(&mut self, index: usize) -> io::Result<(ImageInfo, ColorImage)> {
        let (info, data) = self.load_image_data(index)?;
        
        let size = [info.width as usize, info.height as usize];
        
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
        
        let color_image = ColorImage::from_rgba_unmultiplied(size, &rgba_data);
        
        Ok((info, color_image))
    }
}

/// 纹理管理器 - 负责加载和缓存所有游戏纹理
pub struct TextureManager {
    libraries: HashMap<String, MLibrary>,
    textures: HashMap<TextureKey, TextureHandle>,
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
    pub fn load_library(&mut self, name: &str, path: &Path) -> io::Result<()> {
        let lib = MLibrary::open(path)?;
        self.libraries.insert(name.to_string(), lib);
        Ok(())
    }
    
    /// 获取或加载纹理
    pub fn get_texture(
        &mut self,
        ctx: &EguiContext,
        library: &str,
        index: usize,
    ) -> io::Result<(ImageInfo, TextureHandle)> {
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
        
        // 加载纹理
        let lib = self.libraries.get_mut(library)
            .ok_or_else(|| io::Error::new(
                io::ErrorKind::NotFound,
                format!("Library '{}' not loaded", library)
            ))?;
        
        let (info, color_image) = lib.load_color_image(index)?;
        
        let texture_name = format!("{}_{}", library, index);
        let handle = ctx.load_texture(texture_name, color_image, Default::default());
        
        self.textures.insert(key, handle.clone());
        
        Ok((info, handle))
    }
    
    /// 获取图像信息(不加载纹理)
    pub fn get_image_info(&mut self, library: &str, index: usize) -> io::Result<ImageInfo> {
        let lib = self.libraries.get_mut(library)
            .ok_or_else(|| io::Error::new(
                io::ErrorKind::NotFound,
                format!("Library '{}' not loaded", library)
            ))?;
        lib.get_image_info(index)
    }
    
    /// 清除所有纹理缓存
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

// ===== Library trait implementation =====

/// Implement Library trait for MLibrary
/// 
/// This allows MLibrary to be used with the generic rendering system.
impl Library for MLibrary {
    fn frame_count(&self) -> usize {
        self.count()
    }

    fn has_frame(&self, frame_index: i32) -> bool {
        frame_index >= 0 && (frame_index as usize) < self.count()
    }

    fn frame_size(&mut self, frame_index: i32) -> Option<(u32, u32)> {
        if !self.has_frame(frame_index) {
            return None;
        }
        
        // Get image info (may load from disk if not cached)
        let info = self.get_image_info(frame_index as usize).ok()?;
        Some((info.width as u32, info.height as u32))
    }

    fn frame_offset(&mut self, frame_index: i32) -> Option<(i32, i32)> {
        if !self.has_frame(frame_index) {
            return None;
        }
        
        // Get image info (may load from disk if not cached)
        let info = self.get_image_info(frame_index as usize).ok()?;
        Some((info.x as i32, info.y as i32))
    }
}

/// Helper methods for MLibrary
impl MLibrary {
    /// Check if image index is valid
    /// 
    /// C# equivalent: MLibrary.CheckImage(int index)
    pub fn check_image(&self, index: i32) -> bool {
        self.has_frame(index)
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
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_library_open() {
        // 这个测试需要实际的.lib文件
        // 暂时跳过
    }
    
    #[test]
    fn test_library_check_image() {
        // Mock test - in practice we'd need a real .lib file
        // For now, we'll test the logic with a hypothetical library
    }
}
