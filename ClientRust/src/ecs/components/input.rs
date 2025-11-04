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
