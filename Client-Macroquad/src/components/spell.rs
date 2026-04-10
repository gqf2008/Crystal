// ============================================================================
// 技能/魔法系统组件
// ============================================================================

pub use mir2_shared::{MirClass, Point};

/// 技能数据组件
#[derive(Debug, Clone)]
pub struct SpellData {
    pub spell_id: u16,
    pub caster_id: u32,
    pub target_pos: Point,
    pub power: i32,
}

/// 技能类型 (对应 C# Spell 枚举)
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[repr(u8)]
pub enum SpellType {
    None = 0,
    
    // Warrior (战士)
    Fencing = 1,
    Slaying = 2,
    Thrusting = 3,
    HalfMoon = 4,
    ShoulderDash = 5,
    TwinDrakeBlade = 6,
    Entrapment = 7,
    FlamingSword = 8,
    LionRoar = 9,
    CrossHalfMoon = 10,
    BladeAvalanche = 11,
    ProtectionField = 12,
    Rage = 13,
    CounterAttack = 14,
    SlashingBurst = 15,
    Fury = 16,
    ImmortalSkin = 17,
    
    // Wizard (法师)
    FireBall = 31,
    Repulsion = 32,
    ElectricShock = 33,
    GreatFireBall = 34,
    HellFire = 35,
    ThunderBolt = 36,
    Teleport = 37,
    FireBang = 38,
    FireWall = 39,
    Lightning = 40,
    FrostCrunch = 41,
    ThunderStorm = 42,
    MagicShield = 43,
    TurnUndead = 44,
    Vampirism = 45,
    IceStorm = 46,
    FlameDisruptor = 47,
    Mirroring = 48,
    FlameField = 49,
    Blizzard = 50,
    MagicBooster = 51,
    MeteorStrike = 52,
    IceThrust = 53,
    FastMove = 54,
    StormEscape = 55,
    
    // Taoist (道士)
    Healing = 61,
    SpiritSword = 62,
    Poisoning = 63,
    SoulFireBall = 64,
    SummonSkeleton = 65,
    Hiding = 67,
    MassHiding = 68,
    SoulShield = 69,
    Revelation = 70,
    BlessedArmour = 71,
    EnergyRepulsor = 72,
    TrapHexagon = 73,
    Purification = 74,
    MassHealing = 75,
    Hallucination = 76,
    UltimateEnhancer = 77,
    SummonShinsu = 78,
    Reincarnation = 79,
    SummonHolyDeva = 80,
    Curse = 81,
    Plague = 82,
    PoisonCloud = 83,
    EnergyShield = 84,
    PetEnhancer = 85,
    HealingCircle = 86,
    
    // Assassin (刺客)
    FatalSword = 91,
    DoubleSlash = 92,
    Haste = 93,
    FlashDash = 94,
    LightBody = 95,
    HeavenlySword = 96,
    FireBurst = 97,
    Trap = 98,
    PoisonSword = 99,
    MoonLight = 100,
    MPEater = 101,
    SwiftFeet = 102,
    DarkBody = 103,
    Hemorrhage = 104,
    CrescentSlash = 105,
    MoonMist = 106,
    CatTongue = 107,
    
    // Archer (弓箭手)
    Focus = 121,
    StraightShot = 122,
    DoubleShot = 123,
    ExplosiveTrap = 124,
    DelayedExplosion = 125,
    Meditation = 126,
    BackStep = 127,
    ElementalShot = 128,
    Concentration = 129,
    Stonetrap = 130,
    ElementalBarrier = 131,
    SummonVampire = 132,
    VampireShot = 133,
    SummonToad = 134,
    PoisonShot = 135,
    CrippleShot = 136,
    SummonSnakes = 137,
    NapalmShot = 138,
    OneWithNature = 139,
    BindingShot = 140,
    MentalState = 141,
}

impl SpellType {
    /// 获取技能名称
    pub fn name(&self) -> &'static str {
        match self {
            Self::None => "无",
            // Warrior
            Self::Fencing => "基本剑术",
            Self::Slaying => "攻杀剑术",
            Self::Thrusting => "刺杀剑术",
            Self::HalfMoon => "半月弯刀",
            Self::ShoulderDash => "野蛮冲撞",
            Self::LionRoar => "狮子吼",
            // Wizard
            Self::FireBall => "火球术",
            Self::GreatFireBall => "大火球",
            Self::HellFire => "地狱火",
            Self::ThunderBolt => "雷电术",
            Self::Teleport => "瞬息移动",
            Self::Lightning => "疾光电影",
            Self::MagicShield => "魔法盾",
            // Taoist
            Self::Healing => "治愈术",
            Self::SpiritSword => "精神力战法",
            Self::Poisoning => "施毒术",
            Self::SummonSkeleton => "召唤骷髅",
            Self::Hiding => "隐身术",
            Self::SoulShield => "幽灵盾",
            // Assassin
            Self::FatalSword => "致命剑术",
            Self::DoubleSlash => "双倍斩",
            Self::Haste => "加速",
            Self::FlashDash => "闪避",
            // Archer
            Self::Focus => "集中",
            Self::StraightShot => "直射",
            Self::DoubleShot => "双射",
            Self::Meditation => "冥想",
            _ => "未知技能"
        }
    }
    
    /// 获取技能所需职业
    pub fn required_class(&self) -> MirClass {
        let id = *self as u8;
        if id >= 1 && id <= 17 { MirClass::Warrior }
        else if id >= 31 && id <= 55 { MirClass::Wizard }
        else if id >= 61 && id <= 86 { MirClass::Taoist }
        else if id >= 91 && id <= 107 { MirClass::Assassin }
        else if id >= 121 && id <= 141 { MirClass::Archer }
        else { MirClass::Warrior } // 默认
    }
}

impl TryFrom<u8> for SpellType {
    type Error = ();

    fn try_from(value: u8) -> Result<Self, Self::Error> {
        Ok(match value {
            0 => Self::None,

            1 => Self::Fencing,
            2 => Self::Slaying,
            3 => Self::Thrusting,
            4 => Self::HalfMoon,
            5 => Self::ShoulderDash,
            6 => Self::TwinDrakeBlade,
            7 => Self::Entrapment,
            8 => Self::FlamingSword,
            9 => Self::LionRoar,
            10 => Self::CrossHalfMoon,
            11 => Self::BladeAvalanche,
            12 => Self::ProtectionField,
            13 => Self::Rage,
            14 => Self::CounterAttack,
            15 => Self::SlashingBurst,
            16 => Self::Fury,
            17 => Self::ImmortalSkin,

            31 => Self::FireBall,
            32 => Self::Repulsion,
            33 => Self::ElectricShock,
            34 => Self::GreatFireBall,
            35 => Self::HellFire,
            36 => Self::ThunderBolt,
            37 => Self::Teleport,
            38 => Self::FireBang,
            39 => Self::FireWall,
            40 => Self::Lightning,
            41 => Self::FrostCrunch,
            42 => Self::ThunderStorm,
            43 => Self::MagicShield,
            44 => Self::TurnUndead,
            45 => Self::Vampirism,
            46 => Self::IceStorm,
            47 => Self::FlameDisruptor,
            48 => Self::Mirroring,
            49 => Self::FlameField,
            50 => Self::Blizzard,
            51 => Self::MagicBooster,
            52 => Self::MeteorStrike,
            53 => Self::IceThrust,
            54 => Self::FastMove,
            55 => Self::StormEscape,

            61 => Self::Healing,
            62 => Self::SpiritSword,
            63 => Self::Poisoning,
            64 => Self::SoulFireBall,
            65 => Self::SummonSkeleton,
            67 => Self::Hiding,
            68 => Self::MassHiding,
            69 => Self::SoulShield,
            70 => Self::Revelation,
            71 => Self::BlessedArmour,
            72 => Self::EnergyRepulsor,
            73 => Self::TrapHexagon,
            74 => Self::Purification,
            75 => Self::MassHealing,
            76 => Self::Hallucination,
            77 => Self::UltimateEnhancer,
            78 => Self::SummonShinsu,
            79 => Self::Reincarnation,
            80 => Self::SummonHolyDeva,
            81 => Self::Curse,
            82 => Self::Plague,
            83 => Self::PoisonCloud,
            84 => Self::EnergyShield,
            85 => Self::PetEnhancer,
            86 => Self::HealingCircle,

            91 => Self::FatalSword,
            92 => Self::DoubleSlash,
            93 => Self::Haste,
            94 => Self::FlashDash,
            95 => Self::LightBody,
            96 => Self::HeavenlySword,
            97 => Self::FireBurst,
            98 => Self::Trap,
            99 => Self::PoisonSword,
            100 => Self::MoonLight,
            101 => Self::MPEater,
            102 => Self::SwiftFeet,
            103 => Self::DarkBody,
            104 => Self::Hemorrhage,
            105 => Self::CrescentSlash,
            106 => Self::MoonMist,
            107 => Self::CatTongue,

            121 => Self::Focus,
            122 => Self::StraightShot,
            123 => Self::DoubleShot,
            124 => Self::ExplosiveTrap,
            125 => Self::DelayedExplosion,
            126 => Self::Meditation,
            127 => Self::BackStep,
            128 => Self::ElementalShot,
            129 => Self::Concentration,
            130 => Self::Stonetrap,
            131 => Self::ElementalBarrier,
            132 => Self::SummonVampire,
            133 => Self::VampireShot,
            134 => Self::SummonToad,
            135 => Self::PoisonShot,
            136 => Self::CrippleShot,
            137 => Self::SummonSnakes,
            138 => Self::NapalmShot,
            139 => Self::OneWithNature,
            140 => Self::BindingShot,
            141 => Self::MentalState,

            _ => return Err(()),
        })
    }
}

/// 已学会的技能数据
#[derive(Debug, Clone)]
pub struct LearnedMagic {
    pub spell: SpellType,
    pub level: u8,        // 技能等级 (0-3)
    pub experience: u32,  // 技能经验
    pub key_slot: Option<u8>, // 绑定的快捷键槽位 (F1-F8)
}

impl LearnedMagic {
    pub fn new(spell: SpellType) -> Self {
        Self {
            spell,
            level: 0,
            experience: 0,
            key_slot: None,
        }
    }
}

/// 玩家已学技能列表组件
#[derive(Debug, Clone)]
pub struct MagicList {
    pub magics: Vec<LearnedMagic>,
}

impl MagicList {
    pub fn new() -> Self {
        Self { magics: Vec::new() }
    }
    
    /// 学会新技能
    pub fn learn(&mut self, spell: SpellType) -> bool {
        if self.has_learned(spell) {
            return false;
        }
        self.magics.push(LearnedMagic::new(spell));
        true
    }
    
    /// 是否已学会某技能
    pub fn has_learned(&self, spell: SpellType) -> bool {
        self.magics.iter().any(|m| m.spell == spell)
    }
    
    /// 获取技能
    pub fn get_mut(&mut self, spell: SpellType) -> Option<&mut LearnedMagic> {
        self.magics.iter_mut().find(|m| m.spell == spell)
    }
    
    /// 获取绑定到某槽位的技能
    pub fn get_by_slot(&self, slot: u8) -> Option<&LearnedMagic> {
        self.magics.iter().find(|m| m.key_slot == Some(slot))
    }
}

impl Default for MagicList {
    fn default() -> Self {
        Self::new()
    }
}

/// 可学习技能列表组件 (NPC 提供或职业默认)
#[derive(Debug, Clone)]
pub struct LearnableMagicList {
    pub spells: Vec<(SpellType, u16)>, // (技能, 所需等级)
}

impl LearnableMagicList {
    pub fn new() -> Self {
        Self { spells: Vec::new() }
    }
    
    /// 添加可学技能
    pub fn add(&mut self, spell: SpellType, required_level: u16) {
        self.spells.push((spell, required_level));
    }
    
    /// 获取玩家当前可学习的技能
    pub fn get_available(&self, player_level: u16, learned: &MagicList) -> Vec<SpellType> {
        self.spells.iter()
            .filter(|(spell, req_level)| {
                *req_level <= player_level && !learned.has_learned(*spell)
            })
            .map(|(spell, _)| *spell)
            .collect()
    }
    
    /// 为职业初始化默认可学技能
    pub fn init_for_class(class: MirClass) -> Self {
        let mut list = Self::new();
        match class {
            MirClass::Warrior => {
                list.add(SpellType::Fencing, 7);
                list.add(SpellType::Slaying, 15);
                list.add(SpellType::Thrusting, 22);
                list.add(SpellType::HalfMoon, 28);
                list.add(SpellType::ShoulderDash, 30);
                list.add(SpellType::LionRoar, 36);
            },
            MirClass::Wizard => {
                list.add(SpellType::FireBall, 7);
                list.add(SpellType::Repulsion, 12);
                list.add(SpellType::ElectricShock, 13);
                list.add(SpellType::GreatFireBall, 15);
                list.add(SpellType::HellFire, 19);
                list.add(SpellType::ThunderBolt, 22);
                list.add(SpellType::Teleport, 25);
                list.add(SpellType::Lightning, 29);
                list.add(SpellType::MagicShield, 31);
            },
            MirClass::Taoist => {
                list.add(SpellType::Healing, 7);
                list.add(SpellType::SpiritSword, 9);
                list.add(SpellType::Poisoning, 14);
                list.add(SpellType::SoulFireBall, 18);
                list.add(SpellType::SummonSkeleton, 19);
                list.add(SpellType::Hiding, 20);
                list.add(SpellType::SoulShield, 24);
            },
            MirClass::Assassin => {
                list.add(SpellType::FatalSword, 7);
                list.add(SpellType::DoubleSlash, 15);
                list.add(SpellType::Haste, 20);
                list.add(SpellType::FlashDash, 25);
            },
            MirClass::Archer => {
                list.add(SpellType::Focus, 7);
                list.add(SpellType::StraightShot, 9);
                list.add(SpellType::DoubleShot, 15);
                list.add(SpellType::Meditation, 20);
            },
        }
        list
    }
}

impl Default for LearnableMagicList {
    fn default() -> Self {
        Self::new()
    }
}

/// 技能冷却组件
///
/// 存储多个技能的冷却时间，由 NetworkApplySystem 根据 MagicDelayReceived 设置
#[derive(Debug, Clone)]
pub struct SpellCooldowns {
    /// (skill_id, cooldown_end_time_ms)
    pub active_cooldowns: std::collections::HashMap<u8, u64>,
}

impl SpellCooldowns {
    pub fn new() -> Self {
        Self {
            active_cooldowns: std::collections::HashMap::new(),
        }
    }

    /// 设置技能冷却
    pub fn set(&mut self, spell_id: u8, duration_ms: u32) {
        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_millis() as u64;
        self.active_cooldowns.insert(spell_id, now + duration_ms as u64);
    }

    /// 检查技能是否在冷却中
    pub fn is_on_cooldown(&self, spell_id: u8) -> bool {
        if let Some(&end_time) = self.active_cooldowns.get(&spell_id) {
            let now = std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap_or_default()
                .as_millis() as u64;
            if now >= end_time {
                return false;
            }
            true
        } else {
            false
        }
    }

    /// 清理已过期的冷却
    pub fn cleanup(&mut self) {
        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_millis() as u64;
        self.active_cooldowns.retain(|_, &mut v| v > now);
    }

    /// 获取某技能的剩余冷却时间（毫秒）
    pub fn remaining_ms(&self, spell_id: u8) -> u64 {
        if let Some(&end_time) = self.active_cooldowns.get(&spell_id) {
            let now = std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap_or_default()
                .as_millis() as u64;
            end_time.saturating_sub(now)
        } else {
            0
        }
    }
}

impl Default for SpellCooldowns {
    fn default() -> Self {
        Self::new()
    }
}
