/// 持续性法术对象，对应 C# SpellObject
/// 代表地图上的火墙、暴风雪、毒云等持续效果
use std::time::Instant;

/// 法术对象实例
#[derive(Debug, Clone)]
pub struct SpellObject {
    /// 唯一 ID（用于网络包）
    pub object_id: u32,
    /// 法术类型
    pub spell: mir2_shared::enums::Spell,
    /// 施法者 object_id
    pub caster_id: u32,
    /// 施法者 session_id
    pub caster_session: u64,
    /// 所在地图索引（防跨图命中）
    pub map_index: u16,
    /// 施法位置（格子坐标）
    pub x: i32,
    pub y: i32,
    /// 法术覆盖格集合（C# Map.cs 地面法术落点；空=单格）
    pub cells: Vec<(i32, i32)>,
    /// 朝向
    pub direction: u8,
    /// 每跳间隔（毫秒），对应 C# TickSpeed
    pub tick_interval_ms: u64,
    /// 上次处理时间
    pub last_tick: Instant,
    /// 创建时间
    pub created_at: Instant,
    /// 过期时间（毫秒），对应 C# TickTime
    pub expires_at_ms: u64,
    /// 当前值（伤害/治疗量）
    pub value: i32,
    /// 每跳伤害/治疗
    pub tick_value: i32,
    /// 是否可见
    pub visible: bool,
    /// 施法者是否已死亡
    pub caster_dead: bool,
    /// 法术等级
    pub spell_level: u8,
    /// 额外数据（如火墙的 MAC 伤害、毒云的 SC 加成）
    pub bonus: i32,
    /// 是否已爆炸（陷阱类）
    pub detonated: bool,
    /// 关联的陷阱 ID（用于连锁爆炸）
    pub linked_trap_id: Option<u32>,
    /// 目标对象 ID（DelayedExplosion 挂毒用）
    pub target_id: Option<u32>,
}

impl SpellObject {
    pub fn new(
        object_id: u32,
        spell: mir2_shared::enums::Spell,
        caster_id: u32,
        caster_session: u64,
        map_index: u16,
        x: i32,
        y: i32,
        expires_at_ms: u64,
        tick_value: i32,
        tick_interval_ms: u64,
        spell_level: u8,
        bonus: i32,
    ) -> Self {
        let now = Instant::now();
        Self {
            object_id,
            spell,
            caster_id,
            caster_session,
            map_index,
            x,
            y,
            direction: 0,
            tick_interval_ms,
            last_tick: now,
            created_at: now,
            expires_at_ms,
            value: 0,
            tick_value,
            visible: true,
            caster_dead: false,
            spell_level,
            bonus,
            detonated: false,
            linked_trap_id: None,
            target_id: None,
            cells: Vec::new(),
        }
    }

    /// 设置覆盖格（C# Map.cs：FireWall 十字 5 / PoisonCloud 3x3 / Blizzard·MeteorStrike 5x5）
    pub fn set_cells(&mut self, cells: Vec<(i32, i32)>) {
        self.cells = cells;
    }

    /// 是否需要过期
    pub fn is_expired(&self, elapsed_ms: u64) -> bool {
        elapsed_ms >= self.expires_at_ms
    }
}

/// #2855：C# 起始延迟（`DelayedAction(DelayedType.Spawn, Envir.Time + start, ob)`）的等价折算——
/// 对象在 `start_delay_ms` 之后才存在，其寿命从"对象存在"起算（`ExpireTime = Envir.Time + duration + start`），
/// 且生成后首跳立即结算（`SpellObject.StartTime = 0`）。
///
/// 本端对象在创建时即物化，故：
/// - `expires_ms = start_delay_ms + duration_ms`（相对创建时刻的总寿命）
/// - `last_tick_shift_ms = start_delay_ms - tick_ms`（把 `last_tick` 前移/后移，使首跳恰好落在 `start_delay_ms`；
///   为负表示 `last_tick` 落在创建时刻之前）
pub(crate) fn delayed_spell_timing(
    start_delay_ms: u64,
    duration_ms: u64,
    tick_ms: u64,
) -> (u64, i64) {
    (
        start_delay_ms + duration_ms,
        start_delay_ms as i64 - tick_ms as i64,
    )
}

/// 法术参数配置
struct SpellConfig {
    spell: mir2_shared::enums::Spell,
    duration_ms: u64,
    tick_interval_ms: u64,
    tick_value: i32,
}

fn make_spell_config(
    spell: mir2_shared::enums::Spell,
    _level: u8,
    stat: i32,
    value: i32,
) -> SpellConfig {
    use mir2_shared::enums::Spell;
    match spell {
        // C# Map.cs:1133：ExpireTime=(10+value/2)s、TickSpeed=2000，Value=magic.GetDamage
        Spell::FireWall => SpellConfig {
            spell: Spell::FireWall,
            duration_ms: ((10 + value / 2).max(1) as u64) * 1000,
            tick_interval_ms: 2000,
            tick_value: value.max(1),
        },
        // C# Map.cs:1475：ExpireTime=6000ms、TickSpeed=1000
        Spell::PoisonCloud => SpellConfig {
            spell: Spell::PoisonCloud,
            duration_ms: 6_000,
            tick_interval_ms: 1000,
            tick_value: value.max(1),
        },
        // C# Map.cs:1670：ExpireTime=3000ms、TickSpeed=440
        Spell::Blizzard => SpellConfig {
            spell: Spell::Blizzard,
            duration_ms: 3_000,
            tick_interval_ms: 440,
            tick_value: value.max(1),
        },
        // C# Map.cs:1731：ExpireTime=3000ms、TickSpeed=440
        Spell::MeteorStrike => SpellConfig {
            spell: Spell::MeteorStrike,
            duration_ms: 3_000,
            tick_interval_ms: 440,
            tick_value: value.max(1),
        },
        Spell::HealingCircle => SpellConfig {
            spell: Spell::HealingCircle,
            duration_ms: 15_000,
            tick_interval_ms: 1500,
            tick_value: stat.max(5) * 2 + 25,
        },
        // C# Map.cs:1952：ExpireTime=(10+value/2)s、TickSpeed=500
        Spell::ExplosiveTrap => SpellConfig {
            spell: Spell::ExplosiveTrap,
            duration_ms: ((10 + value / 2).max(1) as u64) * 1000,
            tick_interval_ms: 500,
            tick_value: value.max(1),
        },
        Spell::DelayedExplosion => SpellConfig {
            spell: Spell::DelayedExplosion,
            // expires_at_ms 在施法时按距离覆盖（距离*50+500ms），tick 间隔调小以便及时引爆
            duration_ms: 60_000,
            tick_interval_ms: 100,
            tick_value: stat.max(30) * 2,
        },
        Spell::Portal => SpellConfig {
            spell: Spell::Portal,
            // C# `Map.cs:2186-2195`：ExpireTime = now + duration*1000、TickSpeed = 2000；
            // Value（通行次数）与 ExpireTime 在 `combat.rs` 施法时按 `magic.Level` 覆盖（#2851）。
            duration_ms: 60_000,
            tick_interval_ms: 2_000,
            tick_value: 0,
        },
        _ => SpellConfig {
            spell,
            duration_ms: 10_000,
            tick_interval_ms: 2000,
            tick_value: stat.max(1) * 2,
        },
    }
}

/// 创建持久法术对象
pub fn create_persistent_spell(
    object_id: u32,
    caster_id: u32,
    caster_session: u64,
    map_index: u16,
    x: i32,
    y: i32,
    level: u8,
    stat: i32,
    value: i32,
    spell: mir2_shared::enums::Spell,
) -> SpellObject {
    let cfg = make_spell_config(spell, level, stat, value);
    SpellObject::new(
        object_id,
        cfg.spell,
        caster_id,
        caster_session,
        map_index,
        x,
        y,
        cfg.duration_ms,
        cfg.tick_value,
        cfg.tick_interval_ms,
        level,
        stat,
    )
}

/// C# 地面法术落点（Server/MirEnvir/Map.cs）：FireWall 中心+4 正交（5）、PoisonCloud 3x3（9）、
/// Blizzard/MeteorStrike 5x5（25）；其余单格。
pub fn spell_cells_for(spell: mir2_shared::enums::Spell, x: i32, y: i32) -> Vec<(i32, i32)> {
    use mir2_shared::enums::Spell;
    let mut cells = Vec::new();
    match spell {
        Spell::FireWall => {
            cells.push((x, y));
            for (dx, dy) in [(0, -1), (1, 0), (0, 1), (-1, 0)] {
                cells.push((x + dx, y + dy));
            }
        }
        Spell::PoisonCloud => {
            for dy in -1..=1 {
                for dx in -1..=1 {
                    cells.push((x + dx, y + dy));
                }
            }
        }
        Spell::Blizzard | Spell::MeteorStrike => {
            for dy in -2..=2 {
                for dx in -2..=2 {
                    cells.push((x + dx, y + dy));
                }
            }
        }
        _ => cells.push((x, y)),
    }
    cells
}

#[cfg(test)]
mod tests {
    use super::spell_cells_for;
    use mir2_shared::enums::Spell;

    /// #2855：起始延迟折算——总寿命 = start + duration；`last_tick` 偏移 = start - tick
    #[test]
    fn delayed_spell_timing_matches_csharp() {
        // 无延迟：总寿命 = duration，last_tick 前移一个节拍（首跳落在 tick_ms）
        assert_eq!(super::delayed_spell_timing(0, 2000, 2000), (2000, -2000));
        assert_eq!(super::delayed_spell_timing(0, 6000, 1000), (6000, -1000));
        // HellLord 震击：start = Random(5000)、duration 2000、tick 500
        assert_eq!(super::delayed_spell_timing(0, 2000, 500), (2000, -500));
        assert_eq!(super::delayed_spell_timing(4999, 2000, 500), (6999, 4499));
        // HornedCommander 落石：start = 500 + Random(0,200)、duration = loops*500+500、tick 2000
        assert_eq!(super::delayed_spell_timing(500, 3000, 2000), (3500, -1500));
        assert_eq!(super::delayed_spell_timing(699, 5500, 2000), (6199, -1301));
        // start > tick：last_tick 落在创建时刻之后（首跳仍在 start）
        assert_eq!(super::delayed_spell_timing(5000, 1000, 500), (6000, 4500));
    }

    #[test]
    fn firewall_cells_cross() {
        let cells = spell_cells_for(Spell::FireWall, 10, 10);
        assert_eq!(cells.len(), 5);
        for c in [(10, 10), (10, 9), (11, 10), (10, 11), (9, 10)] {
            assert!(cells.contains(&c), "missing {c:?}");
        }
    }

    #[test]
    fn poison_cloud_3x3() {
        let cells = spell_cells_for(Spell::PoisonCloud, 0, 0);
        assert_eq!(cells.len(), 9);
        let mut sorted = cells.clone();
        sorted.sort_unstable();
        sorted.dedup();
        assert_eq!(sorted.len(), 9);
    }

    /// #1864：地面法术时长/跳速对齐（C# Map.cs：PoisonCloud 6s/1000、Blizzard/MeteorStrike 3s/440）
    #[test]
    fn ground_spell_duration_tick_aligned() {
        let mk = |sp| super::create_persistent_spell(1, 1, 1, 0, 0, 0, 3, 100, 30, sp);
        let pc = mk(Spell::PoisonCloud);
        assert_eq!(pc.expires_at_ms, 6000);
        assert_eq!(pc.tick_interval_ms, 1000);
        let bl = mk(Spell::Blizzard);
        assert_eq!(bl.expires_at_ms, 3000);
        assert_eq!(bl.tick_interval_ms, 440);
        let ms = mk(Spell::MeteorStrike);
        assert_eq!(ms.expires_at_ms, 3000);
        assert_eq!(ms.tick_interval_ms, 440);
        // #1868：每跳伤害 = C# value（magic.GetDamage），非 stat 近似
        assert_eq!(pc.tick_value, 30);
        assert_eq!(bl.tick_value, 30);
        assert_eq!(ms.tick_value, 30);
        // FireWall/ExplosiveTrap 时长 = (10+value/2)s；value=30 → 25s
        let fw = mk(Spell::FireWall);
        assert_eq!(fw.expires_at_ms, 25000);
        assert_eq!(fw.tick_value, 30);
        let et = mk(Spell::ExplosiveTrap);
        assert_eq!(et.expires_at_ms, 25000);
        assert_eq!(et.tick_interval_ms, 500);
        assert_eq!(et.tick_value, 30);
    }

    #[test]
    fn blizzard_meteor_5x5() {
        for sp in [Spell::Blizzard, Spell::MeteorStrike] {
            let cells = spell_cells_for(sp, 5, 5);
            assert_eq!(cells.len(), 25, "{sp:?}");
            let mut sorted = cells.clone();
            sorted.sort_unstable();
            sorted.dedup();
            assert_eq!(sorted.len(), 25, "{sp:?}");
        }
    }
}
