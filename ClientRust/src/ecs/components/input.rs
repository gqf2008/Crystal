// ============================================================================
// 输入相关组件
// ============================================================================

use std::time::Instant;

/// 拖拽组件 - 鼠标拖拽状态
#[derive(Debug, Clone)]
pub struct Draggable {
    pub is_dragging: bool,
    pub drag_start_x: f32,
    pub drag_start_y: f32,
    pub drag_start_pos_x: f32,
    pub drag_start_pos_y: f32,
}

impl Default for Draggable {
    fn default() -> Self {
        Self {
            is_dragging: false,
            drag_start_x: 0.0,
            drag_start_y: 0.0,
            drag_start_pos_x: 0.0,
            drag_start_pos_y: 0.0,
        }
    }
}

/// 鼠标输入状态组件
#[derive(Debug, Clone)]
pub struct MouseInput {
    pub left_pressed: bool,
    pub right_pressed: bool,
    pub left_double_clicked: bool,
    pub right_double_clicked: bool,
    pub left_press_time: i32,
    pub right_press_time: i32,
    pub left_last_click_time: Instant,
    pub right_last_click_time: Instant,
    pub x: f32,
    pub y: f32,
}

impl Default for MouseInput {
    fn default() -> Self {
        Self {
            left_pressed: false,
            right_pressed: false,
            left_double_clicked: false,
            right_double_clicked: false,
            left_press_time: 0,
            right_press_time: 0,
            left_last_click_time: Instant::now(),
            right_last_click_time: Instant::now(),
            x: 0.0,
            y: 0.0,
        }
    }
}

/// 目标类型
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TargetType {
    None,
    Monster(u32),      // 怪物实体 ID
    Player(u32),       // 玩家实体 ID
    NPC(u32),          // NPC 实体 ID
    Location(i32, i32), // 地面位置 (x, y)
}

/// 当前选中的目标组件
#[derive(Debug, Clone, Copy)]
pub struct TargetSelection {
    pub current: TargetType,
    pub last_update: Instant,
}

impl TargetSelection {
    pub fn new() -> Self {
        Self {
            current: TargetType::None,
            last_update: Instant::now(),
        }
    }
    
    pub fn select_monster(&mut self, id: u32) {
        self.current = TargetType::Monster(id);
        self.last_update = Instant::now();
    }
    
    pub fn select_player(&mut self, id: u32) {
        self.current = TargetType::Player(id);
        self.last_update = Instant::now();
    }
    
    pub fn select_location(&mut self, x: i32, y: i32) {
        self.current = TargetType::Location(x, y);
        self.last_update = Instant::now();
    }
    
    pub fn clear(&mut self) {
        self.current = TargetType::None;
        self.last_update = Instant::now();
    }
    
    pub fn has_target(&self) -> bool {
        !matches!(self.current, TargetType::None)
    }
    
    pub fn get_monster_id(&self) -> Option<u32> {
        match self.current {
            TargetType::Monster(id) => Some(id),
            _ => None,
        }
    }
    
    pub fn get_location(&self) -> Option<(i32, i32)> {
        match self.current {
            TargetType::Location(x, y) => Some((x, y)),
            _ => None,
        }
    }
}

impl Default for TargetSelection {
    fn default() -> Self {
        Self::new()
    }
}
