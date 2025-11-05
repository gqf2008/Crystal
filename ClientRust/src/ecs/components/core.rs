// ============================================================================
// 核心组件 - 所有实体的基础组件
// ============================================================================

pub use mir2_shared::{MirDirection as Direction, MirAction, MirClass, MirGender};
use crate::objects::SpriteBlendMode;

/// 位置组件 - 世界坐标（像素级，支持浮点）
/// 统一使用 f32 坐标系统，支持平滑移动和精确渲染
#[derive(Debug, Clone, Copy)]
pub struct Position {
    pub x: f32,      // 世界坐标 X（像素）
    pub y: f32,      // 世界坐标 Y（像素）
}

impl Position {
    pub fn new(x: f32, y: f32) -> Self {
        Self { x, y }
    }
    
    /// 从整数格子坐标创建（48x32像素单元格）
    pub fn from_grid(grid_x: i32, grid_y: i32) -> Self {
        Self {
            x: grid_x as f32 * 48.0,
            y: grid_y as f32 * 32.0,
        }
    }
}

/// 速度组件 - 移动实体必备
#[derive(Debug, Clone, Copy)]
pub struct Velocity {
    pub dx: f32,
    pub dy: f32,
}

impl Velocity {
    pub fn new(dx: f32, dy: f32) -> Self {
        Self { dx, dy }
    }

    pub fn zero() -> Self {
        Self { dx: 0.0, dy: 0.0 }
    }
}


/// 动画帧插值组件 - 实现原版C#的OffSetMove机制
/// 参考: Client/MirObjects/PlayerObject.cs Line 864-1000
/// 
/// 精灵渲染组件 - 可渲染实体必备
#[derive(Debug, Clone)]
pub struct Sprite {
    pub library: i32,      // MLibrary 索引 (0=Tiles, 1=SmTiles, 2=Objects, etc.)
    pub index: i32,        // 贴图索引
    pub frame: i32,        // 当前帧
    pub blend_mode: SpriteBlendMode, // 混合模式
}

impl Sprite {
    pub fn new(library: i32, index: i32) -> Self {
        Self {
            library,
            index,
            frame: 0,
            blend_mode: SpriteBlendMode::Alpha,
        }
    }

    pub fn with_blend(library: i32, index: i32, blend_mode: SpriteBlendMode) -> Self {
        Self { library, index, frame: 0, blend_mode }
    }
}

/// 时间跟踪组件
#[derive(Debug, Clone)]
pub struct TimeTracker {
    pub animation_count: i32,
    pub frame_count: u64,
    pub fps: f32,
    pub last_fps_update: std::time::Instant,
    pub last_frame_time: std::time::Instant,
}

impl Default for TimeTracker {
    fn default() -> Self {
        Self {
            animation_count: 0,
            frame_count: 0,
            fps: 0.0,
            last_fps_update: std::time::Instant::now(),
            last_frame_time: std::time::Instant::now(),
        }
    }
}

/// 动画帧组件 - 存储由 AnimationSystem 计算的当前动画帧索引
/// 
/// **设计原则**: 分离动画逻辑和渲染逻辑
/// - AnimationSystem (逻辑层): 计算当前帧索引，更新此组件
/// - SpriteRenderSystem (渲染层): 读取此组件，渲染对应精灵
/// 
/// **数据流**:
/// ```
/// AnimationSystem 更新 → AnimationFrame.current_frame
///                           ↓
/// SpriteRenderSystem 读取 → 渲染精灵
/// ```
#[derive(Debug, Clone, Copy)]
pub struct AnimationFrame {
    /// 当前角色动画帧索引（身体、头发）
    pub character_frame: i32,
    /// 当前武器动画帧索引
    pub weapon_frame: i32,
}

impl AnimationFrame {
    pub fn new() -> Self {
        Self {
            character_frame: 0,
            weapon_frame: 0,
        }
    }
}

impl Default for AnimationFrame {
    fn default() -> Self {
        Self::new()
    }
}
