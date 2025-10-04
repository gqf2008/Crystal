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

// 移除 egui 依赖，使用自己的类型
use super::dx_manager::TextureHandle;

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
    pub fn get_texture(
        &mut self,
        dx_manager: &super::dx_manager::DXManager,
        library: &str,
        index: usize,
    ) -> io::Result<(ImageInfo, Arc<TextureHandle>)> {
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
        
        // 上传到 GPU
        let texture_name = format!("{}_{}", library, index);
        let handle = dx_manager.load_texture(
            texture_name,
            info.width as u32,
            info.height as u32,
            &rgba_data,
        );
        
        self.textures.insert(key, handle.clone());
        
        Ok((info, handle))
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

/// Helper methods for MLibrary
impl MLibrary {
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
}

#[cfg(test)]
mod tests {
    // Tests require actual .lib files
    // TODO: Add integration tests with sample data
}
