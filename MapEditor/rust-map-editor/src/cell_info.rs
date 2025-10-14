use serde::{Deserialize, Serialize};

/// 地图单元格信息，对应 C# 的 CellInfo
#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub struct CellInfo {
    /// 背景资源索引
    pub back_index: i16,
    /// 背景图片索引（包含标志位）
    pub back_image: i32,
    
    /// 中间层资源索引
    pub middle_index: i16,
    /// 中间层图片索引
    pub middle_image: i16,
    
    /// 前景资源索引
    pub front_index: i16,
    /// 前景图片索引（包含标志位）
    pub front_image: i16,
    
    /// 门索引
    pub door_index: u8,
    /// 门偏移
    pub door_offset: u8,
    
    /// 前景动画帧
    pub front_animation_frame: u8,
    /// 前景动画速度
    pub front_animation_tick: u8,
    
    /// 中间层动画帧
    pub middle_animation_frame: u8,
    /// 中间层动画速度
    pub middle_animation_tick: u8,
    
    /// 瓷砖动画图片
    pub tile_animation_image: i16,
    /// 瓷砖动画偏移
    pub tile_animation_offset: i16,
    /// 瓷砖动画帧数
    pub tile_animation_frames: u8,
    
    /// 光照值
    pub light: u8,
    /// 未知字段
    pub unknown: u8,
    
    /// 钓鱼单元格
    pub fishing_cell: bool,
}

impl Default for CellInfo {
    fn default() -> Self {
        CellInfo {
            back_index: 0,
            back_image: 0,
            middle_index: 0,
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
            fishing_cell: false,
        }
    }
}

impl CellInfo {
    /// 检查背景是否有移动限制
    pub fn has_back_limit(&self) -> bool {
        (self.back_image & 0x20000000) != 0
    }
    
    /// 检查前景是否有移动限制
    pub fn has_front_limit(&self) -> bool {
        (self.front_image & 0x8000) != 0
    }
    
    /// 获取实际的背景图片索引（去除标志位）
    pub fn get_back_image_index(&self) -> i32 {
        (self.back_image & 0x1FFFFFFF) - 1
    }
    
    /// 获取实际的前景图片索引（去除标志位）
    pub fn get_front_image_index(&self) -> i16 {
        (self.front_image & 0x7FFF) - 1
    }
    
    /// 获取实际的中间层图片索引
    pub fn get_middle_image_index(&self) -> i16 {
        self.middle_image - 1
    }
    
    /// 检查门是否为实体门
    pub fn is_entity_door(&self) -> bool {
        (self.door_index & 0x80) != 0
    }
    
    /// 获取实际的门索引
    pub fn get_door_index(&self) -> u8 {
        self.door_index & 0x7F
    }
    
    /// 检查前景动画是否需要混合
    pub fn front_animation_blend(&self) -> bool {
        (self.front_animation_frame & 0x80) != 0
    }
    
    /// 获取实际的前景动画帧数
    pub fn get_front_animation_frames(&self) -> u8 {
        self.front_animation_frame & 0x7F
    }
    
    /// 检查中间层动画是否需要混合
    pub fn middle_animation_blend(&self) -> bool {
        self.middle_animation_frame > 0 
            && self.middle_animation_frame < 255 
            && (self.middle_animation_frame & 0x0F) > 0
    }
    
    /// 获取实际的中间层动画帧数
    pub fn get_middle_animation_frames(&self) -> u8 {
        if self.middle_animation_frame > 0 && self.middle_animation_frame < 255 {
            self.middle_animation_frame & 0x0F
        } else {
            0
        }
    }
}

/// 带坐标的单元格数据
#[derive(Debug, Clone)]
pub struct CellInfoData {
    pub x: i32,
    pub y: i32,
    pub cell_info: CellInfo,
}

impl CellInfoData {
    pub fn new(x: i32, y: i32, cell_info: CellInfo) -> Self {
        CellInfoData { x, y, cell_info }
    }
}
