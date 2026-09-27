//! 怪物远程攻击弹道表 —— **由工具生成，请勿手改**。
//!
//! 生成器：`py -3.12 tools/acceptance/csharp_golden/monster_projectile_gen.py --write`
//! 来源：`Client/MirObjects/MonsterObject.cs` 的 `MirAction.AttackRange1/2/3` 分支里
//! `CreateProjectile(baseIndex, library, blend, count, interval, skip, ...)` 的调用点。
//!
//! 键 `monster` 与怪物库索引都取 **C# `Monster` 枚举值**（`Shared/Enums.cs` 的
//! `enum Monster : ushort`）：它既是怪物的图像索引，也就是 'Data/Monster/{:03}.Lib' 的
//! 资产索引。本端的 `ActorAppearance::monster_type` 来自服务端 DB 的 `image` 字段，
//! 同样是 C# 值 —— 两者同源，可直接比较（注意本端 `Monster` 枚举整体比 C# 大 3，
//! 不能拿本端枚举值当资产索引用，见 `spell_effects::FxLib::Monster` 的说明）。
//!
//! 为什么需要（2026-09-28，owner 反馈「有些魔法是个黄色方框」）：玩家侧占位弹道已抑制，
//! 但怪物侧走的是这张**独立**的表，此前没移植 ⇒ 怪物远程攻击仍画占位方块。
use crate::game::spell_effects::{MissileFx, SpellFxLibrary};
use crate::resources::libraries::LibraryName;

/// 弹道帧的来源库
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum MissileLib {
    /// 扁平库（C# `Libraries.Magic/Magic2/Magic3/Dragon`…）
    Flat(LibraryName),
    /// 怪物库（C# `Libraries.Monsters[(ushort)Monster.X]`）：`asset` = C# `Monster` 值
    Monster { asset: u16 },
    /// 本端没有这个库的资产（如 `Siege`）：保留记录、渲染时退回占位并打日志
    Unavailable { csharp_lib: &'static str },
}

/// 一条怪物弹道：`(怪物图像索引, 动作 1/2/3, 触发帧, 弹道参数)`
#[derive(Clone, Copy, Debug)]
pub struct MonsterMissile {
    /// `ActorAppearance::monster_type`（= C# `Monster` 枚举值 = 图像索引）
    pub monster: i16,
    /// 1/2/3 = C# `MirAction.AttackRange1/2/3`
    pub range: u8,
    /// 原版在该动作的第几帧生成弹道（`switch (FrameIndex)`）
    pub frame: u8,
    /// 库内起始帧
    pub base: usize,
    /// 帧数
    pub frames: usize,
    /// 每帧时长（ms；C# `CreateProjectile` 的 `interval`）
    pub frame_ms: u32,
    /// C# `CreateProjectile` 的 `skip`（按朝向取帧段的步长；本端按帧序播，仅记录备查）
    pub skip: usize,
    pub lib: MissileLib,
}

/// C# 源文件里的行号（可追溯；`monster_projectile_gen.py` 生成时写入）
pub const MONSTER_MISSILE_SRC_LINES: &[u32] = &[
    2546, 2550, 2556, 2571, 2583, 2596, 2631, 2679, 3426, 2701, 2705, 2726, 2734, 2659, 2741, 2635, 2639, 2764, 2785, 2798, 3439, 2801, 2819, 2833, 2497, 2845, 2909, 3475, 2919, 3179, 3191, 2953, 2956, 2968, 3508, 3082, 3540, 3528, 3012, 3025, 3038, 2730, 3265, 3102, 3059, 3072, 2738, 3268, 3563, 3211, 3221, 3114, 2478, 2528, 3400, 2533, 3147, 2708, 3159, 3170
];

pub const MONSTER_MISSILES: &[MonsterMissile] = &[
    // AxeSkeleton @ MonsterObject.cs:2546
    MonsterMissile { monster: 24, range: 1, frame: 4, base: 224, frames: 3, frame_ms: 30, skip: 0, lib: MissileLib::Monster { asset: 24 } },
    // Dark @ MonsterObject.cs:2550
    MonsterMissile { monster: 28, range: 1, frame: 4, base: 224, frames: 3, frame_ms: 30, skip: 0, lib: MissileLib::Monster { asset: 28 } },
    // BoneArcher @ MonsterObject.cs:2556
    MonsterMissile { monster: 92, range: 1, frame: 4, base: 224, frames: 1, frame_ms: 30, skip: 0, lib: MissileLib::Monster { asset: 64 } },
    // BoneLord @ MonsterObject.cs:2571
    MonsterMissile { monster: 93, range: 1, frame: 4, base: 784, frames: 6, frame_ms: 30, skip: 0, lib: MissileLib::Monster { asset: 93 } },
    // LeftGuard @ MonsterObject.cs:2583
    MonsterMissile { monster: 100, range: 1, frame: 4, base: 10, frames: 6, frame_ms: 30, skip: 4, lib: MissileLib::Flat(LibraryName::Magic) },
    // FrostTiger @ MonsterObject.cs:2596
    MonsterMissile { monster: 102, range: 1, frame: 4, base: 410, frames: 4, frame_ms: 30, skip: 6, lib: MissileLib::Flat(LibraryName::Magic2) },
    // CrossbowOma @ MonsterObject.cs:2631
    MonsterMissile { monster: 120, range: 1, frame: 4, base: 38, frames: 1, frame_ms: 30, skip: 6, lib: MissileLib::Monster { asset: 120 } },
    // WhiteFoxman @ MonsterObject.cs:2679
    MonsterMissile { monster: 129, range: 1, frame: 4, base: 1160, frames: 3, frame_ms: 30, skip: 7, lib: MissileLib::Flat(LibraryName::Magic) },
    // WhiteFoxman @ MonsterObject.cs:3426
    MonsterMissile { monster: 129, range: 2, frame: 4, base: 1160, frames: 3, frame_ms: 30, skip: 7, lib: MissileLib::Flat(LibraryName::Magic) },
    // HedgeKekTal @ MonsterObject.cs:2701
    MonsterMissile { monster: 135, range: 1, frame: 4, base: 38, frames: 4, frame_ms: 30, skip: 6, lib: MissileLib::Monster { asset: 135 } },
    // BigHedgeKekTal @ MonsterObject.cs:2705
    MonsterMissile { monster: 136, range: 1, frame: 4, base: 38, frames: 4, frame_ms: 30, skip: 6, lib: MissileLib::Monster { asset: 136 } },
    // ArcherGuard @ MonsterObject.cs:2726
    MonsterMissile { monster: 139, range: 1, frame: 4, base: 38, frames: 3, frame_ms: 30, skip: 6, lib: MissileLib::Monster { asset: 139 } },
    // ArcherGuard2 @ MonsterObject.cs:2734
    MonsterMissile { monster: 141, range: 1, frame: 4, base: 38, frames: 3, frame_ms: 30, skip: 6, lib: MissileLib::Monster { asset: 139 } },
    // PoisonHugger @ MonsterObject.cs:2659
    MonsterMissile { monster: 160, range: 1, frame: 4, base: 208, frames: 1, frame_ms: 30, skip: 0, lib: MissileLib::Monster { asset: 160 } },
    // FinialTurtle @ MonsterObject.cs:2741
    MonsterMissile { monster: 186, range: 1, frame: 4, base: 272, frames: 3, frame_ms: 30, skip: 0, lib: MissileLib::Monster { asset: 186 } },
    // DarkCrossbowOma @ MonsterObject.cs:2635
    MonsterMissile { monster: 192, range: 1, frame: 4, base: 38, frames: 1, frame_ms: 30, skip: 6, lib: MissileLib::Monster { asset: 192 } },
    // DarkWingedOma @ MonsterObject.cs:2639
    MonsterMissile { monster: 193, range: 1, frame: 4, base: 224, frames: 6, frame_ms: 30, skip: 0, lib: MissileLib::Monster { asset: 121 } },
    // WitchDoctor @ MonsterObject.cs:2764
    MonsterMissile { monster: 220, range: 1, frame: 4, base: 313, frames: 5, frame_ms: 30, skip: 0, lib: MissileLib::Monster { asset: 220 } },
    // TrollBomber @ MonsterObject.cs:2785
    MonsterMissile { monster: 235, range: 1, frame: 4, base: 208, frames: 4, frame_ms: 40, skip: 0, lib: MissileLib::Monster { asset: 235 } },
    // TrollStoner @ MonsterObject.cs:2798
    MonsterMissile { monster: 236, range: 1, frame: 4, base: 208, frames: 4, frame_ms: 40, skip: 0, lib: MissileLib::Monster { asset: 236 } },
    // TrollKing @ MonsterObject.cs:3439
    MonsterMissile { monster: 237, range: 2, frame: 4, base: 294, frames: 4, frame_ms: 40, skip: 0, lib: MissileLib::Monster { asset: 237 } },
    // FlameMage @ MonsterObject.cs:2801
    MonsterMissile { monster: 239, range: 1, frame: 4, base: 544, frames: 3, frame_ms: 20, skip: 0, lib: MissileLib::Monster { asset: 239 } },
    // FlameAssassin @ MonsterObject.cs:2819
    MonsterMissile { monster: 241, range: 1, frame: 4, base: 592, frames: 3, frame_ms: 20, skip: 0, lib: MissileLib::Monster { asset: 241 } },
    // AncientBringer @ MonsterObject.cs:2833
    MonsterMissile { monster: 272, range: 1, frame: 4, base: 688, frames: 4, frame_ms: 50, skip: 0, lib: MissileLib::Monster { asset: 272 } },
    // Jar2 @ MonsterObject.cs:2497
    MonsterMissile { monster: 281, range: 1, frame: 3, base: 688, frames: 4, frame_ms: 50, skip: 0, lib: MissileLib::Monster { asset: 281 } },
    // RestlessJar @ MonsterObject.cs:2845
    MonsterMissile { monster: 283, range: 1, frame: 4, base: 476, frames: 2, frame_ms: 100, skip: 0, lib: MissileLib::Monster { asset: 283 } },
    // CannibalTentacles @ MonsterObject.cs:2909
    MonsterMissile { monster: 295, range: 1, frame: 4, base: 472, frames: 8, frame_ms: 100, skip: 0, lib: MissileLib::Monster { asset: 295 } },
    // TucsonGeneral @ MonsterObject.cs:3475
    MonsterMissile { monster: 296, range: 2, frame: 4, base: 592, frames: 9, frame_ms: 30, skip: 0, lib: MissileLib::Monster { asset: 296 } },
    // PeacockSpider @ MonsterObject.cs:2919
    MonsterMissile { monster: 308, range: 1, frame: 4, base: 664, frames: 5, frame_ms: 100, skip: 0, lib: MissileLib::Monster { asset: 308 } },
    // OmaCannibal @ MonsterObject.cs:3179
    MonsterMissile { monster: 311, range: 1, frame: 5, base: 360, frames: 6, frame_ms: 60, skip: 0, lib: MissileLib::Monster { asset: 311 } },
    // OmaMage @ MonsterObject.cs:3191
    MonsterMissile { monster: 315, range: 1, frame: 5, base: 392, frames: 8, frame_ms: 80, skip: 0, lib: MissileLib::Monster { asset: 315 } },
    // FloatingWraith @ MonsterObject.cs:2953
    MonsterMissile { monster: 325, range: 1, frame: 4, base: 248, frames: 2, frame_ms: 20, skip: 0, lib: MissileLib::Monster { asset: 325 } },
    // AvengingSpirit @ MonsterObject.cs:2956
    MonsterMissile { monster: 329, range: 1, frame: 4, base: 368, frames: 4, frame_ms: 40, skip: 0, lib: MissileLib::Monster { asset: 329 } },
    // AvengingWarrior @ MonsterObject.cs:2968
    MonsterMissile { monster: 330, range: 1, frame: 4, base: 312, frames: 5, frame_ms: 50, skip: 0, lib: MissileLib::Monster { asset: 330 } },
    // KingHydrax @ MonsterObject.cs:3508
    MonsterMissile { monster: 337, range: 2, frame: 4, base: 473, frames: 4, frame_ms: 50, skip: 0, lib: MissileLib::Monster { asset: 337 } },
    // HornedArcher @ MonsterObject.cs:3082
    MonsterMissile { monster: 341, range: 1, frame: 4, base: 360, frames: 3, frame_ms: 50, skip: 0, lib: MissileLib::Monster { asset: 341 } },
    // HornedArcher @ MonsterObject.cs:3540
    MonsterMissile { monster: 341, range: 2, frame: 4, base: 414, frames: 3, frame_ms: 50, skip: 0, lib: MissileLib::Monster { asset: 341 } },
    // ColdArcher @ MonsterObject.cs:3528
    MonsterMissile { monster: 342, range: 2, frame: 4, base: 394, frames: 3, frame_ms: 50, skip: 0, lib: MissileLib::Monster { asset: 342 } },
    // ChieftainArcher @ MonsterObject.cs:3012
    MonsterMissile { monster: 356, range: 1, frame: 4, base: 312, frames: 5, frame_ms: 50, skip: 0, lib: MissileLib::Monster { asset: 356 } },
    // ChieftainArcher @ MonsterObject.cs:3025
    MonsterMissile { monster: 356, range: 1, frame: 4, base: 398, frames: 5, frame_ms: 50, skip: 0, lib: MissileLib::Monster { asset: 356 } },
    // ChieftainArcher @ MonsterObject.cs:3038
    MonsterMissile { monster: 356, range: 1, frame: 4, base: 484, frames: 5, frame_ms: 50, skip: 0, lib: MissileLib::Monster { asset: 356 } },
    // SpittingToad @ MonsterObject.cs:2730
    MonsterMissile { monster: 360, range: 1, frame: 4, base: 280, frames: 6, frame_ms: 30, skip: 0, lib: MissileLib::Monster { asset: 360 } },
    // FrozenArcher @ MonsterObject.cs:3265
    MonsterMissile { monster: 365, range: 1, frame: 6, base: 264, frames: 5, frame_ms: 80, skip: 0, lib: MissileLib::Monster { asset: 365 } },
    // WaterDragon @ MonsterObject.cs:3102
    MonsterMissile { monster: 371, range: 1, frame: 4, base: 800, frames: 6, frame_ms: 60, skip: 0, lib: MissileLib::Monster { asset: 371 } },
    // BlackTortoise @ MonsterObject.cs:3059
    MonsterMissile { monster: 372, range: 1, frame: 4, base: 444, frames: 6, frame_ms: 60, skip: 0, lib: MissileLib::Monster { asset: 372 } },
    // DragonArcher @ MonsterObject.cs:3072
    MonsterMissile { monster: 375, range: 1, frame: 4, base: 416, frames: 5, frame_ms: 50, skip: 0, lib: MissileLib::Monster { asset: 375 } },
    // ArcherGuard3 @ MonsterObject.cs:2738
    MonsterMissile { monster: 378, range: 1, frame: 4, base: 104, frames: 3, frame_ms: 30, skip: 0, lib: MissileLib::Monster { asset: 378 } },
    // FrozenMagician @ MonsterObject.cs:3268
    MonsterMissile { monster: 382, range: 1, frame: 6, base: 560, frames: 6, frame_ms: 30, skip: 0, lib: MissileLib::Monster { asset: 382 } },
    // FrozenMagician @ MonsterObject.cs:3563
    MonsterMissile { monster: 382, range: 2, frame: 8, base: 734, frames: 6, frame_ms: 30, skip: 0, lib: MissileLib::Monster { asset: 382 } },
    // SnowYeti @ MonsterObject.cs:3211
    MonsterMissile { monster: 383, range: 1, frame: 5, base: 560, frames: 6, frame_ms: 20, skip: 0, lib: MissileLib::Monster { asset: 383 } },
    // DarkSpirit @ MonsterObject.cs:3221
    MonsterMissile { monster: 386, range: 1, frame: 5, base: 512, frames: 6, frame_ms: 60, skip: 0, lib: MissileLib::Monster { asset: 386 } },
    // AntCommander @ MonsterObject.cs:3114
    MonsterMissile { monster: 394, range: 1, frame: 4, base: 432, frames: 3, frame_ms: 100, skip: 0, lib: MissileLib::Monster { asset: 394 } },
    // PurpleFaeFlower @ MonsterObject.cs:2478
    MonsterMissile { monster: 403, range: 1, frame: 2, base: 331, frames: 6, frame_ms: 60, skip: 0, lib: MissileLib::Monster { asset: 403 } },
    // FurbolgArcher @ MonsterObject.cs:2528
    MonsterMissile { monster: 407, range: 1, frame: 4, base: 344, frames: 5, frame_ms: 30, skip: 0, lib: MissileLib::Monster { asset: 407 } },
    // FurbolgArcher @ MonsterObject.cs:3400
    MonsterMissile { monster: 407, range: 2, frame: 4, base: 429, frames: 5, frame_ms: 30, skip: 0, lib: MissileLib::Monster { asset: 407 } },
    // FurbolgGuard @ MonsterObject.cs:2533
    MonsterMissile { monster: 410, range: 1, frame: 4, base: 391, frames: 1, frame_ms: 30, skip: 0, lib: MissileLib::Monster { asset: 410 } },
    // WizardScroll @ MonsterObject.cs:3147
    MonsterMissile { monster: 416, range: 1, frame: 4, base: 300, frames: 5, frame_ms: 50, skip: 0, lib: MissileLib::Monster { asset: 416 } },
    // EvilMir @ MonsterObject.cs:2708
    MonsterMissile { monster: 900, range: 1, frame: 4, base: 60, frames: 10, frame_ms: 10, skip: 0, lib: MissileLib::Flat(LibraryName::Dragon) },
    // Catapult @ MonsterObject.cs:3159
    MonsterMissile { monster: 940, range: 1, frame: 4, base: 256, frames: 4, frame_ms: 40, skip: 0, lib: MissileLib::Unavailable { csharp_lib: "Siege" } },
    // ChariotBallista @ MonsterObject.cs:3170
    MonsterMissile { monster: 941, range: 1, frame: 4, base: 38, frames: 3, frame_ms: 30, skip: 6, lib: MissileLib::Unavailable { csharp_lib: "Siege" } },
];

/// 查表：`(怪物图像索引, 动作 1/2/3)` → 弹道参数（同一组合取第一条）
pub fn monster_missile(monster_type: i16, range: u8) -> Option<&'static MonsterMissile> {
    MONSTER_MISSILES
        .iter()
        .find(|m| m.monster == monster_type && m.range == range)
}

#[cfg(test)]
mod tests {
    use super::*;

    /// 表来自 C# 的 60 个调用点，条数变了要说明原因（防止生成器静默漏抽）
    #[test]
    fn table_matches_csharp_call_sites() {
        assert_eq!(MONSTER_MISSILES.len(), 60);
        assert_eq!(MONSTER_MISSILE_SRC_LINES.len(), MONSTER_MISSILES.len());
    }

    /// 抽样核对（数值直接来自 C#）：AxeSkeleton=24 → 自己的怪物库 224 起 3 帧；
    /// BoneArcher=92 用的是 **ZumaArcher 的库**；LeftGuard=100 走 Magic 库。
    #[test]
    fn monster_missile_lookup_samples() {
        let axe = monster_missile(24, 1).expect("AxeSkeleton Range1 应有弹道");
        assert_eq!((axe.base, axe.frames), (224, 3));
        assert_eq!(axe.lib, MissileLib::Monster { asset: 24 });
        let bone = monster_missile(92, 1).expect("BoneArcher Range1");
        assert_eq!(bone.lib, MissileLib::Monster { asset: 64 }, "C# 用的是 ZumaArcher 的库");
        let guard = monster_missile(100, 1).expect("LeftGuard Range1");
        assert_eq!(guard.lib, MissileLib::Flat(LibraryName::Magic));
        // 本端没有 `Siege` 资产：表里保留记录，渲染时退回占位（不静默）
        let siege = monster_missile(940, 1).expect("TucsonGeneral Range1");
        assert!(matches!(siege.lib, MissileLib::Unavailable { .. }));
        let h1 = monster_missile(341, 1).expect("HornedArcher Range1");
        let h2 = monster_missile(341, 2).expect("HornedArcher Range2");
        assert_ne!((h1.base, h1.frames), (h2.base, h2.frames), "两条动作的弹道应不同");
        assert!(monster_missile(24, 2).is_none(), "表里没有的组合必须返回 None（由调用方决定占位）");
    }
}
