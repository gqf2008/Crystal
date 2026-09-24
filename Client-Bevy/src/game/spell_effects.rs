// ============================================================================
// 施法特效：按原版 C# 的「Spell → Effect(库, 起始帧, 帧数, 时长)」表播帧动画
//
// 背景（玩家反馈「魔法效果完全不对」）：本端此前把施法特效画成**一个染色的白方块**
// （effects.rs 的 `spell_color` + PendingEffect::Projectile/Burst），而原版播的是
// `Libraries.Magic / Magic2 / Magic3` 里的帧动画，每个法术的起始帧/帧数/时长都不同。
//
// 表的来源：`Client/MirObjects/PlayerObject.cs` 的 `MirAction.Spell` 分支（一张大 switch），
// 由 `Client-Bevy/tools/spell_effects_from_csharp.py` **机械生成**（手抄 100+ 条必错），
// 生成块在下方 SPELL_FX_BEGIN/END 之间，勿手改。
//
// 时长语义（与原版一致）：原版 `new Effect(lib, start, frames, interval, ob)` = 在 `interval` 毫秒里
// 播完 `frames` 帧；C# 写 `Frame.Count * FrameInterval`（依赖施法动作的帧数）时本表记 0，
// 渲染侧按 `DEFAULT_FRAME_MS` 每帧换算。
// ============================================================================

use bevy::prelude::*;

use crate::resources::libraries::LibraryName;
use mir2_shared::enums::{Monster, Spell};
// 生成表里写的是 `library: Magic` 这样的短名（对齐 C# 的 Libraries.Magic），故把变体引进作用域
#[allow(unused_imports)]
use SpellFxLibrary::{Effect, Magic, Magic2, Magic3};

/// 原版写 `Frame.Count * FrameInterval` 时的兜底每帧时长（ms）
pub const DEFAULT_FRAME_MS: u32 = 100;

/// 特效库（原版 Libraries.Magic / Magic2 / Magic3）
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum SpellFxLibrary {
    Magic,
    Magic2,
    Magic3,
    /// 原版 `Libraries.Effect`（`SpellEffect.Reflect` 的 580 帧段）
    Effect,
}

impl SpellFxLibrary {
    pub fn library(self) -> LibraryName {
        match self {
            SpellFxLibrary::Magic => LibraryName::Magic,
            SpellFxLibrary::Magic2 => LibraryName::Magic2,
            SpellFxLibrary::Magic3 => LibraryName::Magic3,
            SpellFxLibrary::Effect => LibraryName::Effect,
        }
    }
}

/// 一条施法特效（原版一条 `new Effect(...)`）
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct SpellFx {
    pub library: SpellFxLibrary,
    /// 库内起始帧
    pub start: usize,
    /// 帧数
    pub frames: usize,
    /// 原版 `2140 + (int)Direction * 10` 这类「按朝向取帧段」的步长（0 = 与朝向无关）
    pub dir_step: usize,
    /// 整段动画时长（ms）；0 = 原版写 Frame.Count * FrameInterval，按 DEFAULT_FRAME_MS 换算
    pub interval_ms: u32,
}

impl SpellFx {
    /// 实际起始帧（含朝向偏移）
    pub fn start_for_dir(&self, dir: u8) -> usize {
        self.start + self.dir_step * dir as usize
    }

    /// 整段动画时长（ms）与每帧时长（ms）
    pub fn timing(&self) -> (f32, f32) {
        let frames = self.frames.max(1) as f32;
        let total_ms = if self.interval_ms > 0 {
            self.interval_ms as f32
        } else {
            frames * DEFAULT_FRAME_MS as f32
        };
        (total_ms / 1000.0, total_ms / frames / 1000.0)
    }
}

/// 一条施法弹道（原版 `CreateProjectile(baseIndex, library, blend, count, interval, skip)`）
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct MissileFx {
    pub library: SpellFxLibrary,
    /// 库内起始帧
    pub base: usize,
    /// 帧数
    pub frames: usize,
    /// 每帧时长（ms）
    pub frame_ms: u32,
    /// 原版 `skip` 参数（本端按帧序播，先记录备查）
    pub skip: usize,
}

// ============================================================================
// 对象特效（`S.ObjectEffect`）：原版 `Client/MirScenes/GameScene.cs:4711-4930` 的
// `ObjectEffect(S.ObjectEffect p)` 大 switch——护盾光环 / 传送 / 治疗 / 暴击 / 冰柱 /
// 天罚 / 觉醒 / 月雾…每一类都是**一条或多条真帧动画**（MPEater、Hemorrhage 还是多条）。
//
// 本端此前把这些全画成一个纯色方块（`PendingEffect::Burst` + `spell_effect_color`），
// 与「魔法效果完全不对」的反馈直接相关。表同样由 C# 机械生成（脚本的
// `parse_object_effects()`），生成块在下方 OBJECT_FX_BEGIN/END 之间，勿手改。
// ============================================================================

/// 对象特效用的库：扁平库或怪物数组库
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum FxLib {
    Flat(SpellFxLibrary),
    /// 怪物库。必须带**两个**值：
    /// - `rust` = 本端 `Monster` 枚举（可读、可查）；
    /// - `lib` = **C# `Monster` 枚举值**，也就是 `Data/Monster/{:03}.Lib` 的资产索引。
    ///
    /// 为什么要分开：本端 `Monster` 枚举整体比 C# **大 3**（C# `Guard = 0` / 本端 `Guard = 3`；
    /// 504 个同名项逐一核对**全部 +3**），而资产是按 **C# 值**编号的——`Monster/201.Lib`
    /// 才是石像（667 帧，含原版 `ObjectEffect` 用的 632 帧），`Monster/204.Lib` 是另一只小怪
    /// （224 帧，632 直接越界）。生成器从 `Shared/Enums.cs` 取 C# 值写进 `lib`，
    /// 不能拿本端枚举值当资产索引用。
    ///
    /// 对照：actor 渲染走服务端 DB 的 `image` 字段，那本来就是 C# 值，所以没踩这个坑；
    /// 只有「按 C# 源码生成的表」需要自己换算。
    Monster {
        rust: Monster,
        lib: usize,
    },
}

impl FxLib {
    /// 扁平库名（`Monster` 走数组库，返回 None）
    pub fn flat(self) -> Option<LibraryName> {
        match self {
            FxLib::Flat(l) => Some(l.library()),
            FxLib::Monster { .. } => None,
        }
    }

    /// 本端怪物枚举值（`Flat` 返回 None）
    pub fn monster(self) -> Option<Monster> {
        match self {
            FxLib::Monster { rust, .. } => Some(rust),
            FxLib::Flat(_) => None,
        }
    }

    /// 怪物库的**资产索引**（= C# `Monster` 枚举值；`Flat` 返回 None）
    pub fn monster_lib(self) -> Option<usize> {
        match self {
            FxLib::Monster { lib, .. } => Some(lib),
            FxLib::Flat(_) => None,
        }
    }

    /// 诊断/探针用的短标签（`Flat(Magic)` / `Monster(StoningStatue)`）
    pub fn label(self) -> String {
        match self {
            FxLib::Flat(l) => format!("Flat({l:?})"),
            FxLib::Monster { rust, .. } => format!("Monster({rust:?})"),
        }
    }
}

/// 原版 `if (p.EffectType == 0)` 这类按 `EffectType` 分流的条件（KingGuard 的 753/763）
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum FxWhen {
    Always,
    EffectTypeZero,
    EffectTypeNonZero,
}

impl FxWhen {
    pub fn matches(self, effect_type: u32) -> bool {
        match self {
            FxWhen::Always => true,
            FxWhen::EffectTypeZero => effect_type == 0,
            FxWhen::EffectTypeNonZero => effect_type != 0,
        }
    }
}

/// 种族过滤（原版 `if (ob.Race != ObjectType.X ...) return;`）。
///
/// 原版在 switch 里对四个 case 做了提前 return（`GameScene.cs:4768/4779/4804/4816`）：
/// - `MagicShieldUp` / `MagicShieldDown`：`!= Player && != Hero` → 玩家或英雄；
/// - `ElementalBarrierUp` / `ElementalBarrierDown`：`!= Player` → 仅玩家。
///
/// 本端的对象分类：`Player` 标记覆盖本地玩家与远程玩家（本地玩家同时挂 `LocalPlayer` + `Player`，
/// 见 `actor/spawn_helpers.rs:44-45`），怪物是 `Monster`，NPC 是 `Npc`；
/// **本端暂无独立的 `Hero` 标记**（英雄在渲染上依附玩家实体），所以 `PlayerOrHero` 与 `PlayerOnly`
/// 目前都按「有 `Player` 标记」判定——两者在表里分开保留，等真的出现英雄实体时只需收紧 `PlayerOnly`。
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum FxRace {
    Any,
    PlayerOrHero,
    PlayerOnly,
}

impl FxRace {
    pub fn matches(self, is_player: bool) -> bool {
        match self {
            FxRace::Any => true,
            FxRace::PlayerOrHero | FxRace::PlayerOnly => is_player,
        }
    }
}

/// 循环光环分组（原版 `PlayerObject.ShieldEffect` / `PlayerObject.ElementalBarrierEffect`）
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum AuraGroup {
    MagicShield,
    ElementalBarrier,
}

/// 循环语义（原版 `Effect.Repeat` / `DelayedExplosionEffect` 的 stage）
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum FxRepeat {
    /// 播完 `frames` 即消失（原版默认）
    Once,
    /// 一直循环，直到收到同组的 Down 包（原版 `Repeat = true`，由 Down 清理）
    UntilDown(AuraGroup),
    /// 原版 `Repeat = p.Time > 0`：`time > 0` 时循环 `time` 毫秒，否则播完即消失
    /// （Stunned / FlamingMutantWeb）
    PacketTime,
    /// 原版 `DelayedExplosionEffect`：`stage != 2` 时循环；同对象新 stage 到达即替换
    StageNot2,
}

/// 特效锚定的对象
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum FxTarget {
    /// 包里的 `ob`（`p.ObjectID`）
    Owner,
    /// `p.EffectType` 指向的那个对象（原版 MPEater 的 `ob2`；该字段在此当**对象 ID** 用）
    EffectType,
    /// `ob.CurrentLocation`（Behemoth：挂地图位置、不跟随对象）
    OwnerLocation,
}

/// 一条对象特效（原版一条 `new Effect(...)` 或 `new DelayedExplosionEffect(...)`）
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ObjectFx {
    pub lib: FxLib,
    /// 库内起始帧
    pub start: usize,
    /// 帧数
    pub frames: usize,
    /// 整段动画时长（ms）；0 = 原版写 `Frame.Count * FrameInterval`，按 DEFAULT_FRAME_MS 换算
    pub interval_ms: u32,
    /// 原版 `Effect.Blend`：`true`（默认）→ 加法混合（`Mesh2d` + `ObjectFxBlendMaterial`）；
    /// `false` → 普通 alpha（`Sprite`）。语义按 C# 源码钉死：`DXManager.cs:378-379` / `Effect.cs:132`。
    pub blend: bool,
    /// 原版 `if (ob.Race != ObjectType.X) return;` 的种族过滤（默认 `Any` = 不过滤）
    pub race: FxRace,
    /// 原版 `SoundManager.PlaySound(<expr>)` 的音效 id（`None` = 该 case 原版不播）
    pub sound: Option<u32>,
    /// 按 `EffectType` 分流的条件（原版 `if (p.EffectType == 0)`）
    pub when: FxWhen,
    pub repeat: FxRepeat,
    pub target: FxTarget,
    /// 原版 `1590 + (int)p.EffectType * 10` 这类按 `EffectType` 取帧段
    pub step_effect_type: usize,
    /// 原版 `375 + CMain.Random.Next(3) * 20` 这类随机取帧段
    pub rand_step: usize,
    pub rand_count: usize,
    /// 原版 `272 + (int)ob.Direction * 4` 这类按朝向取帧段
    pub dir_step: usize,
    /// 原版把生成时刻写成 `CMain.Time + p.DelayTime`（觉醒系列）→ 延迟后再开播
    pub delay_from_packet: bool,
}

impl ObjectFx {
    /// 生成块里只写与默认值不同的字段（`..ObjectFx::DEFAULT`），便于 diff 与幂等
    pub const DEFAULT: ObjectFx = ObjectFx {
        lib: FxLib::Flat(SpellFxLibrary::Magic),
        start: 0,
        frames: 1,
        interval_ms: 0,
        blend: true,
        race: FxRace::Any,
        sound: None,
        when: FxWhen::Always,
        repeat: FxRepeat::Once,
        target: FxTarget::Owner,
        step_effect_type: 0,
        rand_step: 0,
        rand_count: 0,
        dir_step: 0,
        delay_from_packet: false,
    };

    /// 实际起始帧：`start + effect_type * step_effect_type + dir * dir_step`，
    /// 再按 `rand_count` 随机挑一档（`rand` 由调用方给，保持纯函数可测）。
    pub fn base_frame(&self, effect_type: u32, dir: u8, rand: u32) -> usize {
        let mut base = self.start + self.step_effect_type * effect_type as usize;
        if self.dir_step > 0 {
            base += self.dir_step * dir as usize;
        }
        if self.rand_count > 1 && self.rand_step > 0 {
            base += self.rand_step * (rand % self.rand_count as u32) as usize;
        }
        base
    }

    /// 整段时长（秒）与每帧时长（秒）；原版 `interval` 是整段时长而非每帧
    pub fn timing(&self) -> (f32, f32) {
        let frames = self.frames.max(1) as f32;
        let total_ms = if self.interval_ms > 0 {
            self.interval_ms as f32
        } else {
            frames * DEFAULT_FRAME_MS as f32
        };
        (total_ms / 1000.0, total_ms / frames / 1000.0)
    }
}

/// 查表：该法术的弹道（原版 MirAction.Spell 分支里 `CreateProjectile` 的那些法术）
pub fn spell_missile(spell: Spell) -> Option<MissileFx> {
    let name = format!("{spell:?}");
    SPELL_MISSILE
        .iter()
        .find(|(n, _)| *n == name)
        .map(|(_, fx)| *fx)
}

/// 查表：**远程攻击**的弹道（原版 `MirAction.AttackRange1/2/3` 分支里的 `CreateProjectile`）。
///
/// - `spell == 0` = 普通弓射（C# `AttackRange1` 的 `case 5:`，无技能）→ `DefaultArrow`；
/// - 其余按技能名查（StraightShot / DoubleShot / ElementalShot / SummonSnakes /
///   Stonetrap / DelayedExplosion / CrippleShot / NapalmShot）。
///
/// 表里没有的技能返回 `None` → 调用方退回占位弹道（不静默）。
pub fn range_missile(spell: u8) -> Option<MissileFx> {
    let name = if spell == 0 {
        "DefaultArrow".to_string()
    } else {
        format!("{:?}", Spell::try_from(spell).ok()?)
    };
    RANGE_MISSILE
        .iter()
        .find(|(n, _)| *n == name)
        .map(|(_, fx)| *fx)
}

/// 查表：这个对象特效 C# 会画什么。
///
/// - `None` = C# 的 switch 里**没有**这个 case（本端才允许退回占位表现，且不静默）；
/// - `Some(&[])` = C# **明确不画**（`Critical` 被注释掉、`MagicShieldDown` 只做清理）。
pub fn object_fx(name: &str) -> Option<&'static [ObjectFx]> {
    OBJECT_FX.iter().find(|(n, _)| *n == name).map(|(_, v)| *v)
}

/// 同上，但**同时返回表里的名字**（`&'static str`）——渲染实体要长期持有它做诊断/探针，
/// 不能拿调用方那次 `format!` 出来的临时 `String`。
pub fn object_fx_entry(name: &str) -> Option<(&'static str, &'static [ObjectFx])> {
    OBJECT_FX
        .iter()
        .find(|(n, _)| *n == name)
        .map(|(n, v)| (*n, *v))
}

/// 该特效名属于哪个循环光环的 Up/Down。
///
/// 原版在 MagicShield/ElementalBarrier 的 Up 与 Down 里都先
/// `ShieldEffect.Clear(); ShieldEffect.Remove();`（`GameScene.cs:4773-4797`、`:4816-4843`），
/// 所以收到任一包都要先清同组实体，否则护盾会叠成两层。
pub fn aura_group(name: &str) -> Option<AuraGroup> {
    match name {
        "MagicShieldUp" | "MagicShieldDown" => Some(AuraGroup::MagicShield),
        "ElementalBarrierUp" | "ElementalBarrierDown" => Some(AuraGroup::ElementalBarrier),
        _ => None,
    }
}

// ==== SPELL_FX_BEGIN（由 Client-Bevy/tools/spell_effects_from_csharp.py 生成，勿手改）====
/// 原版施法特效表（`Client/MirObjects/PlayerObject.cs` MirAction.Spell 分支机械生成）
#[rustfmt::skip]  // 生成块：保持每条一行，便于 diff 与 --write 幂等
pub const SPELL_FX: &[(&str, SpellFx)] = &[
    ("FireBall", SpellFx { library: Magic, start: 0, frames: 10, dir_step: 0, interval_ms: 0 }),
    ("Healing", SpellFx { library: Magic, start: 200, frames: 10, dir_step: 0, interval_ms: 0 }),
    ("Repulsion", SpellFx { library: Magic, start: 900, frames: 6, dir_step: 0, interval_ms: 0 }),
    ("ElectricShock", SpellFx { library: Magic, start: 1560, frames: 10, dir_step: 0, interval_ms: 0 }),
    ("Poisoning", SpellFx { library: Magic, start: 600, frames: 10, dir_step: 0, interval_ms: 0 }),
    ("GreatFireBall", SpellFx { library: Magic, start: 400, frames: 10, dir_step: 0, interval_ms: 0 }),
    ("HellFire", SpellFx { library: Magic, start: 920, frames: 10, dir_step: 0, interval_ms: 0 }),
    ("ThunderBolt", SpellFx { library: Magic2, start: 20, frames: 3, dir_step: 0, interval_ms: 300 }),
    ("SummonSkeleton", SpellFx { library: Magic, start: 1500, frames: 10, dir_step: 0, interval_ms: 0 }),
    ("StormEscape", SpellFx { library: Magic3, start: 590, frames: 10, dir_step: 0, interval_ms: 0 }),
    ("Teleport", SpellFx { library: Magic, start: 1590, frames: 10, dir_step: 0, interval_ms: 0 }),
    ("Blink", SpellFx { library: Magic, start: 1590, frames: 10, dir_step: 0, interval_ms: 0 }),
    ("Hiding", SpellFx { library: Magic, start: 1520, frames: 10, dir_step: 0, interval_ms: 0 }),
    ("Haste", SpellFx { library: Magic2, start: 2140, frames: 6, dir_step: 10, interval_ms: 0 }),
    ("Fury", SpellFx { library: Magic3, start: 200, frames: 8, dir_step: 0, interval_ms: 0 }),
    ("Fury", SpellFx { library: Magic3, start: 187, frames: 10, dir_step: 0, interval_ms: 0 }),
    ("ImmortalSkin", SpellFx { library: Magic3, start: 550, frames: 17, dir_step: 0, interval_ms: 0 }),
    ("ImmortalSkin", SpellFx { library: Magic3, start: 570, frames: 5, dir_step: 0, interval_ms: 0 }),
    ("FireBang", SpellFx { library: Magic, start: 1650, frames: 10, dir_step: 0, interval_ms: 0 }),
    ("FireWall", SpellFx { library: Magic, start: 1620, frames: 10, dir_step: 0, interval_ms: 0 }),
    ("HealingCircle", SpellFx { library: Magic3, start: 620, frames: 10, dir_step: 0, interval_ms: 0 }),
    ("MoonMist", SpellFx { library: Magic3, start: 680, frames: 25, dir_step: 0, interval_ms: 1800 }),
    ("TrapHexagon", SpellFx { library: Magic, start: 1380, frames: 10, dir_step: 0, interval_ms: 0 }),
    ("EnergyRepulsor", SpellFx { library: Magic2, start: 190, frames: 6, dir_step: 0, interval_ms: 0 }),
    ("FireBurst", SpellFx { library: Magic2, start: 2320, frames: 10, dir_step: 0, interval_ms: 0 }),
    ("FlameDisruptor", SpellFx { library: Magic2, start: 130, frames: 6, dir_step: 0, interval_ms: 0 }),
    ("SummonShinsu", SpellFx { library: Magic2, start: 0, frames: 10, dir_step: 0, interval_ms: 0 }),
    ("UltimateEnhancer", SpellFx { library: Magic2, start: 160, frames: 15, dir_step: 0, interval_ms: 1000 }),
    ("FrostCrunch", SpellFx { library: Magic2, start: 400, frames: 10, dir_step: 0, interval_ms: 0 }),
    ("Purification", SpellFx { library: Magic2, start: 600, frames: 10, dir_step: 0, interval_ms: 0 }),
    ("FlameField", SpellFx { library: Magic2, start: 910, frames: 23, dir_step: 0, interval_ms: 1800 }),
    ("Trap", SpellFx { library: Magic2, start: 2340, frames: 11, dir_step: 0, interval_ms: 0 }),
    ("MoonLight", SpellFx { library: Magic2, start: 2380, frames: 10, dir_step: 0, interval_ms: 0 }),
    ("SwiftFeet", SpellFx { library: Magic2, start: 2440, frames: 16, dir_step: 0, interval_ms: 0 }),
    ("LightBody", SpellFx { library: Magic2, start: 2470, frames: 10, dir_step: 0, interval_ms: 0 }),
    ("PoisonSword", SpellFx { library: Magic2, start: 2490, frames: 10, dir_step: 10, interval_ms: 0 }),
    ("DarkBody", SpellFx { library: Magic2, start: 2580, frames: 10, dir_step: 0, interval_ms: 0 }),
    ("ThunderStorm", SpellFx { library: Magic, start: 1680, frames: 10, dir_step: 0, interval_ms: 0 }),
    ("MassHealing", SpellFx { library: Magic, start: 1790, frames: 10, dir_step: 0, interval_ms: 0 }),
    ("IceStorm", SpellFx { library: Magic, start: 3840, frames: 10, dir_step: 0, interval_ms: 0 }),
    ("MagicShield", SpellFx { library: Magic, start: 3880, frames: 10, dir_step: 0, interval_ms: 0 }),
    ("TurnUndead", SpellFx { library: Magic, start: 3920, frames: 10, dir_step: 0, interval_ms: 0 }),
    ("MagicBooster", SpellFx { library: Magic3, start: 80, frames: 9, dir_step: 0, interval_ms: 0 }),
    ("PetEnhancer", SpellFx { library: Magic3, start: 200, frames: 8, dir_step: 0, interval_ms: 0 }),
    ("Revelation", SpellFx { library: Magic, start: 3960, frames: 20, dir_step: 0, interval_ms: 1200 }),
    ("ProtectionField", SpellFx { library: Magic2, start: 1520, frames: 10, dir_step: 0, interval_ms: 0 }),
    ("Rage", SpellFx { library: Magic2, start: 1510, frames: 10, dir_step: 0, interval_ms: 0 }),
    ("Vampirism", SpellFx { library: Magic2, start: 1040, frames: 7, dir_step: 0, interval_ms: 0 }),
    ("BattleCry", SpellFx { library: Magic2, start: 710, frames: 20, dir_step: 0, interval_ms: 1200 }),
    ("TwinDrakeBlade", SpellFx { library: Magic2, start: 210, frames: 6, dir_step: 0, interval_ms: 500 }),
    ("Entrapment", SpellFx { library: Magic2, start: 990, frames: 10, dir_step: 0, interval_ms: 0 }),
    ("BladeAvalanche", SpellFx { library: Magic2, start: 740, frames: 15, dir_step: 20, interval_ms: 0 }),
    ("SlashingBurst", SpellFx { library: Magic2, start: 1700, frames: 9, dir_step: 10, interval_ms: 0 }),
    ("CounterAttack", SpellFx { library: Magic, start: 3480, frames: 10, dir_step: 10, interval_ms: 0 }),
    ("CounterAttack", SpellFx { library: Magic3, start: 140, frames: 2, dir_step: 0, interval_ms: 0 }),
    ("CrescentSlash", SpellFx { library: Magic2, start: 2620, frames: 20, dir_step: 20, interval_ms: 0 }),
    ("Mirroring", SpellFx { library: Magic2, start: 650, frames: 10, dir_step: 0, interval_ms: 0 }),
    ("Blizzard", SpellFx { library: Magic2, start: 1540, frames: 8, dir_step: 0, interval_ms: 0 }),
    ("MeteorStrike", SpellFx { library: Magic2, start: 1590, frames: 10, dir_step: 0, interval_ms: 0 }),
    ("HeavenlySword", SpellFx { library: Magic2, start: 2230, frames: 8, dir_step: 10, interval_ms: 800 }),
    ("ElementalBarrier", SpellFx { library: Magic3, start: 1880, frames: 8, dir_step: 0, interval_ms: 0 }),
    ("PoisonShot", SpellFx { library: Magic3, start: 2300, frames: 8, dir_step: 0, interval_ms: 1000 }),
    ("OneWithNature", SpellFx { library: Magic3, start: 2710, frames: 8, dir_step: 0, interval_ms: 1200 }),
    ("FireBounce", SpellFx { library: Magic, start: 400, frames: 10, dir_step: 0, interval_ms: 0 }),
    ("MeteorShower", SpellFx { library: Magic, start: 400, frames: 10, dir_step: 0, interval_ms: 0 }),
];
// ==== SPELL_FX_END ====
// ==== SPELL_MISSILE_BEGIN（由 Client-Bevy/tools/spell_effects_from_csharp.py 生成，勿手改）====
/// 原版施法弹道表（`Client/MirObjects/PlayerObject.cs` MirAction.Spell 分支的 CreateProjectile）
#[rustfmt::skip]  // 生成块：保持每条一行，便于 diff 与 --write 幂等
pub const SPELL_MISSILE: &[(&str, MissileFx)] = &[
    ("FireBall", MissileFx { library: Magic, base: 10, frames: 6, frame_ms: 30, skip: 4 }),
    ("GreatFireBall", MissileFx { library: Magic, base: 410, frames: 6, frame_ms: 30, skip: 4 }),
    ("SoulFireBall", MissileFx { library: Magic, base: 1160, frames: 3, frame_ms: 30, skip: 7 }),
    ("MassHiding", MissileFx { library: Magic, base: 1160, frames: 3, frame_ms: 30, skip: 7 }),
    ("SoulShield", MissileFx { library: Magic, base: 1160, frames: 3, frame_ms: 30, skip: 7 }),
    ("BlessedArmour", MissileFx { library: Magic, base: 1160, frames: 3, frame_ms: 30, skip: 7 }),
    ("CatTongue", MissileFx { library: Magic3, base: 260, frames: 6, frame_ms: 30, skip: 4 }),
    ("FrostCrunch", MissileFx { library: Magic2, base: 410, frames: 4, frame_ms: 30, skip: 6 }),
    ("Curse", MissileFx { library: Magic, base: 1160, frames: 3, frame_ms: 30, skip: 7 }),
    ("Hallucination", MissileFx { library: Magic, base: 1160, frames: 3, frame_ms: 48, skip: 7 }),
    ("PoisonCloud", MissileFx { library: Magic, base: 1160, frames: 3, frame_ms: 30, skip: 7 }),
    ("Plague", MissileFx { library: Magic, base: 1160, frames: 3, frame_ms: 30, skip: 7 }),
    ("FireBounce", MissileFx { library: Magic, base: 410, frames: 6, frame_ms: 30, skip: 4 }),
    ("MeteorShower", MissileFx { library: Magic, base: 410, frames: 6, frame_ms: 30, skip: 4 }),
];
// ==== SPELL_MISSILE_END ====
// ==== RANGE_MISSILE_BEGIN（由 Client-Bevy/tools/spell_effects_from_csharp.py 生成，勿手改）====
/// 原版远程攻击弹道表（`Client/MirObjects/PlayerObject.cs` MirAction.AttackRange1/2/3 分支）
/// `DefaultArrow` = 普通弓射（AttackRange1 的 `case 5:`，无技能）
#[rustfmt::skip]  // 生成块：保持每条一行，便于 diff 与 --write 幂等
pub const RANGE_MISSILE: &[(&str, MissileFx)] = &[
    ("DefaultArrow", MissileFx { library: Magic3, base: 1030, frames: 5, frame_ms: 30, skip: 5 }),
    ("StraightShot", MissileFx { library: Magic3, base: 1210, frames: 5, frame_ms: 30, skip: 5 }),
    ("DoubleShot", MissileFx { library: Magic3, base: 1030, frames: 5, frame_ms: 30, skip: 5 }),
    ("ElementalShot", MissileFx { library: Magic3, base: 1690, frames: 6, frame_ms: 30, skip: 4 }),
    ("SummonSnakes", MissileFx { library: Magic3, base: 2750, frames: 5, frame_ms: 10, skip: 5 }),
    ("Stonetrap", MissileFx { library: Magic3, base: 2750, frames: 5, frame_ms: 20, skip: 5 }),
    ("DelayedExplosion", MissileFx { library: Magic3, base: 1030, frames: 5, frame_ms: 30, skip: 5 }),
    ("CrippleShot", MissileFx { library: Magic3, base: 2330, frames: 5, frame_ms: 10, skip: 5 }),
    ("NapalmShot", MissileFx { library: Magic3, base: 2530, frames: 6, frame_ms: 50, skip: 4 }),
];
// ==== RANGE_MISSILE_END ====

// ==== OBJECT_FX_BEGIN（由 Client-Bevy/tools/spell_effects_from_csharp.py 生成，勿手改）====
/// 原版对象特效表（`Client/MirScenes/GameScene.cs` 的 `ObjectEffect` switch 机械生成）
/// 空切片 = C# 明确不画；表里没有的名字 = C# 没有这个 case（调用方才退回占位表现）
#[rustfmt::skip]  // 生成块：保持每条一行，便于 diff 与 --write 幂等
pub const OBJECT_FX: &[(&str, &[ObjectFx])] = &[
    ("FurbolgWarriorCritical", &[
        ObjectFx { lib: FxLib::Monster { rust: Monster::FurbolgWarrior, lib: 406 }, start: 400, frames: 6, interval_ms: 600, sound: Some(20910), ..ObjectFx::DEFAULT },
    ]),
    ("FatalSword", &[
        ObjectFx { lib: FxLib::Flat(Magic2), start: 1940, frames: 4, interval_ms: 400, sound: Some(20910), ..ObjectFx::DEFAULT },
    ]),
    ("StormEscape", &[
        ObjectFx { lib: FxLib::Flat(Magic3), start: 610, frames: 10, interval_ms: 600, sound: Some(10110), ..ObjectFx::DEFAULT },
    ]),
    ("Teleport", &[
        ObjectFx { lib: FxLib::Flat(Magic), start: 1600, frames: 10, interval_ms: 600, sound: Some(10110), ..ObjectFx::DEFAULT },
    ]),
    ("Healing", &[
        ObjectFx { lib: FxLib::Flat(Magic), start: 370, frames: 10, interval_ms: 800, sound: Some(20611), ..ObjectFx::DEFAULT },
    ]),
    ("RedMoonEvil", &[
        ObjectFx { lib: FxLib::Monster { rust: Monster::RedMoonEvil, lib: 62 }, start: 32, frames: 6, interval_ms: 400, blend: false, ..ObjectFx::DEFAULT },
    ]),
    ("TwinDrakeBlade", &[
        ObjectFx { lib: FxLib::Flat(Magic2), start: 380, frames: 6, interval_ms: 800, ..ObjectFx::DEFAULT },
    ]),
    ("MPEater", &[
        ObjectFx { lib: FxLib::Flat(Magic2), start: 2411, frames: 19, interval_ms: 1900, target: FxTarget::EffectType, sound: Some(20910), ..ObjectFx::DEFAULT },
        ObjectFx { lib: FxLib::Flat(Magic2), start: 2400, frames: 9, interval_ms: 900, ..ObjectFx::DEFAULT },
    ]),
    ("Bleeding", &[
        ObjectFx { lib: FxLib::Flat(Magic3), start: 60, frames: 3, interval_ms: 400, ..ObjectFx::DEFAULT },
    ]),
    ("Hemorrhage", &[
        ObjectFx { lib: FxLib::Flat(Magic3), start: 0, frames: 4, interval_ms: 400, sound: Some(21040), ..ObjectFx::DEFAULT },
        ObjectFx { lib: FxLib::Flat(Magic3), start: 28, frames: 6, interval_ms: 600, ..ObjectFx::DEFAULT },
        ObjectFx { lib: FxLib::Flat(Magic3), start: 46, frames: 8, interval_ms: 800, ..ObjectFx::DEFAULT },
    ]),
    ("MagicShieldUp", &[
        ObjectFx { lib: FxLib::Flat(Magic), start: 3890, race: FxRace::PlayerOrHero, frames: 3, interval_ms: 600, repeat: FxRepeat::UntilDown(AuraGroup::MagicShield), ..ObjectFx::DEFAULT },
    ]),
    ("MagicShieldDown", &[]),
    ("GreatFoxSpirit", &[
        ObjectFx { lib: FxLib::Monster { rust: Monster::GreatFoxSpirit, lib: 134 }, start: 375, rand_step: 20, rand_count: 3, frames: 20, interval_ms: 1400, sound: Some(1345), ..ObjectFx::DEFAULT },
    ]),
    ("Entrapment", &[
        ObjectFx { lib: FxLib::Flat(Magic2), start: 1010, frames: 10, interval_ms: 1500, ..ObjectFx::DEFAULT },
        ObjectFx { lib: FxLib::Flat(Magic2), start: 1020, frames: 8, interval_ms: 1200, ..ObjectFx::DEFAULT },
    ]),
    ("Critical", &[]),
    ("Reflect", &[
        ObjectFx { lib: FxLib::Flat(Effect), start: 580, frames: 10, interval_ms: 70, ..ObjectFx::DEFAULT },
    ]),
    ("ElementalBarrierUp", &[
        ObjectFx { lib: FxLib::Flat(Magic3), start: 1890, race: FxRace::PlayerOnly, frames: 10, interval_ms: 2000, repeat: FxRepeat::UntilDown(AuraGroup::ElementalBarrier), ..ObjectFx::DEFAULT },
    ]),
    ("ElementalBarrierDown", &[
        ObjectFx { lib: FxLib::Flat(Magic3), start: 1910, race: FxRace::PlayerOnly, frames: 7, interval_ms: 1400, sound: Some(21315), ..ObjectFx::DEFAULT },
    ]),
    // DelayedExplosion：C# 的 `effectid < 0` 支路是同一段动画的 stage=0（本端按 stage 取帧段，effect_type=0 时帧段相同），故只保留 stage 那条
    ("DelayedExplosion", &[
        ObjectFx { lib: FxLib::Flat(Magic3), start: 1590, step_effect_type: 10, frames: 8, interval_ms: 1200, repeat: FxRepeat::StageNot2, ..ObjectFx::DEFAULT },
    ]),
    ("AwakeningSuccess", &[
        ObjectFx { lib: FxLib::Flat(Magic3), start: 900, frames: 16, interval_ms: 1600, delay_from_packet: true, sound: Some(50002), ..ObjectFx::DEFAULT },
        ObjectFx { lib: FxLib::Flat(Magic3), start: 840, frames: 16, interval_ms: 1600, blend: false, delay_from_packet: true, ..ObjectFx::DEFAULT },
    ]),
    ("AwakeningFail", &[
        ObjectFx { lib: FxLib::Flat(Magic3), start: 920, frames: 9, interval_ms: 900, delay_from_packet: true, sound: Some(50003), ..ObjectFx::DEFAULT },
        ObjectFx { lib: FxLib::Flat(Magic3), start: 860, frames: 9, interval_ms: 900, blend: false, delay_from_packet: true, ..ObjectFx::DEFAULT },
    ]),
    ("AwakeningHit", &[
        ObjectFx { lib: FxLib::Flat(Magic3), start: 880, frames: 5, interval_ms: 500, delay_from_packet: true, sound: Some(50001), ..ObjectFx::DEFAULT },
        ObjectFx { lib: FxLib::Flat(Magic3), start: 820, frames: 5, interval_ms: 500, blend: false, delay_from_packet: true, ..ObjectFx::DEFAULT },
    ]),
    ("AwakeningMiss", &[
        ObjectFx { lib: FxLib::Flat(Magic3), start: 890, frames: 5, interval_ms: 500, delay_from_packet: true, sound: Some(50000), ..ObjectFx::DEFAULT },
        ObjectFx { lib: FxLib::Flat(Magic3), start: 830, frames: 5, interval_ms: 500, blend: false, delay_from_packet: true, ..ObjectFx::DEFAULT },
    ]),
    ("TurtleKing", &[
        ObjectFx { lib: FxLib::Monster { rust: Monster::TurtleKing, lib: 187 }, start: 922, rand_step: 12, rand_count: 2, frames: 12, interval_ms: 1200, sound: Some(20351), ..ObjectFx::DEFAULT },
    ]),
    ("Behemoth", &[
        ObjectFx { lib: FxLib::Monster { rust: Monster::Behemoth, lib: 158 }, start: 788, frames: 10, interval_ms: 1500, target: FxTarget::OwnerLocation, ..ObjectFx::DEFAULT },
        ObjectFx { lib: FxLib::Monster { rust: Monster::Behemoth, lib: 158 }, start: 778, frames: 10, interval_ms: 1500, blend: false, target: FxTarget::OwnerLocation, ..ObjectFx::DEFAULT },
    ]),
    ("Stunned", &[
        ObjectFx { lib: FxLib::Monster { rust: Monster::StoningStatue, lib: 201 }, start: 632, frames: 10, interval_ms: 1000, repeat: FxRepeat::PacketTime, ..ObjectFx::DEFAULT },
    ]),
    ("IcePillar", &[
        ObjectFx { lib: FxLib::Monster { rust: Monster::IcePillar, lib: 231 }, start: 18, frames: 8, interval_ms: 800, ..ObjectFx::DEFAULT },
    ]),
    ("KingGuard", &[
        ObjectFx { lib: FxLib::Monster { rust: Monster::KingGuard, lib: 252 }, start: 753, frames: 10, interval_ms: 1000, blend: false, when: FxWhen::EffectTypeZero, ..ObjectFx::DEFAULT },
        ObjectFx { lib: FxLib::Monster { rust: Monster::KingGuard, lib: 252 }, start: 763, frames: 10, interval_ms: 1000, blend: false, when: FxWhen::EffectTypeNonZero, ..ObjectFx::DEFAULT },
    ]),
    ("FlamingMutantWeb", &[
        ObjectFx { lib: FxLib::Monster { rust: Monster::FlamingMutant, lib: 200 }, start: 330, frames: 10, interval_ms: 1000, repeat: FxRepeat::PacketTime, ..ObjectFx::DEFAULT },
    ]),
    ("DeathCrawlerBreath", &[
        ObjectFx { lib: FxLib::Monster { rust: Monster::DeathCrawler, lib: 261 }, start: 272, dir_step: 4, frames: 4, interval_ms: 400, ..ObjectFx::DEFAULT },
    ]),
    ("MoonMist", &[
        ObjectFx { lib: FxLib::Flat(Magic3), start: 705, frames: 10, interval_ms: 800, ..ObjectFx::DEFAULT },
    ]),
];
// ==== OBJECT_FX_END ====

/// 查表：按法术 + 朝向取该次施法要播的特效（原版每个 Spell 至少一条）
pub fn spell_fx(spell: Spell, dir: u8) -> Option<SpellFx> {
    lookup_by_name(&format!("{spell:?}")).map(|mut fx| {
        fx.start = fx.start_for_dir(dir);
        fx
    })
}

/// 按 C# 枚举名查表（生成表用它做键）
pub fn lookup_by_name(name: &str) -> Option<SpellFx> {
    SPELL_FX.iter().find(|(n, _)| *n == name).map(|(_, fx)| *fx)
}

/// 施法特效实体：跟随施法者播帧（原版 `Effect(..., ob)` 跟随对象）
#[derive(Component, Debug, Clone, Copy)]
pub struct SpellFxAnim {
    pub library: SpellFxLibrary,
    pub base: usize,
    pub frames: usize,
    pub t: f32,
    pub dur: f32,
    pub frame_ms: f32,
    /// 跟随的目标对象（0 = 不跟随，例如按地图坐标生成）
    pub follow_object_id: u32,
}

#[cfg(test)]
mod tests {
    use super::*;

    /// 门禁：施法弹道表必须与原版 `PlayerObject.cs` 的 CreateProjectile 一致（抽有代表性的几例）。
    ///
    /// 阳性对照（落地时实做）：把 FireBall 的 base 改成 999 → 本测试立即红。
    #[test]
    fn missile_table_matches_csharp_create_projectile() {
        let cases = [
            (
                Spell::FireBall,
                SpellFxLibrary::Magic,
                10usize,
                6usize,
                30u32,
                4usize,
            ),
            (Spell::GreatFireBall, SpellFxLibrary::Magic, 410, 6, 30, 4),
            (Spell::SoulFireBall, SpellFxLibrary::Magic, 1160, 3, 30, 7),
            (Spell::Hallucination, SpellFxLibrary::Magic, 1160, 3, 48, 7),
            (Spell::CatTongue, SpellFxLibrary::Magic3, 260, 6, 30, 4),
            (Spell::FrostCrunch, SpellFxLibrary::Magic2, 410, 4, 30, 6),
            (Spell::FireBounce, SpellFxLibrary::Magic, 410, 6, 30, 4),
            (Spell::MeteorShower, SpellFxLibrary::Magic, 410, 6, 30, 4),
        ];
        for (spell, library, base, frames, frame_ms, skip) in cases {
            let fx = spell_missile(spell).unwrap_or_else(|| panic!("{spell:?} 弹道必须在表里"));
            assert_eq!(
                (fx.library, fx.base, fx.frames, fx.frame_ms, fx.skip),
                (library, base, frames, frame_ms, skip),
                "{spell:?} 弹道与原版 CreateProjectile 不一致"
            );
        }
    }

    /// 门禁（结构）：弹道表里每条都必须是魔法库、帧数与帧长合理
    #[test]
    fn missile_table_is_structurally_sane() {
        assert!(
            SPELL_MISSILE.len() >= 10,
            "原版 MirAction.Spell 分支有 14 条施法弹道"
        );
        for (name, fx) in SPELL_MISSILE {
            assert!(
                (1..=32).contains(&fx.frames),
                "{name} 帧数不合理: {}",
                fx.frames
            );
            assert!(
                (10..=200).contains(&fx.frame_ms),
                "{name} 帧长不合理: {}",
                fx.frame_ms
            );
            assert!(fx.base < 6000, "{name} 起始帧越界: {}", fx.base);
        }
    }

    /// 门禁：**远程攻击**弹道表必须与原版 `MirAction.AttackRange1/2/3` 分支的
    /// `CreateProjectile` 逐字段一致（`DefaultArrow` = 普通弓射的 `case 5:` 那条）。
    ///
    /// `CrippleShot` 的值按 C# 现场赋值解析：`1930 + exFrameStart`，`Spell.CrippleShot → 400`
    /// ⇒ 2330（`PlayerObject.cs:2839-2841`）。
    ///
    /// 阳性对照（落地时实做）：把 `DefaultArrow` 的 base 改成 999 → 本测试立即红。
    #[test]
    fn range_missile_table_matches_csharp_attack_range() {
        let cases = [
            ("DefaultArrow", 1030usize, 5usize, 30u32, 5usize),
            ("StraightShot", 1210, 5, 30, 5),
            ("DoubleShot", 1030, 5, 30, 5),
            ("ElementalShot", 1690, 6, 30, 4),
            ("SummonSnakes", 2750, 5, 10, 5),
            ("Stonetrap", 2750, 5, 20, 5),
            ("DelayedExplosion", 1030, 5, 30, 5),
            ("CrippleShot", 2330, 5, 10, 5),
            ("NapalmShot", 2530, 6, 50, 4),
        ];
        assert_eq!(
            RANGE_MISSILE.len(),
            cases.len(),
            "原版 AttackRange 分支共 9 条弓/箭矢弹道（1 条默认 + 8 个技能）"
        );
        for (name, base, frames, frame_ms, skip) in cases {
            let (_, fx) = RANGE_MISSILE
                .iter()
                .find(|(n, _)| *n == name)
                .unwrap_or_else(|| panic!("{name} 必须在远程弹道表里"));
            assert_eq!(
                (fx.library, fx.base, fx.frames, fx.frame_ms, fx.skip),
                (SpellFxLibrary::Magic3, base, frames, frame_ms, skip),
                "{name} 与原版 AttackRange CreateProjectile 不一致"
            );
        }
    }

    /// 门禁：远程弹道查表把 `spell == 0`（普通弓射）映射到 `DefaultArrow`，技能按枚举名映射，
    /// 非远程技能（如表里没有的火球）返回 `None`（调用方据此退回占位弹道，不静默）。
    ///
    /// 阳性对照：把 `range_missile` 里 `spell == 0` 的特判删掉 → 本测试立即红。
    #[test]
    fn range_missile_lookup_covers_default_and_skills() {
        assert_eq!(
            range_missile(0).map(|m| m.base),
            Some(1030),
            "spell=0 = 普通弓射 → DefaultArrow[1030]"
        );
        assert_eq!(
            range_missile(Spell::StraightShot as u8).map(|m| m.base),
            Some(1210)
        );
        assert_eq!(
            range_missile(Spell::ElementalShot as u8).map(|m| (m.base, m.frames)),
            Some((1690, 6))
        );
        assert!(
            range_missile(Spell::FireBall as u8).is_none(),
            "火球不是远程攻击弹道（它在 MirAction.Spell 表里）"
        );
        assert!(range_missile(255).is_none(), "未知 spell 不应瞎映射");
    }

    /// 门禁（本轮核心）：表必须与原版 C# 对得上——抽几个有代表性的法术逐字段比对
    /// （值全部来自 `Client/MirObjects/PlayerObject.cs` MirAction.Spell 分支）。
    ///
    /// 阳性对照（落地时实做）：把 FireBall 的 start 改成 999 → 本测试立即红。
    #[test]
    fn spell_fx_matches_csharp_caster_branch() {
        let cases = [
            (
                Spell::FireBall,
                SpellFxLibrary::Magic,
                0usize,
                10usize,
                0usize,
                0u32,
            ),
            (Spell::Healing, SpellFxLibrary::Magic, 200, 10, 0, 0),
            (Spell::Repulsion, SpellFxLibrary::Magic, 900, 6, 0, 0),
            (Spell::GreatFireBall, SpellFxLibrary::Magic, 400, 10, 0, 0),
            (Spell::ThunderBolt, SpellFxLibrary::Magic2, 20, 3, 0, 300),
            (Spell::Teleport, SpellFxLibrary::Magic, 1590, 10, 0, 0),
            (Spell::FrostCrunch, SpellFxLibrary::Magic2, 400, 10, 0, 0),
            (Spell::Blizzard, SpellFxLibrary::Magic2, 1540, 8, 0, 0),
            (Spell::Haste, SpellFxLibrary::Magic2, 2140, 6, 10, 0),
            (
                Spell::BladeAvalanche,
                SpellFxLibrary::Magic2,
                740,
                15,
                20,
                0,
            ),
            (
                Spell::CrescentSlash,
                SpellFxLibrary::Magic2,
                2620,
                20,
                20,
                0,
            ),
            (
                Spell::HeavenlySword,
                SpellFxLibrary::Magic2,
                2230,
                8,
                10,
                800,
            ),
            (Spell::MoonMist, SpellFxLibrary::Magic3, 680, 25, 0, 1800),
        ];
        for (spell, library, start, frames, dir_step, interval_ms) in cases {
            let fx = spell_fx(spell, 0).unwrap_or_else(|| panic!("{spell:?} 必须在表里"));
            assert_eq!(
                (fx.library, fx.start, fx.frames, fx.dir_step, fx.interval_ms),
                (library, start, frames, dir_step, interval_ms),
                "{spell:?} 与原版 C# 不一致"
            );
        }
    }

    /// 门禁：按朝向取帧段的法术，方向不同 → 起始帧按 `dir_step` 平移（原版 `+ (int)Direction * 10`）
    #[test]
    fn directional_spells_shift_start_by_direction() {
        let d0 = spell_fx(Spell::Haste, 0).expect("Haste");
        let d5 = spell_fx(Spell::Haste, 5).expect("Haste");
        assert_eq!(d0.start, 2140);
        assert_eq!(d5.start, 2140 + 10 * 5);
        assert_eq!(d0.frames, d5.frames, "帧数不随朝向变");
    }

    /// 门禁（结构）：表里每条都要是魔法库、帧数合理、起始帧落在库内常见范围——
    /// 防止生成脚本改了 C# 解析规则后悄悄写出垃圾表。
    ///
    /// 阳性对照：把某条的 frames 改成 0 → 本测试红。
    #[test]
    fn spell_fx_table_is_structurally_sane() {
        assert!(
            SPELL_FX.len() >= 60,
            "原版 MirAction.Spell 分支有 65 条魔法库条目"
        );
        for (name, fx) in SPELL_FX {
            assert!(!name.is_empty(), "法术名不能为空");
            assert!(
                (1..=64).contains(&fx.frames),
                "{name} 帧数不合理: {}",
                fx.frames
            );
            assert!(fx.start < 6000, "{name} 起始帧越界: {}", fx.start);
            assert!(
                fx.interval_ms == 0 || fx.interval_ms <= 4000,
                "{name} 时长不合理: {}",
                fx.interval_ms
            );
        }
    }

    /// 门禁：表里的键必须都是本端 `Spell` 枚举的真实变体（否则查表永远命中不到）
    #[test]
    fn spell_fx_keys_resolve_to_real_spell_variants() {
        let mut known = std::collections::HashSet::new();
        for raw in 0..=255u8 {
            if let Ok(sp) = Spell::try_from(raw) {
                known.insert(format!("{sp:?}"));
            }
        }
        let mut missing: Vec<&str> = SPELL_FX
            .iter()
            .map(|(n, _)| *n)
            .filter(|n| !known.contains(*n))
            .collect();
        missing.sort_unstable();
        assert!(
            missing.is_empty(),
            "这些表项在本端 Spell 枚举里不存在（生成脚本或枚举名漂了）: {missing:?}"
        );
    }

    /// 门禁：时长换算——原版 `interval` 是**整段**时长，每帧 = interval/frames
    #[test]
    fn timing_uses_total_interval_not_per_frame() {
        let fx = SpellFx {
            library: SpellFxLibrary::Magic2,
            start: 20,
            frames: 3,
            dir_step: 0,
            interval_ms: 300,
        };
        let (total, per_frame) = fx.timing();
        assert!((total - 0.3).abs() < 1e-6, "整段应为 300ms");
        assert!((per_frame - 0.1).abs() < 1e-6, "每帧应为 100ms");

        // 原版写 Frame.Count * FrameInterval 的（本表记 0）→ 按 DEFAULT_FRAME_MS 换算
        let fx0 = SpellFx {
            interval_ms: 0,
            frames: 10,
            ..fx
        };
        let (total0, per0) = fx0.timing();
        assert!((total0 - 1.0).abs() < 1e-6);
        assert!((per0 - (DEFAULT_FRAME_MS as f32 / 1000.0)).abs() < 1e-6);
    }

    /// 门禁：对象特效表逐条对原版 `Client/MirScenes/GameScene.cs` 的 `ObjectEffect` switch
    /// （`:4711-4930`）。抽到的每一条都必须与 C# 字面量一致。
    ///
    /// 阳性对照（落地时实做）：把 Teleport 的 `start` 改成 1 → 本测试立即红。
    #[test]
    fn object_fx_table_matches_csharp_game_scene() {
        use crate::game::spell_effects::{AuraGroup, FxRepeat, FxTarget, FxWhen};
        let one = |name: &str| -> ObjectFx {
            let v = object_fx(name).unwrap_or_else(|| panic!("表里没有 {name}"));
            assert_eq!(v.len(), 1, "{name} 应只有一条 Effect");
            v[0]
        };
        // Teleport: `new Effect(Libraries.Magic, 1600, 10, 600, ob)`
        let t = one("Teleport");
        assert_eq!(t.lib, FxLib::Flat(SpellFxLibrary::Magic));
        assert_eq!((t.start, t.frames, t.interval_ms), (1600, 10, 600));
        // Healing: `Libraries.Magic, 370, 10, 800`
        let h = one("Healing");
        assert_eq!((h.start, h.frames, h.interval_ms), (370, 10, 800));
        // Reflect: `Libraries.Effect, 580, 10, 70`（唯一用 Effect 库的一条）
        let r = one("Reflect");
        assert_eq!(r.lib, FxLib::Flat(SpellFxLibrary::Effect));
        assert_eq!((r.start, r.frames, r.interval_ms), (580, 10, 70));
        // MagicShieldUp: `Libraries.Magic, 3890, 3, 600` + `Repeat = true`（循环到 Down）
        let s = one("MagicShieldUp");
        assert_eq!((s.start, s.frames, s.interval_ms), (3890, 3, 600));
        assert_eq!(s.repeat, FxRepeat::UntilDown(AuraGroup::MagicShield));
        // ElementalBarrierUp/Down
        let eu = one("ElementalBarrierUp");
        assert_eq!((eu.start, eu.frames, eu.interval_ms), (1890, 10, 2000));
        assert_eq!(eu.repeat, FxRepeat::UntilDown(AuraGroup::ElementalBarrier));
        let ed = one("ElementalBarrierDown");
        assert_eq!((ed.start, ed.frames, ed.interval_ms), (1910, 7, 1400));
        // Stunned: 632,10,1000 且 `Repeat = p.Time > 0`
        let st = one("Stunned");
        assert_eq!(st.repeat, FxRepeat::PacketTime);
        assert_eq!((st.start, st.frames, st.interval_ms), (632, 10, 1000));
        // DelayedExplosion: `1590 + (int)p.EffectType * 10` 且 stage != 2 才循环
        let de = one("DelayedExplosion");
        assert_eq!((de.start, de.step_effect_type), (1590, 10));
        assert_eq!(de.repeat, FxRepeat::StageNot2);
        // DeathCrawlerBreath: `272 + (int)ob.Direction * 4`，Blend = true
        let dc = one("DeathCrawlerBreath");
        assert_eq!((dc.start, dc.dir_step), (272, 4));
        // TurtleKing / GreatFoxSpirit 的随机档
        let tk = one("TurtleKing");
        assert_eq!((tk.start, tk.rand_step, tk.rand_count), (922, 12, 2));
        let gf = one("GreatFoxSpirit");
        assert_eq!((gf.start, gf.rand_step, gf.rand_count), (375, 20, 3));
        // 音效 id（C# `SoundManager.PlaySound(...)`，由生成器机械提取；未接前一律 None）
        assert_eq!(
            t.sound,
            Some(10110),
            "Teleport → SoundList.Teleport = 10110"
        );
        assert_eq!(gf.sound, Some(1345), "GreatFoxSpirit → Monster(134)*10+5");
        assert_eq!(
            ed.sound,
            Some(21315),
            "ElementalBarrierDown → 20000+131*10+5"
        );
        assert_eq!(
            eu.sound, None,
            "ElementalBarrierUp 原版不播（PlaySound 只在 Down）"
        );
        assert_eq!(s.sound, None, "MagicShieldUp 原版不播音效");
        // 随机档的取帧（纯函数）：rand=0 → start；rand=1 → start+step
        assert_eq!(tk.base_frame(0, 0, 0), 922);
        assert_eq!(tk.base_frame(0, 0, 1), 934);
        assert_eq!(gf.base_frame(0, 0, 2), 415);
        // Hemorrhage 三条同播
        let hm = object_fx("Hemorrhage").expect("Hemorrhage 在表里");
        assert_eq!(hm.len(), 3);
        let vals: Vec<(usize, usize, u32)> = hm
            .iter()
            .map(|f| (f.start, f.frames, f.interval_ms))
            .collect();
        assert_eq!(vals, vec![(0, 4, 400), (28, 6, 600), (46, 8, 800)]);
        // MPEater 两条，第二条 target = EffectType（原版 `ob2`）
        let mp = object_fx("MPEater").expect("MPEater 在表里");
        assert_eq!(mp.len(), 2);
        assert_eq!(
            (mp[0].start, mp[0].frames, mp[0].interval_ms),
            (2411, 19, 1900)
        );
        assert_eq!(mp[0].target, FxTarget::EffectType);
        assert_eq!(
            (mp[1].start, mp[1].frames, mp[1].interval_ms),
            (2400, 9, 900)
        );
        assert_eq!(mp[1].target, FxTarget::Owner);
        // KingGuard: `if (p.EffectType == 0)` → 753 else 763（两条，按 EffectType 分流）
        let kg = object_fx("KingGuard").expect("KingGuard 在表里");
        assert_eq!(kg.len(), 2);
        assert_eq!((kg[0].start, kg[0].when), (753, FxWhen::EffectTypeZero));
        assert_eq!((kg[1].start, kg[1].when), (763, FxWhen::EffectTypeNonZero));
        // Behemoth：两条挂 `ob.CurrentLocation`（地图位置，不跟随）
        let bh = object_fx("Behemoth").expect("Behemoth 在表里");
        assert_eq!(bh.len(), 2);
        assert!(bh.iter().all(|f| f.target == FxTarget::OwnerLocation));
        // 觉醒四条各两条，且带 `CMain.Time + p.DelayTime`
        for name in [
            "AwakeningSuccess",
            "AwakeningFail",
            "AwakeningHit",
            "AwakeningMiss",
        ] {
            let v = object_fx(name).unwrap_or_else(|| panic!("{name} 应在表里"));
            assert_eq!(v.len(), 2, "{name} 是两层（主层 + Blend=false 底图）");
            assert!(v.iter().all(|f| f.delay_from_packet));
            assert!(v.iter().any(|f| !f.blend));
        }
        // 怪物库：**资产索引必须是 C# `Monster` 枚举值**，不是本端枚举值
        // （本端全体比 C# 大 3：C# Guard=0 / 本端 Guard=3）。
        // 这条门禁就是当初漏掉的那个洞：本端 `StoningStatue = 204` 而资产是 `201.Lib`
        // （667 帧，含原版用的 632 帧）；用 204 会取到另一只 224 帧的小怪、632 直接越界，
        // 实机上表现为「对象特效一条都不出现」。
        let expect_monster_lib = [
            ("FurbolgWarriorCritical", "FurbolgWarrior", 406usize),
            ("RedMoonEvil", "RedMoonEvil", 62),
            ("GreatFoxSpirit", "GreatFoxSpirit", 134),
            ("TurtleKing", "TurtleKing", 187),
            ("Behemoth", "Behemoth", 158),
            ("Stunned", "StoningStatue", 201),
            ("IcePillar", "IcePillar", 231),
            ("KingGuard", "KingGuard", 252),
            ("FlamingMutantWeb", "FlamingMutant", 200),
            ("DeathCrawlerBreath", "DeathCrawler", 261),
        ];
        for (case, monster, lib) in expect_monster_lib {
            let v = object_fx(case).unwrap_or_else(|| panic!("{case} 应在表里"));
            for f in v {
                let FxLib::Monster { rust, lib: got } = f.lib else {
                    panic!("{case} 应走怪物库");
                };
                assert_eq!(
                    got, lib,
                    "{case}（{monster}）的资产索引应为 C# 值 {lib}，实得 {got}"
                );
                assert_eq!(
                    got,
                    rust as usize - 3,
                    "本端 Monster 枚举整体比 C# 大 3（C# Guard=0 / 本端 Guard=3）"
                );
            }
        }
        // `Critical` 被 C# 注释掉、`MagicShieldDown` 只做清理 → 表里是空切片（明确不画）
        assert_eq!(object_fx("Critical"), Some(&[][..]));
        assert_eq!(object_fx("MagicShieldDown"), Some(&[][..]));
        // C# 没有的 case → None（才允许退回占位表现）
        assert_eq!(object_fx("KingGuard2"), None);
    }

    /// 门禁：`SpellEffect` 枚举里**除 C# 真的没有 case 的四个**以外，其余都必须有表项。
    ///
    /// 这条守的是「枚举加了/改了名字，生成器没跟上」：表是按 `SpellEffect` 的 Debug 名查的，
    /// 名字漂了就静默查不到 → 实机退回方块（正是本次要修的那类现象）。
    #[test]
    fn object_fx_covers_every_spell_effect_case() {
        use mir2_shared::enums::SpellEffect;
        // C# `GameScene.ObjectEffect` 的 switch 里确实没有这四个：
        // `None`(3) 只是缺省值、`Mine`(15)/`Tester`(36) 走 `S.MapEffect`、`KingGuard2`(32) 无 case。
        let no_case = [
            SpellEffect::None,
            SpellEffect::Mine,
            SpellEffect::Tester,
            SpellEffect::KingGuard2,
        ];
        let mut missing: Vec<String> = Vec::new();
        let mut total = 0usize;
        for v in 0u8..=255 {
            let Ok(e) = SpellEffect::try_from(v) else {
                continue;
            };
            if no_case.contains(&e) {
                continue;
            }
            total += 1;
            let name = format!("{e:?}");
            if object_fx(&name).is_none() {
                missing.push(name);
            }
        }
        assert!(
            missing.is_empty(),
            "这些 SpellEffect 在表里缺失: {missing:?}"
        );
        assert_eq!(
            total,
            OBJECT_FX.len(),
            "表项数必须等于「SpellEffect 变体数 - C# 无 case 的 4 个」"
        );
    }
}
