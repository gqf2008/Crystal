// Client/MirObjects/MapCode.cs 的 Rust 移植
// 包含 CellInfo 和 MapReader 两个类

use std::collections::HashMap;
use std::fs::File;
use std::io::{self, Read};
use std::path::Path;

use ggez::{Context, GameResult};
use ggez::graphics::Canvas;
use mir2_shared::Point;

use super::drawable::DrawableMapObject;

// ============================================================================
// CellInfo - 对应 C# 的 CellInfo 类
// ============================================================================

#[derive(Debug, Clone)]
pub struct CellInfo {
    // 背景层
    pub back_index: i16,
    pub back_image: i32,
    
    // 中间层
    pub middle_index: i16,
    pub middle_image: i32,
    
    // 前景层
    pub front_index: i16,
    pub front_image: i32,
    
    // 门相关
    pub door_index: u8,
    pub door_offset: u8,
    
    // 动画相关
    pub front_animation_frame: u8,
    pub front_animation_tick: u8,
    pub middle_animation_frame: u8,
    pub middle_animation_tick: u8,
    
    pub tile_animation_image: i16,
    pub tile_animation_offset: i16,
    pub tile_animation_frames: u8,
    
    // 光照
    pub light: u8,
    pub unknown: u8,
    
    // 对象列表 (对应 C# 的 List<MapObject> CellObjects)
    pub cell_objects: Option<Vec<u32>>, // 存储 ObjectID，实际对象在 GameScene 中管理
    
    // 钓鱼点
    pub fishing_cell: bool,
}

impl CellInfo {
    pub fn new() -> Self {
        Self {
            back_index: 0,
            back_image: 0,
            middle_index: 1,
            middle_image: 0,
            front_index: 0,
            front_image: 0,
            door_index: 0,
            door_offset: 0,
            front_animation_frame: 0,
            front_animation_tick: 0,
            middle_animation_frame: 0,
            middle_animation_tick: 0,
            tile_animation_image: 0,
            tile_animation_offset: 0,
            tile_animation_frames: 0,
            light: 0,
            unknown: 0,
            cell_objects: None,
            fishing_cell: false,
        }
    }
    
    // 对应 C# 的 AddObject
    pub fn add_object(&mut self, object_id: u32) {
        if self.cell_objects.is_none() {
            self.cell_objects = Some(Vec::new());
        }
        
        if let Some(ref mut objects) = self.cell_objects {
            objects.insert(0, object_id);
            self.sort();
        }
    }
    
    // 对应 C# 的 RemoveObject
    pub fn remove_object(&mut self, object_id: u32) {
        if let Some(ref mut objects) = self.cell_objects {
            objects.retain(|&id| id != object_id);
            
            if objects.is_empty() {
                self.cell_objects = None;
            } else {
                self.sort();
            }
        }
    }
    
    // 对应 C# 的 FindObject
    pub fn find_object(&self, object_id: u32) -> bool {
        if let Some(ref objects) = self.cell_objects {
            objects.contains(&object_id)
        } else {
            false
        }
    }
    
    // 对应 C# 的 Sort (简化版，实际排序在 GameScene 中进行)
    fn sort(&mut self) {
        // TODO: 实现对象排序逻辑
        // 注意：C# 中的排序逻辑比较复杂，涉及对象类型、死亡状态等
        // 暂时保留简单版本，后续在 GameScene 中实现完整排序
    }
    
    /// Draw all live objects in this cell
    /// C# Reference: Client/MirObjects/MapCode.cs lines 55-82
    pub fn draw_objects(
        &self,
        ctx: &mut Context,
        canvas: &mut Canvas,
        objects_map: &HashMap<u32, Box<dyn DrawableMapObject>>,
        draw_location: Point,
    ) -> GameResult {
        if let Some(ref cell_objects) = self.cell_objects {
            for &object_id in cell_objects.iter() {
                if let Some(obj) = objects_map.get(&object_id) {
                    if !obj.is_dead() && !obj.is_hidden() {
                        obj.draw(ctx, canvas, draw_location)?;
                    }
                }
            }
        }
        Ok(())
    }
    
    /// Draw all dead objects in this cell (corpses, etc.)
    /// C# Reference: Client/MirObjects/MapCode.cs lines 85-113
    pub fn draw_dead_objects(
        &self,
        ctx: &mut Context,
        canvas: &mut Canvas,
        objects_map: &HashMap<u32, Box<dyn DrawableMapObject>>,
        draw_location: Point,
    ) -> GameResult {
        if let Some(ref cell_objects) = self.cell_objects {
            for &object_id in cell_objects.iter() {
                if let Some(obj) = objects_map.get(&object_id) {
                    if obj.is_dead() && !obj.is_hidden() {
                        // TODO: Add special handling for dead monsters (walls, HellLord, etc.)
                        // C# checks for ((MonsterObject)CellObjects[i]).EternalStatue
                        obj.draw(ctx, canvas, draw_location)?;
                    }
                }
            }
        }
        Ok(())
    }
    
    // 辅助方法：检查是否可行走
    pub fn is_walkable(&self) -> bool {
        // 简化版本，后续根据 C# 逻辑完善
        self.door_index == 0 && self.front_image == 0
    }
}

// ============================================================================
// MapReader - 对应 C# 的 MapReader 类
// ============================================================================

pub struct MapReader {
    pub width: i32,
    pub height: i32,
    pub map_cells: Vec<Vec<CellInfo>>, // 使用 Vec<Vec> 而非 C# 的二维数组
    pub file_name: String,
    bytes: Vec<u8>,
}

impl MapReader {
    // 对应 C# 的构造函数
    pub fn new(file_name: &str) -> io::Result<Self> {
        let mut reader = Self {
            width: 0,
            height: 0,
            map_cells: Vec::new(),
            file_name: file_name.to_string(),
            bytes: Vec::new(),
        };
        
        reader.initiate()?;
        Ok(reader)
    }
    
    // 对应 C# 的 initiate() 方法
    fn initiate(&mut self) -> io::Result<()> {
        if Path::new(&self.file_name).exists() {
            let mut file = File::open(&self.file_name)?;
            file.read_to_end(&mut self.bytes)?;
        } else {
            // 文件不存在时创建空地图
            self.width = 1000;
            self.height = 1000;
            self.map_cells = vec![vec![CellInfo::new(); self.height as usize]; self.width as usize];
            return Ok(());
        }
        
        // 检测地图格式
        self.detect_and_load()?;
        Ok(())
    }
    
    // 地图格式检测 (对应 C# 中 initiate() 的格式判断逻辑)
    fn detect_and_load(&mut self) -> io::Result<()> {
        let bytes = &self.bytes;
        
        if bytes.len() < 20 {
            return Err(io::Error::new(
                io::ErrorKind::InvalidData,
                "Map file too small",
            ));
        }
        
        // C# 自定义格式 (C#)
        if bytes[2] == 0x43 && bytes[3] == 0x23 {
            return self.load_map_type_100();
        }
        
        // Wemade Mir3 maps (starts with 0)
        if bytes[0] == 0 {
            return self.load_map_type_5();
        }
        
        // Shanda Mir3 maps: (C) SNDA, MIR3.
        if bytes[0] == 0x0F && bytes[5] == 0x53 && bytes[14] == 0x33 {
            return self.load_map_type_6();
        }
        
        // Wemade AntiHack map: Mir2 AntiHack
        if bytes[0] == 0x15 && bytes[4] == 0x32 && bytes[6] == 0x41 && bytes[19] == 0x31 {
            return self.load_map_type_4();
        }
        
        // Map 2010 Ver 1.0
        if bytes[0] == 0x10 && bytes[2] == 0x61 && bytes[7] == 0x31 && bytes[14] == 0x31 {
            return self.load_map_type_1();
        }
        
        // Shanda 2012 format
        if (bytes[4] == 0x0F || bytes[4] == 0x03) && bytes[18] == 0x0D && bytes[19] == 0x0A {
            let w = bytes[0] as usize + ((bytes[1] as usize) << 8);
            let h = bytes[2] as usize + ((bytes[3] as usize) << 8);
            
            if bytes.len() > (52 + (w * h * 14)) {
                return self.load_map_type_3();
            } else {
                return self.load_map_type_2();
            }
        }
        
        // 3/4 Heroes map format
        if bytes[0] == 0x0D && bytes[1] == 0x4C && bytes[7] == 0x20 && bytes[11] == 0x6D {
            return self.load_map_type_7();
        }
        
        // 默认格式
        self.load_map_type_0()
    }
    
    // ========================================================================
    // Map Type 0 - 老格式 (12 bytes per cell)
    // C# Reference: Client/MirObjects/MapCode.cs lines 224-268
    // ========================================================================
    fn load_map_type_0(&mut self) -> io::Result<()> {
        tracing::info!("🗺️  加载地图格式: Type 0 (老格式, 12 bytes/cell)");
        
        let bytes = &self.bytes;
        let mut offset = 0;
        
        self.width = i16::from_le_bytes([bytes[offset], bytes[offset + 1]]) as i32;
        offset += 2;
        self.height = i16::from_le_bytes([bytes[offset], bytes[offset + 1]]) as i32;
        offset = 52; // 跳过头部
        
        tracing::info!("📐 地图尺寸: {}x{}", self.width, self.height);
        
        // C#: MapCells = new CellInfo[Width, Height]
        // Rust: map_cells[x][y] where outer vec has Width elements, inner has Height
        self.map_cells = vec![vec![CellInfo::new(); self.height as usize]; self.width as usize];
        
        for x in 0..self.width as usize {
            for y in 0..self.height as usize {
                let cell = &mut self.map_cells[x][y];
                
                cell.back_index = 0;
                cell.middle_index = 1;
                
                cell.back_image = i16::from_le_bytes([bytes[offset], bytes[offset + 1]]) as i32;
                offset += 2;
                
                cell.middle_image = i16::from_le_bytes([bytes[offset], bytes[offset + 1]]) as i32;
                offset += 2;
                
                cell.front_image = i16::from_le_bytes([bytes[offset], bytes[offset + 1]]) as i32;
                offset += 2;
                
                cell.door_index = bytes[offset] & 0x7F;
                offset += 1;
                
                cell.door_offset = bytes[offset];
                offset += 1;
                
                cell.front_animation_frame = bytes[offset];
                offset += 1;
                
                cell.front_animation_tick = bytes[offset];
                offset += 1;
                
                // C# reads FrontIndex here
                cell.front_index = (bytes[offset] as i16) + 2;
                offset += 1;
                
                cell.light = bytes[offset];
                offset += 1;
                
                // C#: BackImage flag processing
                if (cell.back_image & 0x8000) != 0 {
                    cell.back_image = (cell.back_image & 0x7FFF) | 0x20000000;
                }
                
                // C#: Fishing cell detection
                if cell.light >= 100 && cell.light <= 119 {
                    cell.fishing_cell = true;
                }
            }
        }
        
        Ok(())
    }
    
    // ========================================================================
    // Map Type 1 - Map 2010 Ver 1.0 (14 bytes per cell)
    // ========================================================================
    fn load_map_type_1(&mut self) -> io::Result<()> {
        tracing::info!("🗺️  加载地图格式: Type 1 (Map 2010 Ver 1.0, 14 bytes/cell)");
        
        let bytes = &self.bytes;
        let mut offset = 21;
        
        // Type1 uses XOR encryption for dimensions
        let w = i16::from_le_bytes([bytes[offset], bytes[offset + 1]]);
        offset += 2;
        let xor = i16::from_le_bytes([bytes[offset], bytes[offset + 1]]);
        offset += 2;
        let h = i16::from_le_bytes([bytes[offset], bytes[offset + 1]]);
        
        // Decrypt dimensions
        self.width = (w ^ xor) as i32;
        self.height = (h ^ xor) as i32;
        offset = 54;
        
        // Validate dimensions to prevent capacity overflow
        if self.width <= 0 || self.height <= 0 || self.width > 10000 || self.height > 10000 {
            return Err(std::io::Error::new(
                std::io::ErrorKind::InvalidData,
                format!("Invalid map dimensions: {}x{}", self.width, self.height),
            ));
        }
        
        tracing::debug!("📐 Map dimensions: {}x{} (XOR key: 0x{:04X})", self.width, self.height, xor);
        self.map_cells = vec![vec![CellInfo::new(); self.height as usize]; self.width as usize];
        
        for x in 0..self.width as usize {
            for y in 0..self.height as usize {
                let cell = &mut self.map_cells[x][y];
                
                cell.back_index = 0;
                cell.middle_index = 1;
                
                // BackImage: 4 bytes XOR 0xAA38AA38
                let back_raw = i32::from_le_bytes([
                    bytes[offset],
                    bytes[offset + 1],
                    bytes[offset + 2],
                    bytes[offset + 3],
                ]);
                cell.back_image = back_raw ^ 0xAA38AA38u32 as i32;
                offset += 4;
                
                // MiddleImage: 2 bytes XOR xor
                let middle_raw = i16::from_le_bytes([bytes[offset], bytes[offset + 1]]);
                cell.middle_image = (middle_raw ^ xor) as i32;
                offset += 2;
                
                // FrontImage: 2 bytes XOR xor
                let front_raw = i16::from_le_bytes([bytes[offset], bytes[offset + 1]]);
                cell.front_image = (front_raw ^ xor) as i32;
                offset += 2;
                
                cell.door_index = bytes[offset] & 0x7F;
                offset += 1;
                
                cell.door_offset = bytes[offset];
                offset += 1;
                
                cell.front_animation_frame = bytes[offset];
                offset += 1;
                
                cell.front_animation_tick = bytes[offset];
                offset += 1;
                
                // FrontIndex (C# code: Bytes[++offSet] + 2)
                let front_idx = bytes[offset] as i16 + 2;
                cell.front_index = if front_idx == 102 { 90 } else if front_idx >= 255 { -1 } else { front_idx };
                offset += 1;
                
                cell.light = bytes[offset];
                offset += 1;
                
                cell.unknown = bytes[offset];
                offset += 1;
                
                // C#: Fishing cell detection
                if cell.light >= 100 && cell.light <= 119 {
                    cell.fishing_cell = true;
                }
            }
        }
        
        Ok(())
    }
    
    // ========================================================================
    // Map Type 2 - 旧 Shanda 格式 (14 bytes per cell)
    // C# Reference: Client/MirObjects/MapCode.cs lines 320-362
    // ========================================================================
    fn load_map_type_2(&mut self) -> io::Result<()> {
        let bytes = &self.bytes;
        let mut offset = 0;
        
        self.width = i16::from_le_bytes([bytes[offset], bytes[offset + 1]]) as i32;
        offset += 2;
        self.height = i16::from_le_bytes([bytes[offset], bytes[offset + 1]]) as i32;
        offset = 52; // C# starts at 52, not 28!
        
        self.map_cells = vec![vec![CellInfo::new(); self.height as usize]; self.width as usize];
        
        for x in 0..self.width as usize {
            for y in 0..self.height as usize {
                let cell = &mut self.map_cells[x][y];
                
                // Read images (2 bytes each, standard i16)
                cell.back_image = i16::from_le_bytes([bytes[offset], bytes[offset + 1]]) as i32;
                offset += 2;
                
                cell.middle_image = i16::from_le_bytes([bytes[offset], bytes[offset + 1]]) as i32;
                offset += 2;
                
                cell.front_image = i16::from_le_bytes([bytes[offset], bytes[offset + 1]]) as i32;
                offset += 2;
                
                cell.door_index = bytes[offset] & 0x7F;
                offset += 1;
                
                cell.door_offset = bytes[offset];
                offset += 1;
                
                cell.front_animation_frame = bytes[offset];
                offset += 1;
                
                cell.front_animation_tick = bytes[offset];
                offset += 1;
                
                // C# reads indices here (+ 120, + 100, + 110)
                cell.front_index = (bytes[offset] as i16) + 120;
                offset += 1;
                
                cell.light = bytes[offset];
                offset += 1;
                
                cell.back_index = (bytes[offset] as i16) + 100;
                offset += 1;
                
                cell.middle_index = (bytes[offset] as i16) + 110;
                offset += 1;
                
                // C#: BackImage flag processing
                if (cell.back_image & 0x8000) != 0 {
                    cell.back_image = (cell.back_image & 0x7FFF) | 0x20000000;
                }
                
                // C#: Fishing cell detection
                if cell.light >= 100 && cell.light <= 119 {
                    cell.fishing_cell = true;
                }
            }
        }
        
        Ok(())
    }
    
    // ========================================================================
    // Map Type 3 - Shanda 2012 格式 (36 bytes per cell)
    // C# Reference: Client/MirObjects/MapCode.cs lines 364-407
    // ========================================================================
    fn load_map_type_3(&mut self) -> io::Result<()> {
        let bytes = &self.bytes;
        let mut offset = 0;
        
        self.width = i16::from_le_bytes([bytes[offset], bytes[offset + 1]]) as i32;
        offset += 2;
        self.height = i16::from_le_bytes([bytes[offset], bytes[offset + 1]]) as i32;
        offset = 52; // C# starts at 52, not 20!
        
        self.map_cells = vec![vec![CellInfo::new(); self.height as usize]; self.width as usize];
        
        for x in 0..self.width as usize {
            for y in 0..self.height as usize {
                let cell = &mut self.map_cells[x][y];
                
                // Read images first (not indices)
                cell.back_image = i16::from_le_bytes([bytes[offset], bytes[offset + 1]]) as i32;
                offset += 2;
                
                cell.middle_image = i16::from_le_bytes([bytes[offset], bytes[offset + 1]]) as i32;
                offset += 2;
                
                cell.front_image = i16::from_le_bytes([bytes[offset], bytes[offset + 1]]) as i32;
                offset += 2;
                
                cell.door_index = bytes[offset] & 0x7F;
                offset += 1;
                
                cell.door_offset = bytes[offset];
                offset += 1;
                
                cell.front_animation_frame = bytes[offset];
                offset += 1;
                
                cell.front_animation_tick = bytes[offset];
                offset += 1;
                
                // C# reads indices here (+ 120, + 100, + 110)
                cell.front_index = (bytes[offset] as i16) + 120;
                offset += 1;
                
                cell.light = bytes[offset];
                offset += 1;
                
                cell.back_index = (bytes[offset] as i16) + 100;
                offset += 1;
                
                cell.middle_index = (bytes[offset] as i16) + 110;
                offset += 1;
                
                // TileAnimationImage (2 bytes)
                cell.tile_animation_image = i16::from_le_bytes([bytes[offset], bytes[offset + 1]]);
                offset += 7; // 2 bytes from tileanimframe + 5 bytes unknown
                
                // TileAnimationFrames (1 byte)
                cell.tile_animation_frames = bytes[offset];
                offset += 1;
                
                // TileAnimationOffset (2 bytes)
                cell.tile_animation_offset = i16::from_le_bytes([bytes[offset], bytes[offset + 1]]);
                offset += 14; // Skip light/blending options
                
                // C#: BackImage flag processing
                if (cell.back_image & 0x8000) != 0 {
                    cell.back_image = (cell.back_image & 0x7FFF) | 0x20000000;
                }
                
                // C#: Fishing cell detection
                if cell.light >= 100 && cell.light <= 119 {
                    cell.fishing_cell = true;
                }
            }
        }
        
        Ok(())
    }
    
    // ========================================================================
    // Map Type 4-7, 100 的存根实现
    // ========================================================================
    
    fn load_map_type_4(&mut self) -> io::Result<()> {
        // TODO: 实现 Wemade AntiHack 格式
        Err(io::Error::new(
            io::ErrorKind::Unsupported,
            "Map Type 4 not yet implemented",
        ))
    }
    
    fn load_map_type_5(&mut self) -> io::Result<()> {
        // TODO: 实现 Wemade Mir3 格式
        Err(io::Error::new(
            io::ErrorKind::Unsupported,
            "Map Type 5 not yet implemented",
        ))
    }
    
    fn load_map_type_6(&mut self) -> io::Result<()> {
        // TODO: 实现 Shanda Mir3 格式
        Err(io::Error::new(
            io::ErrorKind::Unsupported,
            "Map Type 6 not yet implemented",
        ))
    }
    
    fn load_map_type_7(&mut self) -> io::Result<()> {
        // TODO: 实现 3/4 Heroes 格式
        Err(io::Error::new(
            io::ErrorKind::Unsupported,
            "Map Type 7 not yet implemented",
        ))
    }
    
    /// Map Type 100 - C# 自定义格式
    /// 参考: Server/MirEnvir/Map.cs LoadMapCellsV100()
    /// 
    /// C# 代码逻辑:
    /// ```csharp
    /// offset += 2;                           // Skip BackImage (2 bytes)
    /// if ((BitConverter.ToInt32(Bytes, offset) & 0x20000000) != 0)
    ///     Cells[x, y] = Cell.HighWall;       // Check MiddleImage (4 bytes)
    /// offset += 10;                          // Skip MiddleImage(4) + MiddleIndex(2) + Animation(4)
    /// if ((BitConverter.ToInt16(Bytes, offset) & 0x8000) != 0)
    ///     Cells[x, y] = Cell.LowWall;        // Check FrontImage (2 bytes)
    /// offset += 2;                           // Skip FrontImage
    /// if (Bytes[offset] > 0)
    ///     DoorIndex[x, y] = AddDoor(...);    // Read Door (1 byte)
    /// offset += 11;                          // Skip Door(1) + Rest(10)
    /// byte light = Bytes[offset++];          // Read Light (1 byte)
    /// ```
    /// 
    /// 总计: 2+4+10+2+2+1+10+1 = 32 bytes per cell
    fn load_map_type_100(&mut self) -> io::Result<()> {
        // 检查版本 (只支持 version 1)
        if self.bytes[0] != 1 || self.bytes[1] != 0 {
            return Err(io::Error::new(
                io::ErrorKind::InvalidData,
                "Map Type 100: Only version 1 is supported",
            ));
        }
        
        // 读取宽度和高度 (offset 4-7)
        let mut offset = 4;
        self.width = i16::from_le_bytes([
            self.bytes[offset],
            self.bytes[offset + 1],
        ]) as i32;
        offset += 2;
        
        self.height = i16::from_le_bytes([
            self.bytes[offset],
            self.bytes[offset + 1],
        ]) as i32;
        offset += 2;
        
        // 初始化地图格子数组
        self.map_cells = vec![vec![CellInfo::new(); self.height as usize]; self.width as usize];
        // offset 现在是 8，开始读取格子数据
        // 🔧 修复：严格按照 C# LoadMapCellsV100 的逻辑实现
        // 参考: Client/MirObjects/MapCode.cs line 707-730
        for x in 0..self.width {
            for y in 0..self.height {
                let cell = &mut self.map_cells[x as usize][y as usize];
                
                // BackIndex (2字节)
                cell.back_index = i16::from_le_bytes([
                    self.bytes[offset],
                    self.bytes[offset + 1],
                ]) as i16;
                offset += 2;
                
                // BackImage (4字节) - 注意：这里是4字节int32
                cell.back_image = i32::from_le_bytes([
                    self.bytes[offset],
                    self.bytes[offset + 1],
                    self.bytes[offset + 2],
                    self.bytes[offset + 3],
                ]);
                offset += 4;
                
                // MiddleIndex (2字节) - 🔧 之前漏掉了！
                cell.middle_index = i16::from_le_bytes([
                    self.bytes[offset],
                    self.bytes[offset + 1],
                ]) as i16;
                offset += 2;
                
                // MiddleImage (2字节) - 🔧 之前错误地读成4字节！
                cell.middle_image = i16::from_le_bytes([
                    self.bytes[offset],
                    self.bytes[offset + 1],
                ]) as i32;
                offset += 2;
                
                // FrontIndex (2字节)
                cell.front_index = i16::from_le_bytes([
                    self.bytes[offset],
                    self.bytes[offset + 1],
                ]) as i16;
                offset += 2;
                
                // FrontImage (2字节)
                cell.front_image = i16::from_le_bytes([
                    self.bytes[offset],
                    self.bytes[offset + 1],
                ]) as i32;
                offset += 2;
                
                // DoorIndex (1字节，低7位)
                cell.door_index = self.bytes[offset] & 0x7F;
                offset += 1;
                
                // DoorOffset (1字节)
                cell.door_offset = self.bytes[offset];
                offset += 1;
                
                // FrontAnimationFrame (1字节)
                cell.front_animation_frame = self.bytes[offset];
                offset += 1;
                
                // FrontAnimationTick (1字节)
                cell.front_animation_tick = self.bytes[offset];
                offset += 1;
                
                // MiddleAnimationFrame (1字节)
                cell.middle_animation_frame = self.bytes[offset];
                offset += 1;
                
                // MiddleAnimationTick (1字节)
                cell.middle_animation_tick = self.bytes[offset];
                offset += 1;
                
                // TileAnimationImage (2字节)
                cell.tile_animation_image = i16::from_le_bytes([
                    self.bytes[offset],
                    self.bytes[offset + 1],
                ]) as i16;
                offset += 2;
                
                // TileAnimationOffset (2字节)
                cell.tile_animation_offset = i16::from_le_bytes([
                    self.bytes[offset],
                    self.bytes[offset + 1],
                ]) as i16;
                offset += 2;
                
                // TileAnimationFrames (1字节)
                cell.tile_animation_frames = self.bytes[offset];
                offset += 1;
                
                // Light (1字节)
                cell.light = self.bytes[offset];
                offset += 1;
                
                // C#: if (light >= 100 && light <= 119)
                //         Cells[x, y].FishingAttribute = (sbyte)(light - 100);
                if cell.light >= 100 && cell.light <= 119 {
                    cell.fishing_cell = true;
                }
            }
        }
        
        Ok(())
    }
    
    // 辅助方法：获取指定位置的 CellInfo
    pub fn get_cell(&self, x: i32, y: i32) -> Option<&CellInfo> {
        if x >= 0 && x < self.width && y >= 0 && y < self.height {
            Some(&self.map_cells[x as usize][y as usize])
        } else {
            None
        }
    }
    
    pub fn get_cell_mut(&mut self, x: i32, y: i32) -> Option<&mut CellInfo> {
        if x >= 0 && x < self.width && y >= 0 && y < self.height {
            Some(&mut self.map_cells[x as usize][y as usize])
        } else {
            None
        }
    }
}

// ============================================================================
// 测试
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_cell_info_creation() {
        let cell = CellInfo::new();
        assert_eq!(cell.back_index, 0);
        assert_eq!(cell.middle_index, 1);
        assert_eq!(cell.fishing_cell, false);
        assert!(cell.cell_objects.is_none());
    }
    
    #[test]
    fn test_cell_info_add_remove_object() {
        let mut cell = CellInfo::new();
        
        cell.add_object(123);
        assert!(cell.find_object(123));
        
        cell.add_object(456);
        assert!(cell.find_object(456));
        
        cell.remove_object(123);
        assert!(!cell.find_object(123));
        assert!(cell.find_object(456));
        
        cell.remove_object(456);
        assert!(cell.cell_objects.is_none());
    }
}
