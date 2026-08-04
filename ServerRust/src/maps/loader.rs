// 地图加载器 - 解析 .map 文件
// 支持 Type 100 (C# custom) + Type 0 (old format)

use std::fs;
use std::io::{self, Cursor};
use std::path::{Path, PathBuf};

use byteorder::{LittleEndian, ReadBytesExt};

/// 地图单元信息（仅服务端需要的字段）
#[derive(Debug, Clone)]
pub struct CellInfo {
    pub back_image: i32,
    pub walkable: bool,
}

/// 安全区矩形 (x1, y1, x2, y2)
pub type SafeZoneRect = (i32, i32, i32, i32);

/// 加载后的地图数据
#[derive(Debug, Clone)]
pub struct MapData {
    pub file_name: String,
    pub title: String,
    pub width: i16,
    pub height: i16,
    pub cells: Vec<Vec<CellInfo>>,
    /// 安全区矩形列表（左闭右闭）
    pub safe_zone_rects: Vec<SafeZoneRect>,
}

impl MapData {
    /// 检查坐标是否在地图范围内
    pub fn is_valid(&self, x: i32, y: i32) -> bool {
        x >= 0 && y >= 0 && (x as i16) < self.width && (y as i16) < self.height
    }

    /// 检查指定格子是否可行走
    pub fn is_walkable(&self, x: i32, y: i32) -> bool {
        if !self.is_valid(x, y) {
            return false;
        }
        self.cells[x as usize][y as usize].walkable
    }

    /// 检查指定坐标是否在安全区内
    pub fn is_safe_zone(&self, x: i32, y: i32) -> bool {
        if !self.is_valid(x, y) {
            return false;
        }
        for (x1, y1, x2, y2) in &self.safe_zone_rects {
            if x >= *x1 && x <= *x2 && y >= *y1 && y <= *y2 {
                return true;
            }
        }
        false
    }
}

/// 从文件系统加载地图
pub fn load_map(file_name: &str, data_dir: &Path) -> io::Result<MapData> {
    let path = resolve_map_path(file_name, data_dir)?;
    let bytes = fs::read(&path)?;
    parse_map_bytes(&bytes, file_name)
}

/// 尝试解析多种格式的 .map 文件
pub fn parse_map_bytes(bytes: &[u8], file_name: &str) -> io::Result<MapData> {
    if bytes.is_empty() {
        return Err(io::Error::new(io::ErrorKind::UnexpectedEof, "empty map file"));
    }

    // 检测格式类型
    let format = detect_format(bytes)?;

    match format {
        MapFormat::Type100 => load_type_100(bytes, file_name),
        MapFormat::Type0 => load_type_0(bytes, file_name),
        MapFormat::Type1 => load_type_1(bytes, file_name),
    }
}

#[derive(Debug)]
enum MapFormat {
    Type100,
    Type0,
    Type1,
}

/// 检测地图格式
fn detect_format(bytes: &[u8]) -> io::Result<MapFormat> {
    if bytes.len() < 4 {
        return Err(io::Error::new(io::ErrorKind::UnexpectedEof, "file too short"));
    }

    // Type 100: magic "C#" at offset 2-3
    if bytes.len() >= 24 && bytes[2] == 0x43 && bytes[3] == 0x23 {
        return Ok(MapFormat::Type100);
    }

    // Map 2010 Ver 1.0（Daneo1989 服务器地图）：0x10 "Map 2010 Ver 1.0"，14 bytes/cell
    // 之前被误判为 Type 0（12 bytes/cell）→ 尺寸/单元/障碍位全错（#57 实测）
    if bytes.len() >= 20 && bytes[0] == 0x10 && bytes[2] == 0x61 && bytes[7] == 0x31 && bytes[14] == 0x31 {
        return Ok(MapFormat::Type1);
    }

    // Type 0: fallback
    Ok(MapFormat::Type0)
}

/// 解析 Type 100 格式 (C# custom, 26 bytes/cell)
#[allow(clippy::needless_range_loop)]
fn load_type_100(bytes: &[u8], file_name: &str) -> io::Result<MapData> {
    if bytes.len() < 8 {
        return Err(io::Error::new(io::ErrorKind::UnexpectedEof, "Type 100 header too short"));
    }

    let width = i16::from_le_bytes([bytes[4], bytes[5]]);
    let height = i16::from_le_bytes([bytes[6], bytes[7]]);

    if width <= 0 || height <= 0 {
        return Err(io::Error::new(io::ErrorKind::InvalidData, "invalid map dimensions"));
    }

    let cell_size: usize = 26;
    let total_cells = (width as usize) * (height as usize);
    let data_size = total_cells * cell_size;
    let header_size = 8;

    if bytes.len() < header_size + data_size {
        return Err(io::Error::new(
            io::ErrorKind::UnexpectedEof,
            "Type 100 file truncated",
        ));
    }

    let mut cells = vec![
        vec![
            CellInfo {
                back_image: 0,
                walkable: true,
            };
            height as usize
        ];
        width as usize
    ];

    let data_start = header_size;
    const OBSTACLE_BIT: i32 = 0x2000_0000;

    for y in 0..height as usize {
        for x in 0..width as usize {
            let offset = data_start + (y * width as usize + x) * cell_size;
            let cell_bytes = &bytes[offset..offset + cell_size];

            let back_image = i32::from_le_bytes([
                cell_bytes[2],
                cell_bytes[3],
                cell_bytes[4],
                cell_bytes[5],
            ]);

            let walkable = (back_image & OBSTACLE_BIT) == 0;

            cells[x][y] = CellInfo {
                back_image,
                walkable,
            };
        }
    }

    // 从文件名提取标题（去掉扩展名）
    let title = Path::new(file_name)
        .file_stem()
        .map(|s| s.to_string_lossy().to_string())
        .unwrap_or_else(|| file_name.to_string());

    Ok(MapData {
        file_name: file_name.to_string(),
        title,
        width,
        height,
        cells,
        safe_zone_rects: Vec::new(),
    })
}

/// 解析 Map 2010 Ver 1.0 格式（14 bytes/cell，尺寸 XOR 加密）
/// 对齐 Client-Bevy map_reader load_map_type_1（参考 C# MapCode.cs）
#[allow(clippy::needless_range_loop)]
fn load_type_1(bytes: &[u8], file_name: &str) -> io::Result<MapData> {
    if bytes.len() < 54 {
        return Err(io::Error::new(io::ErrorKind::UnexpectedEof, "Type 1 header too short"));
    }

    let mut offset = 21usize;
    let w = i16::from_le_bytes([bytes[offset], bytes[offset + 1]]);
    offset += 2;
    let xor = i16::from_le_bytes([bytes[offset], bytes[offset + 1]]);
    offset += 2;
    let h = i16::from_le_bytes([bytes[offset], bytes[offset + 1]]);
    let width = (w ^ xor) as i32;
    let height = (h ^ xor) as i32;
    offset = 54;

    if width <= 0 || height <= 0 || width > 10000 || height > 10000 {
        return Err(io::Error::new(
            io::ErrorKind::InvalidData,
            format!("Type 1 invalid map dimensions: {}x{}", width, height),
        ));
    }

    let cell_size: usize = 14;
    let total_cells = (width as usize) * (height as usize);
    if bytes.len() < offset + total_cells * cell_size {
        return Err(io::Error::new(io::ErrorKind::UnexpectedEof, "Type 1 file truncated"));
    }

    let mut cells = vec![
        vec![
            CellInfo {
                back_image: 0,
                walkable: true,
            };
            height as usize
        ];
        width as usize
    ];

    const OBSTACLE_BIT: i32 = 0x2000_0000;
    const BACK_XOR: i32 = 0xAA38_AA38u32 as i32;

    for x in 0..width as usize {
        for y in 0..height as usize {
            let o = offset + (x * height as usize + y) * cell_size;
            let back_raw = i32::from_le_bytes([
                bytes[o],
                bytes[o + 1],
                bytes[o + 2],
                bytes[o + 3],
            ]);
            let back_image = back_raw ^ BACK_XOR;
            let walkable = (back_image & OBSTACLE_BIT) == 0;
            cells[x][y] = CellInfo {
                back_image,
                walkable,
            };
        }
    }

    let title = Path::new(file_name)
        .file_stem()
        .map(|s| s.to_string_lossy().to_string())
        .unwrap_or_else(|| file_name.to_string());

    Ok(MapData {
        file_name: file_name.to_string(),
        title,
        width: width as i16,
        height: height as i16,
        cells,
        safe_zone_rects: Vec::new(),
    })
}

/// 解析 Type 0 格式 (old format, 12 bytes/cell, 52-byte header)
#[allow(clippy::needless_range_loop)]
fn load_type_0(bytes: &[u8], file_name: &str) -> io::Result<MapData> {
    if bytes.len() < 52 {
        return Err(io::Error::new(io::ErrorKind::UnexpectedEof, "Type 0 header too short"));
    }

    let mut cursor = Cursor::new(&bytes[0..4]);
    let width = cursor.read_i16::<LittleEndian>()?;
    let height = cursor.read_i16::<LittleEndian>()?;

    if width <= 0 || height <= 0 {
        return Err(io::Error::new(io::ErrorKind::InvalidData, "invalid map dimensions"));
    }

    let cell_size: usize = 12;
    let total_cells = (width as usize) * (height as usize);
    let data_size = total_cells * cell_size;
    let header_size = 52;

    if bytes.len() < header_size + data_size {
        return Err(io::Error::new(
            io::ErrorKind::UnexpectedEof,
            "Type 0 file truncated",
        ));
    }

    let mut cells = vec![
        vec![
            CellInfo {
                back_image: 0,
                walkable: true,
            };
            height as usize
        ];
        width as usize
    ];

    let data_start = header_size;
    const OBSTACLE_BIT: i32 = 0x2000_0000;

    for y in 0..height as usize {
        for x in 0..width as usize {
            let offset = data_start + (y * width as usize + x) * cell_size;
            let cell_bytes = &bytes[offset..offset + cell_size];

            // Type 0: back_image is i16, obstacle flag in bit 15
            let back_raw = i16::from_le_bytes([cell_bytes[0], cell_bytes[1]]);
            let back_image = if (back_raw as u16 & 0x8000) != 0 {
                // Convert: keep lower 15 bits, set bit 29
                ((back_raw & 0x7FFF) as i32) | OBSTACLE_BIT
            } else {
                back_raw as i32
            };

            let walkable = (back_image & OBSTACLE_BIT) == 0;

            cells[x][y] = CellInfo {
                back_image,
                walkable,
            };
        }
    }

    let title = Path::new(file_name)
        .file_stem()
        .map(|s| s.to_string_lossy().to_string())
        .unwrap_or_else(|| file_name.to_string());

    Ok(MapData {
        file_name: file_name.to_string(),
        title,
        width,
        height,
        cells,
        safe_zone_rects: Vec::new(),
    })
}

/// 查找地图文件路径
fn resolve_map_path(file_name: &str, data_dir: &Path) -> io::Result<PathBuf> {
    // Try direct path
    let direct = Path::new(file_name);
    if direct.exists() {
        return Ok(direct.to_path_buf());
    }

    // Try {data_dir}/Map/{file_name}.map
    let mut path = data_dir.join("Map");
    if !file_name.ends_with(".map") {
        path.push(format!("{}.map", file_name));
    } else {
        path.push(file_name);
    }
    if path.exists() {
        return Ok(path);
    }

    // Try {data_dir}/{file_name}.map
    let mut path2 = data_dir.to_path_buf();
    if !file_name.ends_with(".map") {
        path2.push(format!("{}.map", file_name));
    } else {
        path2.push(file_name);
    }
    if path2.exists() {
        return Ok(path2);
    }

    Err(io::Error::new(
        io::ErrorKind::NotFound,
        format!("map file not found: {}", file_name),
    ))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_type_100_map_parsing() {
        // Type 100 header: [version, padding, 0x43, 0x23, width_le, height_le]
        // 5x5 map = 25 cells * 26 bytes = 650 bytes of cell data + 8 bytes header = 658 total
        let mut bytes = vec![0x01, 0x00, 0x43, 0x23, 0x05, 0x00, 0x05, 0x00];
        // Fill cells with walkable back_image (0)
        for _ in 0..(5 * 5 * 26) {
            bytes.push(0u8);
        }

        let map = parse_map_bytes(&bytes, "test.map").unwrap();
        assert_eq!(map.width, 5);
        assert_eq!(map.height, 5);
        assert!(map.is_walkable(0, 0));
        assert!(map.is_walkable(4, 4));
        assert!(!map.is_walkable(5, 5)); // out of bounds
    }

    #[test]
    fn test_type_100_obstacle_detection() {
        let mut bytes = vec![0x01, 0x00, 0x43, 0x23, 0x02, 0x00, 0x01, 0x00];
        // Cell 0,0: walkable (back_image=0, starts at offset 2 in cell)
        bytes.push(0); bytes.push(0); // back_index
        bytes.extend_from_slice(&0i32.to_le_bytes()); // back_image
        for _ in 0..20 { bytes.push(0u8); } // rest of cell (20 bytes to total 26)
        // Cell 1,0: obstacle (back_image has bit 0x20000000)
        bytes.push(0); bytes.push(0); // back_index
        bytes.extend_from_slice(&0x20000000i32.to_le_bytes()); // back_image
        for _ in 0..20 { bytes.push(0u8); }

        let map = parse_map_bytes(&bytes, "test.map").unwrap();
        assert_eq!(map.width, 2);
        assert_eq!(map.height, 1);
        assert!(map.is_walkable(0, 0));
        assert!(!map.is_walkable(1, 0));
    }

    #[test]
    fn test_type_0_map_parsing() {
        // Type 0: 52-byte header + 12 bytes/cell
        // 3x2 map = 6 cells * 12 = 72 + 52 = 124 bytes
        let mut bytes = vec![0u8; 52];
        bytes[0] = 0x03; bytes[1] = 0x00; // width=3
        bytes[2] = 0x02; bytes[3] = 0x00; // height=2
        // All cells walkable
        for _ in 0..(3 * 2 * 12) {
            bytes.push(0u8);
        }

        let map = parse_map_bytes(&bytes, "test.map").unwrap();
        assert_eq!(map.width, 3);
        assert_eq!(map.height, 2);
        assert!(map.is_walkable(0, 0));
        assert!(map.is_walkable(2, 1));
    }

    #[test]
    fn test_empty_map_error() {
        let result = parse_map_bytes(&[], "empty.map");
        assert!(result.is_err());
    }

    #[test]
    fn test_too_small_file_error() {
        let result = parse_map_bytes(&[0x01], "small.map");
        assert!(result.is_err());
    }
}
