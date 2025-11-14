// Client/MirObjects/MapCode.cs 的 Rust 移植
// 包含 CellInfo 和 MapReader 两个类

use std::fs::File;
use std::io::{self, Read};
use std::path::Path;


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

    // 辅助方法：检查是否可行走
    // C# Reference: MapControl.ValidPoint() - (M2CellInfo[x, y].BackImage & 0x20000000) == 0
    // 
    // ✅ 关键修正：只检查 back_image 的障碍物标志位！
    // - front_image/middle_image 只是图片装饰，不阻挡移动
    // - door_index 在传奇2中通常为0（门系统未实现）
    // - 必须与 CollisionSystem 和 DebugSystem 的障碍物判断保持一致
    pub fn is_walkable(&self) -> bool {
        // ✅ 只检查 back_image 的障碍物标志位 (0x20000000)
        // 如果设置了这个位，说明该格子有障碍物，不可通行
        let has_obstacle = (self.back_image & 0x20000000) != 0;
        
        // ✅ 返回: 没有障碍物 = 可行走
        !has_obstacle
    }

    pub fn back_tile(&self) -> Option<(i16, i32)> {
        let index = (self.back_image & 0x1FFFFFFF) - 1;
        if self.back_image == 0 || self.back_index == -1 || index < 0 {
            return None;
        }
        Some((self.back_index, index))
    }

    pub fn middle_tile(&self) -> Option<(i16, i32)> {
        let index = self.middle_image - 1;
        // C#: if (index > 0) - 只处理index > 0的情况
        if index <= 0 || self.middle_index == -1 {
            return None;
        }
        Some((self.middle_index, index))
    }

    pub fn middle_has_animation(&self) -> bool {
        self.middle_animation_frame > 0 && self.middle_animation_frame < 255
    }

    pub fn middle_use_blend(&self) -> bool {
        (self.middle_animation_frame & 0x0f) > 0
    }

    pub fn front_tile(&self) -> Option<(i16, i32)> {
        let index = (self.front_image & 0x7FFF) - 1;
        if index == -1 || self.front_index == -1 || self.front_index == 200 {
            return None;
        }
        Some((self.front_index, index))
    }

    pub fn debug_cell_data(&self, x: i32, y: i32) {
        println!("🔍 格子 ({},{}) 数据:", x, y);
        println!("   Back:   index={:2}, image=0x{:08X}", self.back_index, self.back_image);
        if let Some((lib, img)) = self.back_tile() {
            println!("           → 库{} 图{}", lib, img);
        }
        println!("   Middle: index={:2}, image={}", self.middle_index, self.middle_image);
        if let Some((lib, img)) = self.middle_tile() {
            println!("           → 库{} 图{}", lib, img);
        }
        println!("   Front:  index={:2}, image=0x{:04X}", self.front_index, self.front_image);
        if let Some((lib, img)) = self.front_tile() {
            println!("           → 库{} 图{}", lib, img);
        }
        println!("   Flags:  door_index={}, door_offset={}", self.door_index, self.door_offset);
        println!("           light={}, fishing={}", self.light, self.fishing_cell);
    }
}

pub enum TileLayer {
    Back {
        index: i16,
        image: i32,
    },
    Middle {
        index: i16,
        image: i32,
        animation_frame: u8,
        animation_tick: u8,
    },
    Front {
        index: i16,
        image: i32,
        animation_frame: u8,
        animation_tick: u8,
    },
}

impl TileLayer {
    pub fn from_cell_info(cell: &CellInfo, layer: TileLayer) -> Option<Self> {
        match layer {
            TileLayer::Back { .. } => {
                let index = (cell.back_image & 0x1FFFFFFF) - 1;
                if cell.back_image == 0 || cell.back_index == -1 || index < 0 {
                    None
                } else {
                    Some(TileLayer::Back {
                        index: cell.back_index,
                        image: index,
                    })
                }
            }
            TileLayer::Middle { .. } => {
                let index = cell.middle_image - 1;
                if index < 0 || cell.middle_index == -1 {
                    None
                } else {
                    Some(TileLayer::Middle {
                        index: cell.middle_index,
                        image: index,
                        animation_frame: cell.middle_animation_frame,
                        animation_tick: cell.middle_animation_tick,
                    })
                }
            }
            TileLayer::Front { .. } => {
                let index = (cell.front_image & 0x7FFF) - 1;
                if index == -1 || cell.front_index == -1 || cell.front_index == 200 {
                    None
                } else {
                    Some(TileLayer::Front {
                        index: cell.front_index,
                        image: index,
                        animation_frame: cell.front_animation_frame,
                        animation_tick: cell.front_animation_tick,
                    })
                }
            }
        }
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

        // C# 自定义格式 (C#) - 必须在Type 5之前检查！
        // Type 100 格式特征：bytes[2]='C' (0x43), bytes[3]='#' (0x23)
        // 参考: Client/MirObjects/MapCode.cs line 198
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

        tracing::debug!(
            "📐 Map dimensions: {}x{} (XOR key: 0x{:04X})",
            self.width,
            self.height,
            xor
        );
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
                cell.front_index = if front_idx == 102 {
                    90
                } else if front_idx >= 255 {
                    -1
                } else {
                    front_idx
                };
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

    // ========================================================================
    // Map Type 4 - Wemade AntiHack 迷宫地图 (12 bytes per cell)
    // C# Reference: Client/MirObjects/MapCode.cs lines 409-445
    // ========================================================================
    fn load_map_type_4(&mut self) -> io::Result<()> {
        tracing::info!("🗺️  加载地图格式: Type 4 (Wemade AntiHack, 12 bytes/cell)");

        let bytes = &self.bytes;
        let mut offset = 31;

        // XOR 加密的尺寸
        let w = i16::from_le_bytes([bytes[offset], bytes[offset + 1]]);
        offset += 2;
        let xor = i16::from_le_bytes([bytes[offset], bytes[offset + 1]]);
        offset += 2;
        let h = i16::from_le_bytes([bytes[offset], bytes[offset + 1]]);

        // 解密尺寸
        self.width = (w ^ xor) as i32;
        self.height = (h ^ xor) as i32;
        offset = 64; // 头部 64 字节

        // 验证尺寸
        if self.width <= 0 || self.height <= 0 || self.width > 10000 || self.height > 10000 {
            return Err(std::io::Error::new(
                std::io::ErrorKind::InvalidData,
                format!("Invalid map dimensions: {}x{}", self.width, self.height),
            ));
        }

        tracing::info!(
            "📐 地图尺寸: {}x{} (XOR key: 0x{:04X})",
            self.width,
            self.height,
            xor
        );
        self.map_cells = vec![vec![CellInfo::new(); self.height as usize]; self.width as usize];

        for x in 0..self.width as usize {
            for y in 0..self.height as usize {
                let cell = &mut self.map_cells[x][y];

                cell.back_index = 0;
                cell.middle_index = 1;

                // BackImage: XOR 加密
                let back_raw = i16::from_le_bytes([bytes[offset], bytes[offset + 1]]);
                cell.back_image = (back_raw ^ xor) as i32;
                offset += 2;

                // MiddleImage: XOR 加密
                let middle_raw = i16::from_le_bytes([bytes[offset], bytes[offset + 1]]);
                cell.middle_image = (middle_raw ^ xor) as i32;
                offset += 2;

                // FrontImage: XOR 加密
                let front_raw = i16::from_le_bytes([bytes[offset], bytes[offset + 1]]);
                cell.front_image = (front_raw ^ xor) as i32;
                offset += 2;

                // Door
                cell.door_index = bytes[offset] & 0x7F;
                offset += 1;
                cell.door_offset = bytes[offset];
                offset += 1;

                // Animation
                cell.front_animation_frame = bytes[offset];
                offset += 1;
                cell.front_animation_tick = bytes[offset];
                offset += 1;

                // FrontIndex
                cell.front_index = (bytes[offset] as i16) + 2;
                offset += 1;

                // Light
                cell.light = bytes[offset];
                offset += 1;

                // C# 逻辑: BackImage 高位标记处理
                if (cell.back_image & 0x8000) != 0 {
                    cell.back_image = (cell.back_image & 0x7FFF) | 0x20000000;
                }

                // 钓鱼点检测
                if cell.light >= 100 && cell.light <= 119 {
                    cell.fishing_cell = true;
                }
            }
        }

        Ok(())
    }

    // ========================================================================
    // Map Type 5 - Wemade Mir3 格式 (木/沙/雪/森林风格)
    // C# Reference: Client/MirObjects/MapCode.cs lines 447-525
    // ========================================================================
    fn load_map_type_5(&mut self) -> io::Result<()> {
        tracing::info!("🗺️  加载地图格式: Type 5 (Wemade Mir3)");

        let bytes = &self.bytes;
        let mut offset = 20;

        // 读取属性和尺寸
        let _attribute = i16::from_le_bytes([bytes[offset], bytes[offset + 1]]);
        offset += 2;
        self.width = i16::from_le_bytes([bytes[offset], bytes[offset + 1]]) as i32;
        offset += 2;
        self.height = i16::from_le_bytes([bytes[offset], bytes[offset + 1]]) as i32;
        // offset += 2; // EventFile 和 FogColor 暂时忽略

        tracing::info!("📐 地图尺寸: {}x{}", self.width, self.height);

        offset = 28;

        // 初始化所有格子
        self.map_cells = vec![vec![CellInfo::new(); self.height as usize]; self.width as usize];

        // 读取所有 BackTiles (2x2 格子共享，压缩存储)
        for x in 0..(self.width / 2) as usize {
            for y in 0..(self.height / 2) as usize {
                // 每 3 字节存储 4 个格子的 BackTile 信息
                let back_index = if bytes[offset] != 255 {
                    (bytes[offset] as i16) + 200
                } else {
                    -1
                };

                let back_image =
                    (u16::from_le_bytes([bytes[offset + 1], bytes[offset + 2]]) as i32) + 1;

                // 分配给 4 个格子 (2x2)
                for i in 0..4 {
                    let cell_x = (x * 2) + (i % 2);
                    let cell_y = (y * 2) + (i / 2);
                    if cell_x < self.width as usize && cell_y < self.height as usize {
                        self.map_cells[cell_x][cell_y].back_index = back_index;
                        self.map_cells[cell_x][cell_y].back_image = back_image;
                    }
                }

                offset += 3;
            }
        }

        // 读取剩余数据
        offset = 28
            + (3 * (((self.width / 2) + (self.width % 2)) as usize) * ((self.height / 2) as usize));

        for x in 0..self.width as usize {
            for y in 0..self.height as usize {
                let cell = &mut self.map_cells[x][y];

                let flag = bytes[offset];
                offset += 1;

                cell.middle_animation_frame = bytes[offset];
                offset += 1;

                let front_anim_raw = bytes[offset];
                cell.front_animation_frame = if front_anim_raw == 255 {
                    0
                } else {
                    front_anim_raw & 0x8F
                };
                offset += 1;

                cell.middle_animation_tick = 0;
                cell.front_animation_tick = 0;

                cell.front_index = if bytes[offset] != 255 {
                    (bytes[offset] as i16) + 200
                } else {
                    -1
                };
                offset += 1;

                cell.middle_index = if bytes[offset] != 255 {
                    (bytes[offset] as i16) + 200
                } else {
                    -1
                };
                offset += 1;

                cell.middle_image =
                    (u16::from_le_bytes([bytes[offset], bytes[offset + 1]]) as i32) + 1;
                offset += 2;

                cell.front_image =
                    (u16::from_le_bytes([bytes[offset], bytes[offset + 1]]) as i32) + 1;
                offset += 2;

                // 特殊处理: FrontImage==1 且 FrontIndex==200 表示无前景
                if cell.front_image == 1 && cell.front_index == 200 {
                    cell.front_index = -1;
                }

                offset += 3; // Mir3 地图没有门，跳过

                cell.light = bytes[offset] & 0x0F;
                offset += 2;

                // Flag 处理
                if (flag & 0x01) != 1 {
                    cell.back_image |= 0x20000000;
                }
                if (flag & 0x02) != 2 {
                    cell.front_image = (cell.front_image as u16 | 0x8000) as i32;
                }

                // 钓鱼点检测
                if cell.light >= 100 && cell.light <= 119 {
                    cell.fishing_cell = true;
                } else {
                    // 扩展 Mir3 光照范围（默认范围较小）
                    cell.light *= 2;
                }
            }
        }

        Ok(())
    }

    // ========================================================================
    // Map Type 6 - Shanda Mir3 格式
    // C# Reference: Client/MirObjects/MapCode.cs lines 527-576
    // ========================================================================
    fn load_map_type_6(&mut self) -> io::Result<()> {
        tracing::info!("🗺️  加载地图格式: Type 6 (Shanda Mir3)");

        let bytes = &self.bytes;
        let mut offset = 16;

        self.width = i16::from_le_bytes([bytes[offset], bytes[offset + 1]]) as i32;
        offset += 2;
        self.height = i16::from_le_bytes([bytes[offset], bytes[offset + 1]]) as i32;
        offset = 40; // 头部 40 字节

        tracing::info!("📐 地图尺寸: {}x{}", self.width, self.height);

        self.map_cells = vec![vec![CellInfo::new(); self.height as usize]; self.width as usize];

        for x in 0..self.width as usize {
            for y in 0..self.height as usize {
                let cell = &mut self.map_cells[x][y];

                let flag = bytes[offset];
                offset += 1;

                // 图库索引 (+ 300 偏移)
                cell.back_index = if bytes[offset] != 255 {
                    (bytes[offset] as i16) + 300
                } else {
                    -1
                };
                offset += 1;

                cell.middle_index = if bytes[offset] != 255 {
                    (bytes[offset] as i16) + 300
                } else {
                    -1
                };
                offset += 1;

                cell.front_index = if bytes[offset] != 255 {
                    (bytes[offset] as i16) + 300
                } else {
                    -1
                };
                offset += 1;

                // 图像索引
                cell.back_image =
                    (i16::from_le_bytes([bytes[offset], bytes[offset + 1]]) as i32) + 1;
                offset += 2;

                cell.middle_image =
                    (i16::from_le_bytes([bytes[offset], bytes[offset + 1]]) as i32) + 1;
                offset += 2;

                cell.front_image =
                    (i16::from_le_bytes([bytes[offset], bytes[offset + 1]]) as i32) + 1;
                offset += 2;

                // 特殊处理: FrontImage==1 且 FrontIndex==200 表示无前景
                if cell.front_image == 1 && cell.front_index == 200 {
                    cell.front_index = -1;
                }

                // 动画
                cell.middle_animation_frame = bytes[offset];
                offset += 1;

                let front_anim_raw = bytes[offset];
                cell.front_animation_frame = if front_anim_raw == 255 {
                    0
                } else {
                    front_anim_raw
                };

                // 混合模式检测 (Shanda 使用相同的值)
                if cell.front_animation_frame > 0x0F {
                    cell.front_animation_frame = cell.front_animation_frame & 0x0F;
                }
                offset += 1;

                cell.middle_animation_tick = 1;
                cell.front_animation_tick = 1;

                // 光照
                cell.light = bytes[offset] & 0x0F;
                cell.light *= 4; // Shanda Mir3 光照需要放大
                offset += 8;

                // Flag 处理
                if (flag & 0x01) != 1 {
                    cell.back_image |= 0x20000000;
                }
                if (flag & 0x02) != 2 {
                    cell.front_image = (cell.front_image as u16 | 0x8000) as i32;
                }
            }
        }

        Ok(())
    }

    // ========================================================================
    // Map Type 7 - 3/4 Heroes 格式
    // C# Reference: Client/MirObjects/MapCode.cs lines 578-621
    // 类似 Type 1，但每格 15 字节（多 1 字节）
    // ========================================================================
    fn load_map_type_7(&mut self) -> io::Result<()> {
        tracing::info!("🗺️  加载地图格式: Type 7 (3/4 Heroes)");

        let bytes = &self.bytes;
        let mut offset = 16;

        self.width = i16::from_le_bytes([bytes[offset], bytes[offset + 1]]) as i32;
        offset += 2;
        self.height = i16::from_le_bytes([bytes[offset], bytes[offset + 1]]) as i32;
        offset = 40; // 头部 40 字节

        tracing::info!("📐 地图尺寸: {}x{}", self.width, self.height);

        self.map_cells = vec![vec![CellInfo::new(); self.height as usize]; self.width as usize];

        for x in 0..self.width as usize {
            for y in 0..self.height as usize {
                let cell = &mut self.map_cells[x][y];

                // 图库索引 (+ 1 偏移)
                cell.back_index = bytes[offset] as i16;
                if cell.back_index == 0 {
                    cell.back_index = -1;
                }
                offset += 1;

                cell.middle_index = bytes[offset] as i16;
                if cell.middle_index == 0 {
                    cell.middle_index = -1;
                }
                offset += 1;

                cell.front_index = bytes[offset] as i16;
                if cell.front_index == 0 {
                    cell.front_index = -1;
                }
                offset += 1;

                // 图像索引
                cell.back_image = (bytes[offset] as i32) + 1;
                offset += 1;

                cell.middle_image = (bytes[offset] as i32) + 1;
                offset += 1;

                cell.front_image =
                    (i16::from_le_bytes([bytes[offset], bytes[offset + 1]]) as i32) + 1;
                offset += 2;

                // 动画
                cell.middle_animation_frame = bytes[offset];
                offset += 1;

                let front_anim_raw = bytes[offset];
                cell.front_animation_frame = if front_anim_raw == 255 {
                    0
                } else {
                    front_anim_raw
                };

                // 混合模式检测 (与 Type 1 相同)
                if cell.front_animation_frame > 0x0F {
                    cell.front_animation_frame = cell.front_animation_frame & 0x0F;
                }
                offset += 1;

                cell.middle_animation_tick = 1;
                cell.front_animation_tick = 1;

                // 光照
                cell.light = bytes[offset] & 0x0F;
                offset += 1;

                // 未知字段 (Type 7 特有)
                cell.unknown = bytes[offset];
                offset += 5; // Type 7 每格 15 字节，最后 5 字节跳过
            }
        }

        Ok(())
    }

    /// Map Type 100 - C# 自定义格式
    /// 参考: Client/MirObjects/MapCode.cs line 700-733
    ///
    /// 格式说明:
    /// - offset 4-5: Width (2 bytes)
    /// - offset 6-7: Height (2 bytes)
    /// - offset 8: 数据开始
    ///
    /// 每个格子 24 bytes:
    /// - BackIndex (2 bytes)
    /// - BackImage (4 bytes)
    /// - MiddleIndex (2 bytes)
    /// - MiddleImage (2 bytes)
    /// - FrontIndex (2 bytes)
    /// - FrontImage (2 bytes)
    /// - DoorIndex (1 byte)
    /// - DoorOffset (1 byte)
    /// - FrontAnimationFrame (1 byte)
    /// - FrontAnimationTick (1 byte)
    /// - MiddleAnimationFrame (1 byte)
    /// - MiddleAnimationTick (1 byte)
    /// - TileAnimationImage (2 bytes)
    /// - TileAnimationOffset (2 bytes)
    /// - TileAnimationFrames (1 byte)
    /// - Light (1 byte)
    fn load_map_type_100(&mut self) -> io::Result<()> {
        tracing::info!("🗺️  加载地图格式: Type 100 (C# 自定义格式, 24 bytes/cell)");

        // 读取宽度和高度 (offset 4-7)
        let mut offset = 4;
        self.width = i16::from_le_bytes([self.bytes[offset], self.bytes[offset + 1]]) as i32;
        offset += 2;

        self.height = i16::from_le_bytes([self.bytes[offset], self.bytes[offset + 1]]) as i32;
        offset += 2;

        tracing::info!("📐 地图尺寸: {}x{}", self.width, self.height);

        // 初始化地图格子数组
        self.map_cells = vec![vec![CellInfo::new(); self.height as usize]; self.width as usize];

        // offset 现在是 8，开始读取格子数据
        // 参考: Client/MirObjects/MapCode.cs line 707-730
        for x in 0..self.width {
            for y in 0..self.height {
                let cell = &mut self.map_cells[x as usize][y as usize];

                // BackIndex (2字节)
                cell.back_index = i16::from_le_bytes([self.bytes[offset], self.bytes[offset + 1]]);
                offset += 2;

                // BackImage (4字节) - 注意：这里是4字节int32
                cell.back_image = i32::from_le_bytes([
                    self.bytes[offset],
                    self.bytes[offset + 1],
                    self.bytes[offset + 2],
                    self.bytes[offset + 3],
                ]);
                offset += 4;

                // MiddleIndex (2字节)
                cell.middle_index =
                    i16::from_le_bytes([self.bytes[offset], self.bytes[offset + 1]]);
                offset += 2;

                // MiddleImage (2字节)
                cell.middle_image =
                    i16::from_le_bytes([self.bytes[offset], self.bytes[offset + 1]]) as i32;
                offset += 2;

                // FrontIndex (2字节)
                cell.front_index = i16::from_le_bytes([self.bytes[offset], self.bytes[offset + 1]]);
                offset += 2;

                // FrontImage (2字节)
                cell.front_image =
                    i16::from_le_bytes([self.bytes[offset], self.bytes[offset + 1]]) as i32;
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
                cell.tile_animation_image =
                    i16::from_le_bytes([self.bytes[offset], self.bytes[offset + 1]]);
                offset += 2;

                // TileAnimationOffset (2字节)
                cell.tile_animation_offset =
                    i16::from_le_bytes([self.bytes[offset], self.bytes[offset + 1]]);
                offset += 2;

                // TileAnimationFrames (1字节)
                cell.tile_animation_frames = self.bytes[offset];
                offset += 1;

                // Light (1字节)
                cell.light = self.bytes[offset];
                offset += 1;

                // 钓鱼点检测
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

    // ============================================================================
    // 测试 Type 0 地图格式
    // ============================================================================
    #[test]
    fn test_map_type0_basic() {
        let mut bytes = vec![0u8; 52 + 12]; // 头部52字节 + 1个格子(12字节)

        // 写入宽度和高度
        bytes[0..2].copy_from_slice(&1i16.to_le_bytes()); // width = 1
        bytes[2..4].copy_from_slice(&1i16.to_le_bytes()); // height = 1

        // 写入第一个格子的数据 (offset 52)
        bytes[52..54].copy_from_slice(&100i16.to_le_bytes()); // BackImage
        bytes[54..56].copy_from_slice(&200i16.to_le_bytes()); // MiddleImage
        bytes[56..58].copy_from_slice(&300i16.to_le_bytes()); // FrontImage
        bytes[58] = 0; // DoorIndex
        bytes[59] = 0; // DoorOffset
        bytes[60] = 0; // FrontAnimationFrame
        bytes[61] = 0; // FrontAnimationTick
        bytes[62] = 2; // FrontIndex (会被转换为 2+2=4)
        bytes[63] = 50; // Light

        // 保存到临时文件
        let temp_path = "test_type0.map";
        std::fs::write(temp_path, &bytes).unwrap();

        // 执行测试
        let reader = MapReader::new(temp_path).unwrap();

        // 验证
        assert_eq!(reader.width, 1);
        assert_eq!(reader.height, 1);
        assert_eq!(reader.map_cells[0][0].back_image, 100);
        assert_eq!(reader.map_cells[0][0].middle_image, 200);
        assert_eq!(reader.map_cells[0][0].front_image, 300);
        assert_eq!(reader.map_cells[0][0].front_index, 4); // 2 + 2
        assert_eq!(reader.map_cells[0][0].light, 50);

        // 清理
        std::fs::remove_file(temp_path).unwrap();
    }

    // ============================================================================
    // 测试 Type 100 地图格式
    // ============================================================================
    #[test]
    fn test_map_type100_structure() {
        let mut bytes = vec![0u8; 8 + 26]; // 头部8字节 + 1个格子(26字节)

        // 魔术字节: C# (0x43, 0x23)
        bytes[2] = 0x43;
        bytes[3] = 0x23;

        // 版本号
        bytes[0] = 1; // version 1
        bytes[1] = 0;

        // 尺寸
        bytes[4..6].copy_from_slice(&1i16.to_le_bytes()); // width = 1
        bytes[6..8].copy_from_slice(&1i16.to_le_bytes()); // height = 1

        // 第一个格子 (offset 8)
        let offset = 8;
        bytes[offset..offset + 2].copy_from_slice(&5i16.to_le_bytes()); // BackIndex
        bytes[offset + 2..offset + 6].copy_from_slice(&1000i32.to_le_bytes()); // BackImage
        bytes[offset + 6..offset + 8].copy_from_slice(&6i16.to_le_bytes()); // MiddleIndex
        bytes[offset + 8..offset + 10].copy_from_slice(&2000i16.to_le_bytes()); // MiddleImage
        bytes[offset + 10..offset + 12].copy_from_slice(&7i16.to_le_bytes()); // FrontIndex
        bytes[offset + 12..offset + 14].copy_from_slice(&3000i16.to_le_bytes()); // FrontImage
        bytes[offset + 14] = 10; // DoorIndex
        bytes[offset + 15] = 5; // DoorOffset
        bytes[offset + 16] = 2; // FrontAnimationFrame
        bytes[offset + 17] = 1; // FrontAnimationTick
        bytes[offset + 18] = 3; // MiddleAnimationFrame
        bytes[offset + 19] = 2; // MiddleAnimationTick
        bytes[offset + 20..offset + 22].copy_from_slice(&100i16.to_le_bytes()); // TileAnimationImage
        bytes[offset + 22..offset + 24].copy_from_slice(&0x2000i16.to_le_bytes()); // TileAnimationOffset
        bytes[offset + 24] = 8; // TileAnimationFrames
        bytes[offset + 25] = 128; // Light

        let temp_path = "test_type100.map";
        std::fs::write(temp_path, &bytes).unwrap();

        let reader = MapReader::new(temp_path).unwrap();

        // 验证头部
        assert_eq!(reader.width, 1);
        assert_eq!(reader.height, 1);

        // 验证第一个格子
        let cell = &reader.map_cells[0][0];
        assert_eq!(cell.back_index, 5);
        assert_eq!(cell.back_image, 1000);
        assert_eq!(cell.middle_index, 6);
        assert_eq!(cell.middle_image, 2000);
        assert_eq!(cell.front_index, 7);
        assert_eq!(cell.front_image, 3000);
        assert_eq!(cell.door_index, 10);
        assert_eq!(cell.door_offset, 5);
        assert_eq!(cell.front_animation_frame, 2);
        assert_eq!(cell.front_animation_tick, 1);
        assert_eq!(cell.middle_animation_frame, 3);
        assert_eq!(cell.middle_animation_tick, 2);
        assert_eq!(cell.tile_animation_image, 100);
        assert_eq!(cell.tile_animation_offset, 0x2000);
        assert_eq!(cell.tile_animation_frames, 8);
        assert_eq!(cell.light, 128);

        std::fs::remove_file(temp_path).unwrap();
    }

    // ============================================================================
    // 测试 BackImage 高位标记处理
    // ============================================================================
    #[test]
    fn test_back_image_flag_processing() {
        // Type 0 格式: 如果 BackImage & 0x8000 != 0，需要转换为 0x20000000 标记
        let mut bytes = vec![0u8; 52 + 12];
        bytes[0..2].copy_from_slice(&1i16.to_le_bytes());
        bytes[2..4].copy_from_slice(&1i16.to_le_bytes());

        // BackImage = 0x8001 (设置了高位标记)
        let back_with_flag = 0x8001u16 as i16;
        bytes[52..54].copy_from_slice(&back_with_flag.to_le_bytes());

        let temp_path = "test_back_flag.map";
        std::fs::write(temp_path, &bytes).unwrap();

        let reader = MapReader::new(temp_path).unwrap();

        // C# 逻辑: if ((BackImage & 0x8000) != 0)
        //              BackImage = (BackImage & 0x7FFF) | 0x20000000;
        let expected = (0x8001 & 0x7FFF) | 0x20000000;
        assert_eq!(reader.map_cells[0][0].back_image, expected);

        std::fs::remove_file(temp_path).unwrap();
    }

    // ============================================================================
    // 测试钓鱼格子检测
    // ============================================================================
    #[test]
    fn test_fishing_cell_detection() {
        let mut bytes = vec![0u8; 52 + 12];
        bytes[0..2].copy_from_slice(&1i16.to_le_bytes());
        bytes[2..4].copy_from_slice(&1i16.to_le_bytes());

        // Light = 105 (100-119 范围表示钓鱼点)
        bytes[52 + 11] = 105;

        let temp_path = "test_fishing.map";
        std::fs::write(temp_path, &bytes).unwrap();

        let reader = MapReader::new(temp_path).unwrap();

        assert_eq!(reader.map_cells[0][0].light, 105);
        assert!(reader.map_cells[0][0].fishing_cell);

        std::fs::remove_file(temp_path).unwrap();
    }

    // ============================================================================
    // 测试 get_cell 边界检查
    // ============================================================================
    #[test]
    fn test_get_cell_bounds() {
        let mut bytes = vec![0u8; 52 + 12 * 4]; // 2x2 地图
        bytes[0..2].copy_from_slice(&2i16.to_le_bytes());
        bytes[2..4].copy_from_slice(&2i16.to_le_bytes());

        let temp_path = "test_bounds.map";
        std::fs::write(temp_path, &bytes).unwrap();

        let reader = MapReader::new(temp_path).unwrap();

        // 有效坐标
        assert!(reader.get_cell(0, 0).is_some());
        assert!(reader.get_cell(1, 1).is_some());

        // 无效坐标
        assert!(reader.get_cell(-1, 0).is_none());
        assert!(reader.get_cell(0, -1).is_none());
        assert!(reader.get_cell(2, 0).is_none());
        assert!(reader.get_cell(0, 2).is_none());

        std::fs::remove_file(temp_path).unwrap();
    }
}
