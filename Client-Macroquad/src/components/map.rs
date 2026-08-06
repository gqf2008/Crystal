// ============================================================================
// 地图相关组件
// ============================================================================

use crate::resources::map_reader::CellInfo;
use std::time::Instant;

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

/// 碰撞信息组件 - 记录最近的碰撞位置
#[derive(Debug, Clone)]
pub struct CollisionInfo {
    /// 碰撞的格子坐标
    pub collision_grids: Vec<(i32, i32)>,
    /// 上次更新时间
    pub last_update: std::time::Instant,
}

impl CollisionInfo {
    pub fn new() -> Self {
        Self {
            collision_grids: Vec::new(),
            last_update: std::time::Instant::now(),
        }
    }

    pub fn clear(&mut self) {
        self.collision_grids.clear();
    }

    pub fn add_collision(&mut self, grid_x: i32, grid_y: i32) {
        // 避免重复添加
        if !self.collision_grids.contains(&(grid_x, grid_y)) {
            self.collision_grids.push((grid_x, grid_y));
        }
        self.last_update = std::time::Instant::now();
    }

    /// 清除超过指定时间的碰撞记录
    pub fn clear_old_collisions(&mut self, max_age_secs: f32) {
        if self.last_update.elapsed().as_secs_f32() > max_age_secs {
            self.clear();
        }
    }
}

impl Default for CollisionInfo {
    fn default() -> Self {
        Self::new()
    }
}

/// 当前地图天气状态（全局资源组件，绑定到 World 中唯一实体）
#[derive(Debug, Clone, Copy, Default)]
pub struct WeatherState {
    /// 天气类型码：0=晴天, 1=雨, 2=雪, 3=雾, 4=沙尘
    pub weather_code: u16,
    /// 当前天气对应的粒子发射器实体（如果有）
    pub emitter_entity: Option<hecs::Entity>,
}

/// 当前时间（全局资源组件，来自服务器 TimeOfDayChanged）
#[derive(Debug, Clone, Copy, Default)]
pub struct TimeOfDay {
    pub hour: u8,
}
