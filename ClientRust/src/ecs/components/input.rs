// ============================================================================
// 输入相关组件
// ============================================================================

use std::time::Instant;

// ============================================================================
// 事件类型定义
// ============================================================================

#[derive(Debug, Clone)]
pub enum InputEvent {
    // KeyDown {
    //     keycode: KeyCode,
    //     repeat: bool, // 是否是重复按键
    //     text: Option<SmolStr>,
    //     timestamp: std::time::Instant,
    // },
    // KeyUp {
    //     keycode: KeyCode,
    //     text: Option<SmolStr>,
    //     timestamp: std::time::Instant,
    // },
    MouseMove {
        x: f32,
        y: f32,
        dx: f32,
        dy: f32,
    },
    // /// 鼠标按钮按下
    // MouseDown {
    //     button: MouseButton,
    //     x: f32,
    //     y: f32,
    // },
    // /// 鼠标按钮释放
    // MouseUp {
    //     button: MouseButton,
    //     x: f32,
    //     y: f32,
    // },
    /// 鼠标滚轮
    MouseWheel {
        x: f32,
        y: f32,
    },
    /// 鼠标进入/离开窗口
    MouseEnterOrLeave {
        entered: bool,
    },
    Ime {
        character: char,
        timestamp: std::time::Instant,
    },
    Resize {
        width: f32,
        height: f32,
    },
}


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

// ============================================================================
// 玩家输入组件 - 存储玩家的输入意图（符合ECS架构）
// ============================================================================

use mir2_shared::enums::MirDirection;
use crate::ecs::components::SpellType;

/// 移动模式
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MovementMode {
    /// 无移动
    None,
    /// 自动寻路(双击) - 计算完整路径,松开后继续走
    Pathfinding,
    /// 直接跟随(长按) - 直线移动,不避障,松开立即停止
    DirectFollow,
}

impl Default for MovementMode {
    fn default() -> Self {
        Self::None
    }
}

/// 玩家输入组件 - 存储玩家的输入意图
#[derive(Debug, Clone)]
pub struct PlayerInput {
    /// 移动目标（世界坐标）
    pub move_to: Option<(f32, f32)>,
    
    /// 移动模式
    pub movement_mode: MovementMode,
    
    /// 攻击目标实体
    pub attack_target: Option<hecs::Entity>,
    
    /// 施放技能
    pub cast_spell: Option<SpellType>,
    
    /// 施法目标位置
    pub spell_target_pos: Option<(f32, f32)>,
    
    /// 施法目标实体
    pub spell_target_entity: Option<hecs::Entity>,
    
    /// 拾取物品位置
    pub pickup_at: Option<(i32, i32)>,
    
    /// 转向方向
    pub turn_to: Option<MirDirection>,
}

impl PlayerInput {
    pub fn new() -> Self {
        Self {
            move_to: None,
            movement_mode: MovementMode::None,
            attack_target: None,
            cast_spell: None,
            spell_target_pos: None,
            spell_target_entity: None,
            pickup_at: None,
            turn_to: None,
        }
    }
    
    /// 清除所有输入
    pub fn clear(&mut self) {
        self.move_to = None;
        self.attack_target = None;
        self.cast_spell = None;
        self.spell_target_pos = None;
        self.spell_target_entity = None;
        self.pickup_at = None;
        self.turn_to = None;
    }
    
    /// 设置移动指令（寻路模式）
    pub fn set_move(&mut self, target: (f32, f32)) {
        self.move_to = Some(target);
        self.movement_mode = MovementMode::Pathfinding;
    }
    
    /// 设置直接跟随指令（不使用寻路）
    pub fn set_follow(&mut self, target: (f32, f32)) {
        self.move_to = Some(target);
        self.movement_mode = MovementMode::DirectFollow;
    }
    
    /// 设置攻击指令
    pub fn set_attack(&mut self, target: hecs::Entity) {
        self.attack_target = Some(target);
    }
    
    /// 设置施法指令
    pub fn set_cast_spell(&mut self, spell: SpellType, target_pos: Option<(f32, f32)>, target_entity: Option<hecs::Entity>) {
        self.cast_spell = Some(spell);
        self.spell_target_pos = target_pos;
        self.spell_target_entity = target_entity;
    }
    
    /// 是否有任何输入
    pub fn has_input(&self) -> bool {
        self.move_to.is_some()
            || self.attack_target.is_some()
            || self.cast_spell.is_some()
            || self.pickup_at.is_some()
            || self.turn_to.is_some()
    }
}

impl Default for PlayerInput {
    fn default() -> Self {
        Self::new()
    }
}

// ============================================================================
// 输入状态组件 - 统一管理输入的边缘检测
// ============================================================================

/// 输入状态组件 - 追踪上一帧的键盘和鼠标状态
/// 
/// **设计目标**：
/// - ✅ 单一职责：专门负责输入状态追踪
/// - ✅ 逻辑内聚：所有边缘检测逻辑集中在 InputStateSystem
/// - ✅ 可复用：所有需要边缘检测的 System 都可以查询此 Component
/// - ✅ 符合 ECS：状态存储在 Component 中，而非 System 或 Context
/// 
/// **使用者**：
/// - `DebugSystem` - 调试热键边缘检测
/// - `PlayerControlSystem` - 鼠标点击边缘检测（双击、单击）
/// - `CameraSystem` - 拖拽开始检测
/// - UI 系统 - 按钮点击检测
/// 
/// **更新时机**：
/// - 由 `InputStateSystem` 在每帧开始时更新（优先级 10）
/// - 在所有其他输入处理系统之前执行
#[derive(Debug, Clone, Default)]
pub struct InputState {
    /// 上一帧按下的按键集合
    pub prev_pressed_keys: std::collections::HashSet<ggez::input::keyboard::KeyCode>,
    
    /// 上一帧鼠标左键是否按下
    pub prev_mouse_left: bool,
    
    /// 上一帧鼠标右键是否按下
    pub prev_mouse_right: bool,
    
    /// 上一帧鼠标中键是否按下
    pub prev_mouse_middle: bool,
}
