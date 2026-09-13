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

/// 宠物规则（C# `IntelligentCreatureRules`；随 `S.UpdateIntelligentCreatureList` 下发给客户端，
/// 客户端据此渲染 `CreatureInfo`/`CreatureInfo1`/`CreatureInfo2` 三行）。
///
/// 数据源：C# 服务端静态表 `Server/MirDatabase/IntelligentCreatureInfo.cs:30-44`
/// （每行含 Icon/MinimalFullness/MousePickup*/AutoPickup*/SemiAutoPickup*/CanProduceBlackStone，
/// 由 `:185-195` 组装成 `CreatureRules`）。本端 `CreatureType` 与之按名称对应：
/// `BabyPig`/`BabySkeleton`/`BabyKitten`(C# Kitten)/`BabyChicken`(C# Chick)/
/// `BabyBabyDragon`(C# BabyDragon) 取对应行；本端独有类型（Panda/Oma/Sheep/Gorilla/Custom/None）
/// C# 表中没有对应宠物，取全禁用默认（`MinimalFullness` 沿用 C# 字段默认 1000）。
pub fn creature_rules(t: CreatureType) -> mir2_shared::data::client_data::IntelligentCreatureRules {
    use mir2_shared::data::client_data::IntelligentCreatureRules as R;
    // C# 字段默认：全禁用 + `MinimalFullness = 1000`（`IntelligentCreatureInfo.cs:17`）
    let base = R {
        minimal_fullness: 1000,
        ..R::default()
    };
    match t {
        // C# `BabyPig`：SemiAutoPickupEnabled=true, SemiAutoPickupRange=3, MinimalFullness=4000
        CreatureType::BabyPig => R {
            minimal_fullness: 4000,
            semi_auto_pickup_enabled: true,
            semi_auto_pickup_range: 3,
            ..base
        },
        // C# `Chick`：Mouse 11 / Auto 7 / Semi 7 + CanProduceBlackStone
        CreatureType::BabyChicken => R {
            mouse_pickup_enabled: true,
            mouse_pickup_range: 11,
            auto_pickup_enabled: true,
            auto_pickup_range: 7,
            semi_auto_pickup_enabled: true,
            semi_auto_pickup_range: 7,
            can_produce_black_stone: true,
            ..base
        },
        // C# `Kitten`：Semi 3, MinimalFullness=6000
        CreatureType::BabyKitten => R {
            minimal_fullness: 6000,
            semi_auto_pickup_enabled: true,
            semi_auto_pickup_range: 3,
            ..base
        },
        // C# `BabySkeleton`：Mouse 11 / Auto 7 / Semi 7 + CanProduceBlackStone
        CreatureType::BabySkeleton => R {
            mouse_pickup_enabled: true,
            mouse_pickup_range: 11,
            auto_pickup_enabled: true,
            auto_pickup_range: 7,
            semi_auto_pickup_enabled: true,
            semi_auto_pickup_range: 7,
            can_produce_black_stone: true,
            ..base
        },
        // C# `BabyDragon`：Mouse 7 / Auto 5 / Semi 5, MinimalFullness=7000
        CreatureType::BabyBabyDragon => R {
            minimal_fullness: 7000,
            mouse_pickup_enabled: true,
            mouse_pickup_range: 7,
            auto_pickup_enabled: true,
            auto_pickup_range: 5,
            semi_auto_pickup_enabled: true,
            semi_auto_pickup_range: 5,
            ..base
        },
        _ => base,
    }
}

/// 宠物图标（C# `IntelligentCreatureInfo.Icon`，`Prguse2` 索引，`Server/MirDatabase/IntelligentCreatureInfo.cs:30-44`）。
///
/// 与 `creature_rules` 同一张 C# 静态表、同一套名称对应：`BabyPig`=500、`BabyChicken`(Chick)=501、
/// `BabyKitten`(Kitten)=502、`BabySkeleton`=503、`BabyBabyDragon`(BabyDragon)=507；
/// 本端独有类型（None/Panda/Oma/Sheep/Gorilla/Custom）C# 表中无对应 → `0`（客户端跳过绘制图标，
/// 与 C# `PetButton.Index` 构造默认 0 一致）。
pub fn creature_icon(t: CreatureType) -> i32 {
    match t {
        CreatureType::BabyPig => 500,
        CreatureType::BabyChicken => 501,
        CreatureType::BabyKitten => 502,
        CreatureType::BabySkeleton => 503,
        CreatureType::BabyBabyDragon => 507,
        _ => 0,
    }
}

/// 宠物蛋 `Info.Effect`（天数）→ 到期时刻（unix 秒；`0` = 永久）。
///
/// C# `PlayerObject.cs:6231`：`new UserIntelligentCreature(..., item.Info.Effect)`，
/// 构造里 `Expire = effect > 0 ? Now.AddDays(effect) : DateTime.MinValue`。
/// （C# `Info.Effect` 是 `byte`，本端物品库读出来是 `i32`，故参数取 `i32`。）
pub fn expire_from_effect_days(effect_days: i32, now_secs: i64) -> i64 {
    if effect_days == 0 {
        0
    } else {
        now_secs + effect_days as i64 * 86400
    }
}

/// 由宠物蛋创建宠物（C# `PlayerObject.cs:6231`）：`effect_days` 为 `Info.Effect`（0 = 永久）。
pub fn new_from_egg(
    creature_type: CreatureType,
    effect_days: i32,
    now_secs: i64,
) -> IntelligentCreature {
    let mut creature = IntelligentCreature::new(creature_type);
    creature.expire_at = expire_from_effect_days(effect_days, now_secs);
    creature
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

/// 物品过滤（C# IntelligentCreatureItemFilter：全部/金币/武器/盔甲/头盔/靴子/腰带/饰品/其他 + 品质）
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct CreatureFilter {
    pub pickup_all: bool,
    pub gold: bool,
    pub weapons: bool,
    pub armours: bool,
    pub helmets: bool,
    pub boots: bool,
    pub belts: bool,
    pub accessories: bool,
    pub others: bool,
    /// 品质（C# ItemGrade；0=None）
    pub grade: u8,
}

impl Default for CreatureFilter {
    fn default() -> Self {
        Self {
            pickup_all: true,
            gold: false,
            weapons: false,
            armours: false,
            helmets: false,
            boots: false,
            belts: false,
            accessories: false,
            others: false,
            grade: 0,
        }
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
    /// 宠物等级（NPC 脚本 PETLEVEL，对齐 C# PetLevel；默认 1，serde 兼容旧存档）
    #[serde(default = "default_creature_level")]
    pub level: u8,
    /// 物品过滤（serde 兼容旧存档）
    #[serde(default)]
    pub filter: CreatureFilter,
    /// 珍珠产出计数（C# IntelligentCreatureObject.PearlTicker；瞬态，不持久化，serde 兼容旧存档）
    #[serde(default)]
    pub pearl_ticker: u32,
    /// 黑曜石产出计时（秒；C# CreatureInfo.BlackstoneTime，持久化，serde 兼容旧存档）
    #[serde(default)]
    pub blackstone_time: u32,
    /// #2761 到期时间（unix 秒；0 = 永久，对齐 C# `UserIntelligentCreature.Expire == DateTime.MinValue`）。
    /// C# 由宠物蛋 `Info.Effect`（天数）在 `PlayerObject.cs:6231` 设定，`effect=0` 表示永久。
    #[serde(default)]
    pub expire_at: i64,
}

fn default_creature_level() -> u8 {
    1
}

impl IntelligentCreature {
    pub fn new(creature_type: CreatureType) -> Self {
        Self {
            creature_type,
            custom_name: None,
            pickup_mode: PickupMode::None,
            hunger: 100,
            enabled: false,
            level: 1,
            filter: CreatureFilter::default(),
            pearl_ticker: 0,
            blackstone_time: 0,
            expire_at: 0,
        }
    }

    /// 完整度（0..10000，C# `ClientIntelligentCreature.Fullness` 同一量纲）。
    /// 本端内部用 `hunger`(0..100) 并且满值 100 → 显示值 `hunger × 100`，
    /// 于是 `CreatureRules.MinimalFullness`（C# 4000/6000/7000/1000）可直接当比例用。
    pub fn fullness(&self) -> i32 {
        self.hunger as i32 * 100
    }

    /// 到期剩余秒数（C# 客户端用 `Expire - Now` 渲染 `过期: {PrintTimeSpanFromSeconds}`）；
    /// `expire_at == 0`（永久）返回 `None`。
    pub fn expire_in_secs(&self, now_secs: i64) -> Option<i64> {
        if self.expire_at <= 0 {
            None
        } else {
            Some((self.expire_at - now_secs).max(0))
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

    /// C# IntelligentCreatureObject.PearlProduceCount = 1000：拾取 1000 次 → 1 珍珠
    pub const PEARL_PRODUCE_COUNT: u32 = 1000;
    /// C# IntelligentCreatureObject.BlackstoneProduceTime = 10800 秒（3 小时）
    pub const BLACKSTONE_PRODUCE_TIME: u32 = 10800;

    /// C# IncreasePearlProduction（:720-733）：每次拾取操作 +1；满 1000 产出珍珠（返回 true）
    pub fn increase_pearl_production(&mut self) -> bool {
        self.pearl_ticker += 1;
        if self.pearl_ticker >= Self::PEARL_PRODUCE_COUNT {
            self.pearl_ticker = 0;
            true
        } else {
            false
        }
    }

    /// C# ProcessBlackStoneProduction（:735-750）：每 dt 秒 +dt；满 10800 产出黑曜石（返回 true，计时归零）
    pub fn process_blackstone_production(&mut self, dt_seconds: u32) -> bool {
        self.blackstone_time = self.blackstone_time.saturating_add(dt_seconds);
        if self.blackstone_time >= Self::BLACKSTONE_PRODUCE_TIME {
            self.blackstone_time = 0;
            true
        } else {
            false
        }
    }
}

/// 玩家宠物信息
#[derive(Debug, Clone, Default, serde::Serialize, serde::Deserialize)]
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
    fn test_restore_hunger_clamps() {
        // C# IncreaseFullness：恢复饥饿值且封顶 100（Pets 食物 23/24 用）
        let mut c = IntelligentCreature::new(CreatureType::BabyPanda);
        c.hunger = 90;
        c.restore_hunger(50);
        assert_eq!(c.hunger, 100);
        c.hunger = 0;
        c.restore_hunger(1);
        assert_eq!(c.hunger, 1);
    }

    /// #2757：规则表按名称对应 C# `IntelligentCreatureInfo`
    /// （Server/MirDatabase/IntelligentCreatureInfo.cs:30-44）——`BabyPig` 只开 Semi 3/满 4000、
    /// `BabyChicken`(=C# Chick) 与 `BabySkeleton` 开 M11/A7/S7 且产黑石、
    /// `BabyKitten`(=C# Kitten) Semi 3/满 6000、`BabyBabyDragon`(=C# BabyDragon) M7/A5/S5/满 7000；
    /// 本端独有类型（Panda/Oma/Sheep/Gorilla/Custom/None）C# 表无对应 → 全禁用默认。
    /// #2761：宠物蛋 `Effect`（天数）→ 到期时刻（C# `PlayerObject.cs:6231`，0 = 永久）。
    #[test]
    fn expire_from_effect_days_matches_csharp() {
        assert_eq!(expire_from_effect_days(0, 1_000_000), 0);
        assert_eq!(expire_from_effect_days(1, 1_000_000), 1_000_000 + 86400);
        assert_eq!(
            expire_from_effect_days(30, 1_000_000),
            1_000_000 + 30 * 86400
        );
    }

    /// #2761：宠物蛋创建路径把 `Effect` 落到 `expire_at`（0 = 永久）。
    #[test]
    fn new_from_egg_sets_expire_at() {
        let permanent = new_from_egg(CreatureType::BabyPig, 0, 1_000_000);
        assert_eq!(permanent.expire_at, 0);
        assert_eq!(permanent.expire_in_secs(1_000_000), None);

        let week = new_from_egg(CreatureType::BabyPig, 7, 1_000_000);
        assert_eq!(week.expire_in_secs(1_000_000), Some(7 * 86400));
    }

    /// #2761：图标表按名称对应 C# `IntelligentCreatureInfo.Icon`（500..514），
    /// 本端独有类型无对应 → 0（客户端跳过绘制）。
    #[test]
    fn creature_icon_mirrors_csharp_table() {
        assert_eq!(creature_icon(CreatureType::BabyPig), 500);
        assert_eq!(creature_icon(CreatureType::BabyChicken), 501);
        assert_eq!(creature_icon(CreatureType::BabyKitten), 502);
        assert_eq!(creature_icon(CreatureType::BabySkeleton), 503);
        assert_eq!(creature_icon(CreatureType::BabyBabyDragon), 507);
        for t in [
            CreatureType::None,
            CreatureType::BabyPanda,
            CreatureType::BabyOma,
            CreatureType::BabySheep,
            CreatureType::BabyGorilla,
            CreatureType::Custom,
        ] {
            assert_eq!(creature_icon(t), 0, "{t:?} 应无图标");
        }
    }

    /// #2761：完整度 = `hunger × 100`（0..10000，C# `Fullness` 量纲），
    /// 到期剩余秒数：0 = 永久（`None`），过期钳到 0。
    #[test]
    fn creature_fullness_and_expire_match_csharp_scale() {
        let mut c = IntelligentCreature::new(CreatureType::BabyPig);
        assert_eq!(c.fullness(), 10000); // 新宠物 hunger=100
        assert_eq!(c.expire_in_secs(1_000), None); // expire_at=0 → 永久

        c.hunger = 40;
        assert_eq!(c.fullness(), 4000); // = C# BabyPig 的 MinimalFullness（40%）

        c.expire_at = 1_000 + 7 * 86400;
        assert_eq!(c.expire_in_secs(1_000), Some(7 * 86400));
        assert_eq!(c.expire_in_secs(1_000 + 8 * 86400), Some(0)); // 已过期钳到 0
    }

    #[test]
    fn creature_rules_mirror_csharp_table() {
        let pig = creature_rules(CreatureType::BabyPig);
        assert_eq!(pig.minimal_fullness, 4000);
        assert!(pig.semi_auto_pickup_enabled && pig.semi_auto_pickup_range == 3);
        assert!(
            !pig.mouse_pickup_enabled && !pig.auto_pickup_enabled && !pig.can_produce_black_stone
        );

        let chick = creature_rules(CreatureType::BabyChicken);
        assert_eq!(
            (
                chick.mouse_pickup_range,
                chick.auto_pickup_range,
                chick.semi_auto_pickup_range,
                chick.can_produce_black_stone
            ),
            (11, 7, 7, true)
        );

        let kitten = creature_rules(CreatureType::BabyKitten);
        assert_eq!(
            (kitten.minimal_fullness, kitten.semi_auto_pickup_range),
            (6000, 3)
        );

        let skeleton = creature_rules(CreatureType::BabySkeleton);
        assert!(skeleton.can_produce_black_stone && skeleton.mouse_pickup_range == 11);

        let dragon = creature_rules(CreatureType::BabyBabyDragon);
        assert_eq!(
            (
                dragon.minimal_fullness,
                dragon.mouse_pickup_range,
                dragon.auto_pickup_range,
                dragon.semi_auto_pickup_range
            ),
            (7000, 7, 5, 5)
        );

        for t in [
            CreatureType::None,
            CreatureType::BabyPanda,
            CreatureType::BabyOma,
            CreatureType::BabySheep,
            CreatureType::BabyGorilla,
            CreatureType::Custom,
        ] {
            let r = creature_rules(t);
            assert!(
                !r.mouse_pickup_enabled
                    && !r.auto_pickup_enabled
                    && !r.semi_auto_pickup_enabled
                    && !r.can_produce_black_stone,
                "{t:?} 应取全禁用默认"
            );
        }
    }

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
        assert_eq!(
            log.active_creature.as_ref().unwrap().pickup_mode,
            PickupMode::GoldAndItem
        );

        log.restore_hunger(30);
        assert_eq!(log.active_creature.as_ref().unwrap().hunger, 100);
    }

    #[test]
    fn test_pearl_production() {
        // C# PearlProduceCount = 1000：拾取 1000 次 → 1 珍珠
        let mut c = IntelligentCreature::new(CreatureType::None);
        for _ in 0..999 {
            assert!(!c.increase_pearl_production());
        }
        assert!(c.increase_pearl_production()); // 第 1000 次产出
        assert_eq!(c.pearl_ticker, 0); // 计数归零
        assert!(!c.increase_pearl_production());
    }

    #[test]
    fn test_blackstone_production() {
        // C# BlackstoneProduceTime = 10800 秒（3 小时）
        let mut c = IntelligentCreature::new(CreatureType::None);
        c.blackstone_time = 10790;
        assert!(!c.process_blackstone_production(9)); // 10799 未满
        assert!(c.process_blackstone_production(10)); // 10800 → 产出
        assert_eq!(c.blackstone_time, 0); // 归零
        assert!(!c.process_blackstone_production(1));
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
