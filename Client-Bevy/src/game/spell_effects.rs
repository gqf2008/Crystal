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
use mir2_shared::enums::Spell;
// 生成表里写的是 `library: Magic` 这样的短名（对齐 C# 的 Libraries.Magic），故把变体引进作用域
#[allow(unused_imports)]
use SpellFxLibrary::{Magic, Magic2, Magic3};

/// 原版写 `Frame.Count * FrameInterval` 时的兜底每帧时长（ms）
pub const DEFAULT_FRAME_MS: u32 = 100;

/// 特效库（原版 Libraries.Magic / Magic2 / Magic3）
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum SpellFxLibrary {
    Magic,
    Magic2,
    Magic3,
}

impl SpellFxLibrary {
    pub fn library(self) -> LibraryName {
        match self {
            SpellFxLibrary::Magic => LibraryName::Magic,
            SpellFxLibrary::Magic2 => LibraryName::Magic2,
            SpellFxLibrary::Magic3 => LibraryName::Magic3,
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
}
