use crate::cell_info::CellInfo;
use anyhow::{Context, Result};
use byteorder::{LittleEndian, ReadBytesExt};
use std::fs::File;
use std::io::{BufReader, Read};
use std::path::Path;

pub struct MapReader {
    pub width: usize,
    pub height: usize,
    pub cells: Vec<Vec<CellInfo>>,
}

impl MapReader {
    /// 创建新的空白地图
    pub fn new_empty(width: usize, height: usize) -> Self {
        let cells = vec![vec![CellInfo::default(); height]; width];
        MapReader {
            width,
            height,
            cells,
        }
    }

    /// 从文件加载地图
    pub fn load_from_file<P: AsRef<Path>>(path: P) -> Result<Self> {
        let file = File::open(&path)
            .with_context(|| format!("Failed to open map file: {:?}", path.as_ref()))?;
        
        let mut reader = BufReader::new(file);
        let mut header = [0u8; 4];
        reader.read_exact(&mut header)?;

        // 检测地图格式
        if header[2] == 0x43 && header[3] == 0x23 {
            // C# 自定义格式
            Self::load_custom_format(reader)
        } else if header[0] == 0 {
            // Wemade Mir3 格式
            Self::load_wemade_mir3(reader)
        } else if header[0] == 0x0F && header[5] == 0x53 {
            // Shanda Mir3 格式
            Self::load_shanda_mir3(reader)
        } else {
            // 默认为 Wemade Mir2 格式
            Self::load_wemade_mir2(reader, header)
        }
    }

    /// 加载自定义格式地图 (C#)
    fn load_custom_format(mut reader: BufReader<File>) -> Result<Self> {
        let width = reader.read_i16::<LittleEndian>()? as usize;
        let height = reader.read_i16::<LittleEndian>()? as usize;

        let mut cells = vec![vec![CellInfo::default(); height]; width];

        for x in 0..width {
            for y in 0..height {
                let mut cell = CellInfo::default();
                
                cell.back_index = reader.read_i16::<LittleEndian>()?;
                cell.back_image = reader.read_i32::<LittleEndian>()?;
                cell.middle_index = reader.read_i16::<LittleEndian>()?;
                cell.middle_image = reader.read_i16::<LittleEndian>()?;
                cell.front_index = reader.read_i16::<LittleEndian>()?;
                cell.front_image = reader.read_i16::<LittleEndian>()?;
                cell.door_index = reader.read_u8()?;
                cell.door_offset = reader.read_u8()?;
                cell.front_animation_frame = reader.read_u8()?;
                cell.front_animation_tick = reader.read_u8()?;
                cell.middle_animation_frame = reader.read_u8()?;
                cell.middle_animation_tick = reader.read_u8()?;
                cell.tile_animation_image = reader.read_i16::<LittleEndian>()?;
                cell.tile_animation_offset = reader.read_i16::<LittleEndian>()?;
                cell.tile_animation_frames = reader.read_u8()?;
                cell.light = reader.read_u8()?;
                cell.unknown = reader.read_u8()?;
                cell.fishing_cell = reader.read_u8()? != 0;

                cells[x][y] = cell;
            }
        }

        Ok(MapReader {
            width,
            height,
            cells,
        })
    }

    /// 加载 Wemade Mir2 格式 (占位实现)
    fn load_wemade_mir2(_reader: BufReader<File>, _header: [u8; 4]) -> Result<Self> {
        // TODO: 实现 Wemade Mir2 格式解析
        Ok(Self::new_empty(1000, 1000))
    }

    /// 加载 Wemade Mir3 格式 (占位实现)
    fn load_wemade_mir3(_reader: BufReader<File>) -> Result<Self> {
        // TODO: 实现 Wemade Mir3 格式解析
        Ok(Self::new_empty(1000, 1000))
    }

    /// 加载 Shanda Mir3 格式 (占位实现)
    fn load_shanda_mir3(_reader: BufReader<File>) -> Result<Self> {
        // TODO: 实现 Shanda Mir3 格式解析
        Ok(Self::new_empty(1000, 1000))
    }

    /// 保存地图到文件
    pub fn save_to_file<P: AsRef<Path>>(&self, path: P) -> Result<()> {
        use byteorder::WriteBytesExt;
        use std::io::BufWriter;

        let file = File::create(&path)?;
        let mut writer = BufWriter::new(file);

        // 写入头部
        writer.write_i16::<LittleEndian>(1)?; // version
        writer.write_all(b"C#")?; // tag

        // 写入地图尺寸
        writer.write_i16::<LittleEndian>(self.width as i16)?;
        writer.write_i16::<LittleEndian>(self.height as i16)?;

        // 写入单元格数据
        for x in 0..self.width {
            for y in 0..self.height {
                let cell = &self.cells[x][y];
                
                writer.write_i16::<LittleEndian>(cell.back_index)?;
                writer.write_i32::<LittleEndian>(cell.back_image)?;
                writer.write_i16::<LittleEndian>(cell.middle_index)?;
                writer.write_i16::<LittleEndian>(cell.middle_image)?;
                writer.write_i16::<LittleEndian>(cell.front_index)?;
                writer.write_i16::<LittleEndian>(cell.front_image)?;
                writer.write_u8(cell.door_index)?;
                writer.write_u8(cell.door_offset)?;
                writer.write_u8(cell.front_animation_frame)?;
                writer.write_u8(cell.front_animation_tick)?;
                writer.write_u8(cell.middle_animation_frame)?;
                writer.write_u8(cell.middle_animation_tick)?;
                writer.write_i16::<LittleEndian>(cell.tile_animation_image)?;
                writer.write_i16::<LittleEndian>(cell.tile_animation_offset)?;
                writer.write_u8(cell.tile_animation_frames)?;
                writer.write_u8(cell.light)?;
                writer.write_u8(cell.unknown)?;
                writer.write_u8(if cell.fishing_cell { 1 } else { 0 })?;
            }
        }

        Ok(())
    }

    /// 获取指定位置的单元格
    pub fn get_cell(&self, x: usize, y: usize) -> Option<&CellInfo> {
        if x < self.width && y < self.height {
            Some(&self.cells[x][y])
        } else {
            None
        }
    }

    /// 设置指定位置的单元格
    pub fn set_cell(&mut self, x: usize, y: usize, cell: CellInfo) -> bool {
        if x < self.width && y < self.height {
            self.cells[x][y] = cell;
            true
        } else {
            false
        }
    }
}
