// player_movement_fsm.rs - 角色移动有限状态机
// 处理平滑的格子移动和动画

use std::time::{Duration, Instant};
use mir2_shared::{enums::MirDirection, Point};

/// 移动状态
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MovementState {
    /// 静止状态
    Idle,
    /// 正在移动到目标格子
    Moving {
        /// 移动开始时间
        start_time: Instant,
        /// 移动持续时间(毫秒)
        duration_ms: u64,
    },
}

/// 角色移动状态机
#[derive(Debug, Clone)]
pub struct PlayerMovementFSM {
    /// 当前状态
    pub state: MovementState,
    
    /// 当前所在格子(逻辑位置)
    pub current_cell: Point,
    
    /// 目标格子
    pub target_cell: Point,
    
    /// 渲染起始格子(用于插值计算)
    pub render_start_cell: Point,
    
    /// 移动方向
    pub direction: MirDirection,
    
    /// 是否在跑步(跑步速度更快)
    pub running: bool,
    
    /// 行走速度(毫秒/格)
    pub walk_speed_ms: u64,
    
    /// 跑步速度(毫秒/格)
    pub run_speed_ms: u64,
}

impl Default for PlayerMovementFSM {
    fn default() -> Self {
        Self {
            state: MovementState::Idle,
            current_cell: Point { x: 0, y: 0 },
            target_cell: Point { x: 0, y: 0 },
            render_start_cell: Point { x: 0, y: 0 },
            direction: MirDirection::Up,
            running: false,
            walk_speed_ms: 600,  // 600ms走一格
            run_speed_ms: 300,   // 300ms跑一格
        }
    }
}

impl PlayerMovementFSM {
    /// 创建新的移动状态机
    pub fn new(spawn_point: Point) -> Self {
        Self {
            current_cell: spawn_point,
            target_cell: spawn_point,
            render_start_cell: spawn_point,
            ..Default::default()
        }
    }
    
    /// 设置目标位置(开始移动)
    /// 
    /// # 参数
    /// * `target` - 目标格子坐标
    /// * `direction` - 移动方向
    /// * `running` - 是否跑步
    /// 
    /// # 返回
    /// * `true` - 成功开始移动
    /// * `false` - 已经在移动中或已到达目标
    pub fn move_to(&mut self, target: Point, direction: MirDirection, running: bool) -> bool {
        // 如果已经在目标位置,不需要移动
        if self.current_cell == target {
            return false;
        }
        
        // 更新状态
        self.target_cell = target;
        self.direction = direction;
        self.running = running;
        
        // 如果当前是静止状态,记录渲染起始点
        if matches!(self.state, MovementState::Idle) {
            self.render_start_cell = self.current_cell;
        }
        
        // 计算移动时间
        let duration_ms = if running {
            self.run_speed_ms
        } else {
            self.walk_speed_ms
        };
        
        // 切换到移动状态
        self.state = MovementState::Moving {
            start_time: Instant::now(),
            duration_ms,
        };
        
        true
    }
    
    /// 停止移动(立即停在当前格子)
    pub fn stop(&mut self) {
        self.state = MovementState::Idle;
        self.target_cell = self.current_cell;
        self.render_start_cell = self.current_cell;
    }
    
    /// 更新状态机(每帧调用)
    /// 
    /// # 返回
    /// * `true` - 完成了一次格子移动
    /// * `false` - 仍在移动或静止中
    pub fn update(&mut self) -> bool {
        match self.state {
            MovementState::Idle => false,
            
            MovementState::Moving { start_time, duration_ms } => {
                let elapsed = start_time.elapsed().as_millis() as u64;
                
                // 检查是否完成当前格子的移动
                if elapsed >= duration_ms {
                    // 移动到下一格
                    self.current_cell = self.get_next_cell();
                    self.render_start_cell = self.current_cell;
                    
                    // 检查是否到达最终目标
                    if self.current_cell == self.target_cell {
                        // 到达目标,停止
                        self.state = MovementState::Idle;
                    } else {
                        // 继续向目标移动
                        let new_duration = if self.running {
                            self.run_speed_ms
                        } else {
                            self.walk_speed_ms
                        };
                        
                        self.state = MovementState::Moving {
                            start_time: Instant::now(),
                            duration_ms: new_duration,
                        };
                    }
                    
                    return true; // 完成了一次移动
                }
                
                false
            }
        }
    }
    
    /// 计算下一个格子位置(向目标方向移动一格)
    fn get_next_cell(&self) -> Point {
        let mut next = self.current_cell;
        
        match self.direction {
            MirDirection::Up => next.y -= 1,
            MirDirection::Down => next.y += 1,
            MirDirection::Left => next.x -= 1,
            MirDirection::Right => next.x += 1,
            MirDirection::UpLeft => {
                next.x -= 1;
                next.y -= 1;
            }
            MirDirection::UpRight => {
                next.x += 1;
                next.y -= 1;
            }
            MirDirection::DownLeft => {
                next.x -= 1;
                next.y += 1;
            }
            MirDirection::DownRight => {
                next.x += 1;
                next.y += 1;
            }
        }
        
        next
    }
    
    /// 获取当前渲染偏移(像素)
    /// 
    /// # 参数
    /// * `cell_width` - 格子宽度(像素)
    /// * `cell_height` - 格子高度(像素)
    /// 
    /// # 返回
    /// * `(offset_x, offset_y)` - 从 render_start_cell 的偏移量
    pub fn get_render_offset(&self, cell_width: i32, cell_height: i32) -> (i32, i32) {
        match self.state {
            MovementState::Idle => (0, 0),
            
            MovementState::Moving { start_time, duration_ms } => {
                let elapsed = start_time.elapsed().as_millis() as u64;
                let progress = (elapsed as f32 / duration_ms as f32).min(1.0);
                
                // 计算目标格子相对于起始格子的偏移
                let next_cell = self.get_next_cell();
                let dx = next_cell.x - self.render_start_cell.x;
                let dy = next_cell.y - self.render_start_cell.y;
                
                // 计算插值后的像素偏移
                let offset_x = (dx * cell_width) as f32 * progress;
                let offset_y = (dy * cell_height) as f32 * progress;
                
                (offset_x as i32, offset_y as i32)
            }
        }
    }
    
    /// 获取渲染位置(世界坐标像素)
    /// 
    /// # 参数
    /// * `cell_width` - 格子宽度(像素)
    /// * `cell_height` - 格子高度(像素)
    /// 
    /// # 返回
    /// * `(world_x, world_y)` - 角色中心的世界坐标
    pub fn get_world_position(&self, cell_width: i32, cell_height: i32) -> (f32, f32) {
        // 起始格子的世界坐标
        let base_x = (self.render_start_cell.x * cell_width) as f32;
        let base_y = (self.render_start_cell.y * cell_height) as f32;
        
        // 加上移动偏移
        let (offset_x, offset_y) = self.get_render_offset(cell_width, cell_height);
        
        (base_x + offset_x as f32, base_y + offset_y as f32)
    }
    
    /// 是否正在移动
    pub fn is_moving(&self) -> bool {
        matches!(self.state, MovementState::Moving { .. })
    }
    
    /// 是否静止
    pub fn is_idle(&self) -> bool {
        matches!(self.state, MovementState::Idle)
    }
    
    /// 获取移动进度(0.0 - 1.0)
    pub fn get_progress(&self) -> f32 {
        match self.state {
            MovementState::Idle => 1.0,
            MovementState::Moving { start_time, duration_ms } => {
                let elapsed = start_time.elapsed().as_millis() as u64;
                (elapsed as f32 / duration_ms as f32).min(1.0)
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_movement_fsm_basic() {
        let mut fsm = PlayerMovementFSM::new(Point { x: 10, y: 10 });
        
        assert!(fsm.is_idle());
        assert_eq!(fsm.current_cell, Point { x: 10, y: 10 });
        
        // 开始移动
        let moved = fsm.move_to(Point { x: 12, y: 12 }, MirDirection::DownRight, false);
        assert!(moved);
        assert!(fsm.is_moving());
    }
    
    #[test]
    fn test_movement_direction() {
        let mut fsm = PlayerMovementFSM::new(Point { x: 10, y: 10 });
        
        fsm.move_to(Point { x: 10, y: 11 }, MirDirection::Down, false);
        assert_eq!(fsm.get_next_cell(), Point { x: 10, y: 11 });
        
        fsm.move_to(Point { x: 11, y: 10 }, MirDirection::Right, false);
        assert_eq!(fsm.get_next_cell(), Point { x: 11, y: 10 });
    }
}
