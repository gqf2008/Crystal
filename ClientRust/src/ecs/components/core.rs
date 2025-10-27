// ============================================================================
// 核心组件 - 所有实体的基础组件
// ============================================================================

pub use mir2_shared::{MirDirection, MirAction, MirClass, MirGender};
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
/// 核心原理:
/// - `current_grid`: 服务器确认的当前格子位置（离散）
/// - `movement_grid`: 移动目标格子位置（离散）
/// - `offset_move`: 动画帧进度插值（连续像素偏移）
/// - 绘制位置 = movement_grid * CELL_SIZE - offset_move
/// 
/// 这样避免了客户端预测修改Position，完全依赖服务器同步
#[derive(Debug, Clone, Copy)]
pub struct MovementAnimation {
    /// 当前格子位置（服务器确认的）
    pub current_grid: (i32, i32),
    
    /// 移动目标格子位置（CurrentLocation + Direction * Distance）
    pub movement_grid: (i32, i32),
    
    /// 动画帧插值偏移（像素）
    /// 计算公式: (frame_index / frame_count) * distance * CELL_SIZE
    pub offset_move: (f32, f32),
    
    /// 移动距离（格子数）: Walk=1, Run=2, Mount=3
    pub move_distance: i32,
}

impl MovementAnimation {
    pub fn new(grid_x: i32, grid_y: i32) -> Self {
        Self {
            current_grid: (grid_x, grid_y),
            movement_grid: (grid_x, grid_y),
            offset_move: (0.0, 0.0),
            move_distance: 0,
        }
    }
    
    /// 开始移动动画
    pub fn start_move(&mut self, target_grid: (i32, i32), distance: i32) {
        self.movement_grid = target_grid;
        self.move_distance = distance;
        // offset_move 将在 update_offset 中根据动画帧计算
    }
    
    /// 更新动画帧插值（每帧调用）
    /// 参考: PlayerObject.cs Line 906-932
    pub fn update_offset(&mut self, frame_index: i32, frame_count: i32, direction: u8) {
        if self.move_distance == 0 || frame_count == 0 {
            self.offset_move = (0.0, 0.0);
            return;
        }
        
        const CELL_WIDTH: f32 = 48.0;
        const CELL_HEIGHT: f32 = 32.0;
        
        let progress = (frame_index + 1) as f32 / frame_count as f32;
        let distance = self.move_distance as f32;
        
        // 根据方向计算偏移（原版C#逻辑）
        self.offset_move = match direction {
            0 => (0.0, CELL_HEIGHT * distance * progress),           // Up
            1 => (CELL_WIDTH * distance * progress, CELL_HEIGHT * distance * progress), // UpRight
            2 => (CELL_WIDTH * distance * progress, 0.0),            // Right
            3 => (CELL_WIDTH * distance * progress, -CELL_HEIGHT * distance * progress), // DownRight
            4 => (0.0, -CELL_HEIGHT * distance * progress),          // Down
            5 => (-CELL_WIDTH * distance * progress, -CELL_HEIGHT * distance * progress), // DownLeft
            6 => (-CELL_WIDTH * distance * progress, 0.0),           // Left
            7 => (-CELL_WIDTH * distance * progress, CELL_HEIGHT * distance * progress), // UpLeft
            _ => (0.0, 0.0),
        };
    }
    
    /// 完成移动（动画播完或服务器确认）
    pub fn finish_move(&mut self, confirmed_grid: (i32, i32)) {
        self.current_grid = confirmed_grid;
        self.movement_grid = confirmed_grid;
        self.offset_move = (0.0, 0.0);
        self.move_distance = 0;
    }
}

/// 方向组件
#[derive(Debug, Clone, Copy)]
pub struct Direction {
    pub current: MirDirection,
    pub target: MirDirection,
}

impl Direction {
    pub fn new(dir: MirDirection) -> Self {
        Self { current: dir, target: dir }
    }
}

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

/// 动画状态组件
#[derive(Debug, Clone)]
pub struct Animation {
    pub action: MirAction,
    pub direction: u8,       // 方向 0-7
    pub frame_count: u8,
    pub frame_index: u8,
    pub frame_interval: u32, // 毫秒
    pub frame_timer: u32,
    pub loop_animation: bool,
}

impl Animation {
    pub fn new(action: MirAction, frame_count: u8, frame_interval: u32) -> Self {
        Self {
            action,
            direction: 0,    // 默认朝右
            frame_count,
            frame_index: 0,
            frame_interval,
            frame_timer: 0,
            loop_animation: true,
        }
    }

    /// 更新动画 (返回 true 表示播放完成)
    pub fn update(&mut self, delta_ms: u32) -> bool {
        self.frame_timer += delta_ms;
        if self.frame_timer >= self.frame_interval {
            self.frame_timer = 0;
            self.frame_index += 1;

            if self.frame_index >= self.frame_count {
                if self.loop_animation {
                    self.frame_index = 0;
                } else {
                    self.frame_index = self.frame_count - 1;
                    return true; // 动画完成
                }
            }
        }
        false
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
