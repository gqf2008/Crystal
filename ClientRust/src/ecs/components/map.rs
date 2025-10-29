// ============================================================================
// 地图相关组件
// ============================================================================

use std::time::Instant;
use crate::objects::CellInfo;

/// 地图瓦片组件
#[derive(Debug, Clone)]
pub struct MapTile {
    pub grid_x: i32,
    pub grid_y: i32,
    pub layer: TileLayer,
    pub library_index: i16,
    pub image_index: i32,
    pub use_blend: bool,
    pub brightness: f32,
    pub z_order: i32,
}

/// 瓦片层级
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum TileLayer {
    Back = 0,
    Middle = 1,
    Front = 2,
}

/// 动画瓦片组件
#[derive(Debug, Clone)]
pub struct AnimatedTile {
    pub frame_count: u8,
    pub frame_interval: u8,
    pub base_image_index: i32,
}

/// 门组件
#[derive(Debug, Clone)]
pub struct Door {
    pub door_index: u8,
    pub door_offset: i32,
    pub state: DoorState,
    pub current_frame: i32,
    pub last_tick: Instant,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum DoorState {
    Closed = 0,
    Opening = 1,
    Open = 2,
    Closing = 3,
}

/// 地图数据组件
#[derive(Clone)]
pub struct MapData {
    pub cells: Vec<Vec<CellInfo>>,
    pub width: i32,
    pub height: i32,
}

/// 地图边界组件 (用于碰撞检测)
#[derive(Debug, Clone, Copy)]
pub struct MapBounds {
    pub width: i32,
    pub height: i32,
}

impl MapBounds {
    pub fn new(width: i32, height: i32) -> Self {
        Self { width, height }
    }

    pub fn contains(&self, x: i32, y: i32) -> bool {
        x >= 0 && y >= 0 && x < self.width && y < self.height
    }
}

/// 常量
pub const CELL_WIDTH: i32 = 48;
pub const CELL_HEIGHT: i32 = 32;

/// 瓦片遮挡效果组件
/// 用于实现角色被前景瓦片遮挡时的半透明效果
#[derive(Debug, Clone)]
pub struct TileOcclusion {
    /// 当前透明度 (0.0 = 完全透明, 1.0 = 完全不透明)
    pub current_alpha: f32,
    /// 是否正在遮挡角色
    pub is_occluding: bool,
}

impl TileOcclusion {
    pub fn new() -> Self {
        Self {
            current_alpha: 1.0,
            is_occluding: false,
        }
    }
}

impl Default for TileOcclusion {
    fn default() -> Self {
        Self::new()
    }
}
