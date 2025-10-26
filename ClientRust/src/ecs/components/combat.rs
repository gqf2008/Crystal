// ============================================================================
// 战斗相关组件
// ============================================================================

pub use mir2_shared::{MirClass, MirGender};

/// 生命值组件
#[derive(Debug, Clone, Copy)]
pub struct Health {
    pub current: i32,
    pub max: i32,
}

impl Health {
    pub fn new(max: i32) -> Self {
        Self { current: max, max }
    }

    pub fn is_alive(&self) -> bool {
        self.current > 0
    }

    pub fn take_damage(&mut self, damage: i32) {
        self.current = (self.current - damage).max(0);
    }

    pub fn heal(&mut self, amount: i32) {
        self.current = (self.current + amount).min(self.max);
    }
}

/// 魔法值组件
#[derive(Debug, Clone, Copy)]
pub struct Mana {
    pub current: i32,
    pub max: i32,
}

impl Mana {
    pub fn new(max: i32) -> Self {
        Self { current: max, max }
    }

    pub fn has_enough(&self, cost: i32) -> bool {
        self.current >= cost
    }

    pub fn consume(&mut self, cost: i32) -> bool {
        if self.current >= cost {
            self.current -= cost;
            true
        } else {
            false
        }
    }

    pub fn restore(&mut self, amount: i32) {
        self.current = (self.current + amount).min(self.max);
    }

    pub fn percent(&self) -> f32 {
        if self.max > 0 {
            self.current as f32 / self.max as f32
        } else {
            0.0
        }
    }
}

/// 战斗属性组件 (玩家/怪物)
#[derive(Debug, Clone)]
pub struct CombatStats {
    pub level: u16,
    pub attack_min: i32,
    pub attack_max: i32,
    pub defense: i32,
    pub magic_defense: i32,
    pub accuracy: u8,
    pub agility: u8,
}
