// Intelligent Creature (宠物) 数据结构
// 纯数据结构，由 WorldActor 调用

/// 宠物类型（对应 mir2_shared::IntelligentCreatureType）
#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub enum CreatureType {
    None = 0,
    BabyPanda = 1,
    BabyPig = 2,
    BabyOma = 3,
    BabySkeleton = 4,
    BabyKitten = 5,
    BabyChicken = 6,
    BabySheep = 7,
    BabyGorilla = 8,
    BabyBabyDragon = 9,
    Custom = 100,
}

impl From<u8> for CreatureType {
    fn from(v: u8) -> Self {
        match v {
            0 => CreatureType::None,
            1 => CreatureType::BabyPanda,
            2 => CreatureType::BabyPig,
            3 => CreatureType::BabyOma,
            4 => CreatureType::BabySkeleton,
            5 => CreatureType::BabyKitten,
            6 => CreatureType::BabyChicken,
            7 => CreatureType::BabySheep,
            8 => CreatureType::BabyGorilla,
            9 => CreatureType::BabyBabyDragon,
            _ => CreatureType::Custom,
        }
    }
}

/// 拾取模式
#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub enum PickupMode {
    None = 0,
    GoldOnly = 1,
    GoldAndItem = 2,
    All = 3,
}

impl From<u8> for PickupMode {
    fn from(v: u8) -> Self {
        match v {
            0 => PickupMode::None,
            1 => PickupMode::GoldOnly,
            2 => PickupMode::GoldAndItem,
            _ => PickupMode::All,
        }
    }
}

impl From<PickupMode> for u8 {
    fn from(mode: PickupMode) -> Self {
        mode as u8
    }
}

/// 宠物实例
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct IntelligentCreature {
    /// 宠物类型
    pub creature_type: CreatureType,
    /// 自定义名称
    pub custom_name: Option<String>,
    /// 拾取模式
    pub pickup_mode: PickupMode,
    /// 饥饿值（0-100，低于20时停止工作）
    pub hunger: u8,
    /// 是否启用
    pub enabled: bool,
}

impl IntelligentCreature {
    pub fn new(creature_type: CreatureType) -> Self {
        Self {
            creature_type,
            custom_name: None,
            pickup_mode: PickupMode::None,
            hunger: 100,
            enabled: false,
        }
    }

    /// 饥饿值随时间减少
    pub fn tick_hunger(&mut self, dt_seconds: u32) {
        // 每分钟减少 1 点饥饿值
        self.hunger = self.hunger.saturating_sub((dt_seconds / 60) as u8);
    }

    /// 恢复饥饿值
    pub fn restore_hunger(&mut self, amount: u8) {
        self.hunger = (self.hunger + amount).min(100);
    }

    /// 是否因饥饿无法工作
    pub fn is_starving(&self) -> bool {
        self.hunger < 20
    }
}

/// 玩家宠物信息
#[derive(Debug, Clone, Default)]
pub struct CreatureLog {
    /// 当前激活的宠物
    pub active_creature: Option<IntelligentCreature>,
    /// 已拥有的宠物列表
    pub owned_creatures: Vec<IntelligentCreature>,
    /// 是否请求更新
    pub request_updates: bool,
}

impl CreatureLog {
    pub fn new() -> Self {
        Self::default()
    }

    /// 设置宠物
    pub fn set_creature(&mut self, creature: IntelligentCreature) {
        self.active_creature = Some(creature);
    }

    /// 更新宠物拾取模式
    pub fn update_pickup_mode(&mut self, mode: PickupMode) {
        if let Some(c) = &mut self.active_creature {
            c.pickup_mode = mode;
        }
    }

    /// 更新宠物饥饿值
    pub fn tick(&mut self, dt_seconds: u32) {
        if let Some(c) = &mut self.active_creature {
            c.tick_hunger(dt_seconds);
        }
    }

    /// 喂养宠物（恢复饥饿值）
    pub fn restore_hunger(&mut self, amount: u8) {
        if let Some(c) = &mut self.active_creature {
            c.hunger = (c.hunger + amount).min(100);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_creature_type_from() {
        assert_eq!(CreatureType::from(0u8), CreatureType::None);
        assert_eq!(CreatureType::from(1u8), CreatureType::BabyPanda);
        assert_eq!(CreatureType::from(5u8), CreatureType::BabyKitten);
        assert_eq!(CreatureType::from(200u8), CreatureType::Custom);
    }

    #[test]
    fn test_pickup_mode_from() {
        assert_eq!(PickupMode::from(0u8), PickupMode::None);
        assert_eq!(PickupMode::from(1u8), PickupMode::GoldOnly);
        assert_eq!(PickupMode::from(2u8), PickupMode::GoldAndItem);
        assert_eq!(PickupMode::from(3u8), PickupMode::All);
    }

    #[test]
    fn test_hunger_tick() {
        let mut c = IntelligentCreature::new(CreatureType::BabyPanda);
        assert_eq!(c.hunger, 100);
        c.tick_hunger(60); // 1 minute
        assert_eq!(c.hunger, 99);
        c.tick_hunger(5940); // 99 minutes more
        assert_eq!(c.hunger, 0);
    }

    #[test]
    fn test_is_starving() {
        let mut c = IntelligentCreature::new(CreatureType::BabyPanda);
        assert!(!c.is_starving());
        c.hunger = 19;
        assert!(c.is_starving());
        c.hunger = 20;
        assert!(!c.is_starving());
    }

    #[test]
    fn test_feed() {
        let mut c = IntelligentCreature::new(CreatureType::BabyPanda);
        c.hunger = 10;
        c.restore_hunger(50);
        assert_eq!(c.hunger, 60);
        c.restore_hunger(50); // should cap at 100
        assert_eq!(c.hunger, 100);
    }

    #[test]
    fn test_creature_log() {
        let mut log = CreatureLog::new();
        assert!(log.active_creature.is_none());

        log.set_creature(IntelligentCreature::new(CreatureType::BabyPanda));
        assert!(log.active_creature.is_some());

        log.update_pickup_mode(PickupMode::GoldAndItem);
        assert_eq!(log.active_creature.as_ref().unwrap().pickup_mode, PickupMode::GoldAndItem);

        log.restore_hunger(30);
        assert_eq!(log.active_creature.as_ref().unwrap().hunger, 100);
    }

    #[test]
    fn test_log_tick() {
        let mut log = CreatureLog::new();
        // No creature - should not panic
        log.tick(600);
        assert!(log.active_creature.is_none());

        log.set_creature(IntelligentCreature::new(CreatureType::BabyPanda));
        log.tick(600); // 10 minutes = 10 hunger loss
        assert_eq!(log.active_creature.as_ref().unwrap().hunger, 90);

        log.tick(5400); // 90 more minutes
        assert_eq!(log.active_creature.as_ref().unwrap().hunger, 0);

        // Should not underflow
        log.tick(3600);
        assert_eq!(log.active_creature.as_ref().unwrap().hunger, 0);
    }
}
