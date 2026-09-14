//! HellLord（地狱领主）behavior
//!
//! C# 参考：Server/MirObjects/Monsters/HellLord.cs
//! 机制：不能移动、5阶段(stage 0..4，靠 Knight 被杀推进)、stage<4 完全无敌、
//! 自身不直接攻击（空 Attack），纯靠召唤 Knight + 撒 Bomb + Quake 法术场

use crate::actors::world::ai::behavior::MonsterBehavior;
use crate::actors::world::ai::ctx::AiCtx;
use crate::actors::world::MonsterState;
use crate::combat::poison::Poison;
use mir2_shared::enums::Spell;

/// C# `HellLord._rageDelay = Settings.Minute * 2`——Knight 死后 2 分钟才召唤下一只（100ms/tick）
const RAGE_DURATION_TICKS: u64 = 1200;
/// C# `ProcessTarget` 末尾 `ActionTime = Envir.Time + 600`——HellLord 的 AI 节拍固定 600ms
const ACTION_TICKS: u64 = 6;
/// C# `_quakeCount = 5`（`SpawnQuakes` 的 `Random.Next(1, 狂暴 ? 10 : 5)`，上界开区间）
const QUAKE_COUNT: i32 = 5;
/// C# `_quakeSpreadMin/_quakeSpreadMax = 5/15`（`Random.Next(5, 15)` = 5..14）
const QUAKE_SPREAD_MIN: i32 = 5;
const QUAKE_SPREAD_MAX: i32 = 15;
/// C# `_bombSpreadMin/_bombSpreadMax = 5/20`（`Random.Next(5, 20)` = 5..19）
const BOMB_SPREAD_MIN: i32 = 5;
const BOMB_SPREAD_MAX: i32 = 20;
/// C# `SpawnKnight`：`PointMove(CurrentLocation, MirDirection.DownLeft, 12)`（DownLeft = (-1, +1)）
const KNIGHT_FRONT_STEPS: i32 = 12;
/// C# `SpawnKnight` 的 HellKnight1..4（按 `_stage` 取）
const KNIGHT_NAMES: [&str; 4] = ["HellKnight1", "HellKnight2", "HellKnight3", "HellKnight4"];
/// C# `SpawnBomb` 的 HellBomb1..3（按 `Random(3)` 取）
const BOMB_NAMES: [&str; 3] = ["HellBomb1", "HellBomb2", "HellBomb3"];

/// C# `SpawnKnight`（`HornedCommander` 不适用）落点基准：`PointMove(CurrentLocation, DownLeft, 12)`。
/// DownLeft 在 8 向枚举里是 5 ⇒ `(dx, dy) = (-1, +1)`。
pub(crate) fn knight_front(x: i32, y: i32) -> (i32, i32) {
    (x - KNIGHT_FRONT_STEPS, y + KNIGHT_FRONT_STEPS)
}

/// C# `HellLord.SpawnQuakes`（`HellLord.cs:117-118`）——`count` 与 `distance` **每次调用只 roll 一次**，
/// 对本次调用里的所有玩家共用：
/// `count = Random(1, 狂暴 ? _quakeCount*2 : _quakeCount)`（上界开区间 ⇒ 1..4 / 1..9）、
/// `distance = Random(_quakeSpreadMin, _quakeSpreadMax)` = 5..14。
pub(crate) fn quake_count_and_spread(rng: &mut fastrand::Rng, raged: bool) -> (i32, i32) {
    let limit = if raged { QUAKE_COUNT * 2 } else { QUAKE_COUNT };
    (
        rng.i32(1..limit),
        rng.i32(QUAKE_SPREAD_MIN..QUAKE_SPREAD_MAX),
    )
}

/// C# `HellLord.SpawnQuakes`（`HellLord.cs:124-152`）的单玩家震击生成（纯函数 + 注入 rng，便于单测）：
/// 每个落点 `player ± Random(-distance, distance+1)`，`Random(10) == 0` 时直接落在玩家脚下；
/// `ValidPoint` 失败（越界 / 不可走）→ 跳过（本端 `is_walkable` 含边界判定）；
/// `start = Random(5000)`（ms）；`Value = Random(Random(MinDC, MaxDC))`（嵌套 roll，可为 0）。
///
/// 返回 `(x, y, value, start_delay_ms)`。
pub(crate) fn quake_spawns(
    rng: &mut fastrand::Rng,
    px: i32,
    py: i32,
    count: i32,
    distance: i32,
    min_dmg: i32,
    max_dmg: i32,
    is_walkable: impl Fn(i32, i32) -> bool,
) -> Vec<(i32, i32, i32, u64)> {
    let mut out = Vec::new();
    for _ in 0..count {
        let (lx, ly) = if rng.i32(0..10) == 0 {
            (px, py)
        } else {
            (
                px + rng.i32(-distance..=distance),
                py + rng.i32(-distance..=distance),
            )
        };
        if !is_walkable(lx, ly) {
            continue;
        }
        // C# `Value = Envir.Random.Next(Envir.Random.Next(MinDC, MaxDC))`：内层上界开区间，外层 0..upper-1
        let upper = if max_dmg > min_dmg {
            rng.i32(min_dmg..max_dmg)
        } else {
            min_dmg
        };
        let value = rng.i32(0..upper.max(1));
        let start = rng.i64(0..5000) as u64;
        out.push((lx, ly, value, start));
    }
    out
}

pub struct HellLordBehavior {
    stage: u8, // 0..4
    begin: bool,
    raged: bool,
    rage_end_tick: u64,
}

impl Default for HellLordBehavior {
    fn default() -> Self {
        Self::new()
    }
}

impl HellLordBehavior {
    pub fn new() -> Self {
        Self {
            stage: 0,
            begin: true,
            raged: false,
            rage_end_tick: 0,
        }
    }

    /// Knight 被杀时推进阶段（由外部回调）
    pub fn advance_stage(&mut self, current_tick: u64) {
        self.raged = true;
        self.rage_end_tick = current_tick + RAGE_DURATION_TICKS;
        self.stage = (self.stage + 1).min(4);
    }
}

impl MonsterBehavior for HellLordBehavior {
    fn can_move(&self) -> bool {
        false
    }
    fn can_regen(&self) -> bool {
        false
    }
    fn on_poison(&mut self, _poison: Poison) -> bool {
        false
    } // 完全免疫毒

    fn as_any_mut(&mut self) -> Option<&mut dyn std::any::Any> {
        Some(self)
    }

    fn on_attacked(&mut self, damage: i32) -> i32 {
        // C# HellLord.cs:47-64：stage<4 时完全无敌
        if self.stage < 4 {
            0
        } else {
            damage
        }
    }

    fn process_tick(&mut self, monster: &mut MonsterState, ctx: &mut AiCtx) {
        // C# Process：玩家全部离开则复位
        let players_on_map = ctx
            .players
            .iter()
            .filter(|p| p.map_index == monster.map_index && p.hp > 0)
            .count();
        if players_on_map == 0 && self.stage > 0 {
            self.stage = 0;
            self.begin = true;
            return;
        }
        // C# `ProcessTarget` 首行：`if (CurrentMap.Players.Count == 0) return;`
        if players_on_map == 0 {
            return;
        }

        if ctx.tick_count < monster.next_attack_tick {
            return;
        }

        // C# `ActionTime = Envir.Time + 600`——HellLord 无视 `AttackSpeed`/ai_profile，固定 600ms 节拍
        monster.next_attack_tick = ctx.tick_count + ACTION_TICKS;

        // ===== 阶段推进检测 =====
        // C# 语义：Knight 被玩家杀死 → KnightKilled() → stage += 1 + 狂暴 2min（由死亡回调 advance_stage 触发）；
        // 狂暴到期（且 stage < 4）或开场 → 召唤当前阶段 Knight（C# `ProcessTarget` 的同一条 `||` 条件）
        if (self.raged && ctx.tick_count >= self.rage_end_tick && self.stage < 4) || self.begin {
            self.begin = false;
            self.raged = false;
            self.spawn_knight(monster, ctx);
        }

        // C# `if (Envir.Random.Next(_bombChance) == 0 || _raged) SpawnBomb();`（`_bombChance = 3`）
        if self.raged || fastrand::i32(0..3) == 0 {
            self.spawn_bombs(monster, ctx);
        }

        // C# 无条件 `SpawnQuakes()`
        self.spawn_quakes(monster, ctx);
    }
}

impl HellLordBehavior {
    /// C# `SpawnKnight`（`HellLord.cs:188-228`）——按 `_stage` 取 HellKnight1..4，
    /// 落点 = `PointMove(CurrentLocation, DownLeft, 12)` ± `Random(-10, 10)`，最多 50 次试到可走格；
    /// 50 次都失败则不召唤。`knight.Owner/Lord = this` 由召唤登记（`summoner_oid`）体现。
    fn spawn_knight(&mut self, monster: &MonsterState, ctx: &mut AiCtx) {
        let Some(name) = KNIGHT_NAMES.get(self.stage as usize) else {
            return;
        };
        let (fx, fy) = knight_front(monster.x, monster.y);
        for _ in 0..50 {
            let lx = fx + fastrand::i32(-10..10);
            let ly = fy + fastrand::i32(-10..10);
            if (ctx.is_walkable)(lx, ly) {
                ctx.out_summons.push(crate::actors::world::ai::BossSummon {
                    monster_name: (*name).to_string(),
                    x: lx,
                    y: ly,
                    is_slave: true,
                    summoner_oid: Some(monster.object_id),
                });
                return;
            }
        }
    }

    /// C# `SpawnBomb`（`:157-186`）——`distance = Random(5, 20)`，对**每个**玩家各撒一个炸弹
    /// （种类 `Random(3)`：HellBomb1/2/3；炸弹不进 SlaveList）。
    fn spawn_bombs(&mut self, monster: &MonsterState, ctx: &mut AiCtx) {
        let distance = fastrand::i32(BOMB_SPREAD_MIN..BOMB_SPREAD_MAX);
        for p in ctx
            .players
            .iter()
            .filter(|p| p.map_index == monster.map_index && p.hp > 0)
        {
            let bomb = BOMB_NAMES[fastrand::usize(0..BOMB_NAMES.len())];
            ctx.out_summons.push(crate::actors::world::ai::BossSummon {
                monster_name: bomb.to_string(),
                x: p.x + fastrand::i32(-distance..=distance),
                y: p.y + fastrand::i32(-distance..=distance),
                is_slave: false,
                summoner_oid: Some(monster.object_id),
            });
        }
    }

    /// C# `SpawnQuakes`（`:115-155`）——`count`/`distance` 每次调用只 roll 一次，对地图上**每个**玩家
    /// 各生成 `count` 个震击对象（`start = Random(5000)`ms 延迟、寿命 2000ms、500ms 一跳、`Caster = null`）。
    fn spawn_quakes(&mut self, monster: &MonsterState, ctx: &mut AiCtx) {
        let mut rng = fastrand::Rng::new();
        let (count, distance) = quake_count_and_spread(&mut rng, self.raged);
        for p in ctx
            .players
            .iter()
            .filter(|p| p.map_index == monster.map_index && p.hp > 0)
        {
            let spawns = quake_spawns(
                &mut rng,
                p.x,
                p.y,
                count,
                distance,
                monster.min_dmg,
                monster.max_dmg,
                |x, y| (ctx.is_walkable)(x, y),
            );
            for (x, y, value, start_delay_ms) in spawns {
                let spell = if rng.i32(0..2) == 0 {
                    Spell::MapQuake1
                } else {
                    Spell::MapQuake2
                };
                ctx.out_spell_fields
                    .push(crate::actors::world::ai::SpellFieldSpawn {
                        spell,
                        x,
                        y,
                        value,
                        duration_ms: 2000,
                        tick_ms: 500,
                        caster_oid: monster.object_id,
                        caster_session: 0,
                        cells: Vec::new(),
                        show: true,
                        start_delay_ms,
                    });
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// #2855：C# `SpawnQuakes`（`HellLord.cs:117-118`）——`count ∈ [1,5)`（狂暴 `[1,10)`）、
    /// `distance ∈ [5,15)`；落点 `player ± distance`（含 1/10 落在玩家脚下）。
    #[test]
    fn quake_geometry_matches_csharp() {
        let mut rng = fastrand::Rng::with_seed(7);
        // 断言用 C# 字面量（`_quakeCount = 5`、`_quakeSpreadMin/Max = 5/15`），不引用本端常量——
        // 否则常量被改动时断言会与被测对象一起漂移（自洽失明）；多轮抽样才能覆盖取值范围
        let mut max_distance_seen = i32::MIN;
        for _ in 0..300 {
            let (count, distance) = quake_count_and_spread(&mut rng, false);
            assert!((1..5).contains(&count), "非狂暴 count ∈ [1,5)");
            assert!((5..15).contains(&distance), "distance ∈ [5,15)");
            max_distance_seen = max_distance_seen.max(distance);
            let (count_raged, distance_raged) = quake_count_and_spread(&mut rng, true);
            assert!((1..10).contains(&count_raged), "狂暴 count ∈ [1,10)");
            assert!((5..15).contains(&distance_raged), "狂暴 distance ∈ [5,15)");
        }
        assert!(max_distance_seen > 5, "多次抽样应覆盖环带而非常量");

        // 落点分布在 player ± distance 内（切比雪夫），value 与 start 落在 C# 取值域
        let (count, distance) = quake_count_and_spread(&mut rng, false);
        let spawns = quake_spawns(&mut rng, 100, 100, count, distance, 10, 30, |_, _| true);
        assert_eq!(spawns.len(), count as usize);
        for (x, y, value, start) in &spawns {
            assert!((x - 100).abs() <= distance && (y - 100).abs() <= distance);
            assert!(0 <= *value && *value < 30);
            assert!(*start < 5000);
        }

        // `ValidPoint` 过滤：不可走格不计入；全不可走 → 无落点
        let blocked = quake_spawns(&mut rng, 100, 100, 8, 10, 10, 30, |x, y| (x + y) % 2 == 0);
        assert!(blocked.iter().all(|(x, y, _, _)| (x + y) % 2 == 0));
        assert!(quake_spawns(&mut rng, 100, 100, 8, 10, 10, 30, |_, _| false).is_empty());
    }

    /// #2855：C# `Value = Random(Random(MinDC, MaxDC))` 是嵌套 roll——取值可到 0（该震击对象不出伤害，
    /// 对应 `SpellObject.ProcessSpell` 的 `if (Value == 0) return;`），且恒小于 `MaxDC`。
    #[test]
    fn quake_value_nested_roll() {
        let mut rng = fastrand::Rng::with_seed(3);
        let spawns = quake_spawns(&mut rng, 0, 0, 200, 1, 0, 5, |_, _| true);
        assert!(
            spawns.iter().any(|(_, _, v, _)| *v == 0),
            "嵌套 roll 必须能取到 0"
        );
        assert!(spawns.iter().all(|(_, _, v, _)| *v < 5));
    }

    /// #2855：C# `SpawnKnight` 落点基准 `PointMove(CurrentLocation, DownLeft, 12)`（DownLeft = (-1,+1)）
    #[test]
    fn knight_front_matches_csharp() {
        assert_eq!(knight_front(50, 60), (38, 72));
    }
}
