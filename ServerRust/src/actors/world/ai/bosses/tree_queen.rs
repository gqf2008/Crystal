//! TreeQueen（树后）behavior
//!
//! C# 参考：Server/MirObjects/Monsters/TreeQueen.cs
//! 机制：不能移动、双独立定时器驱动根刺法术场、近战时冷却×4 鼓励远程、
//! 近战两形态（FireBombardment 3格AOE / PushAttack 推开5格）

use crate::actors::world::ai::behavior::MonsterBehavior;
use crate::actors::world::ai::ctx::AiCtx;
use crate::actors::world::MonsterState;
use crate::combat::poison::Poison;
use mir2_shared::enums::Spell;

/// C# `TreeQueen._rootCount = 5`（`SpawnRoots` 的 `Random.Next(1, 5)` ⇒ 1..4）
const ROOT_COUNT: i32 = 5;
/// C# `_rootSpreadMin/_rootSpreadMax = 5/15`（`Random.Next(5, 15)` ⇒ 5..14）
const ROOT_SPREAD_MIN: i32 = 5;
const ROOT_SPREAD_MAX: i32 = 15;
/// C# `SpawnMassRoots`：目标玩家脚下 7×7（±3）
const MASS_ROOT_RADIUS: i32 = 3;
/// C# `SpawnGroundRoots`：锚点 = 怪自身 ± `Random(-5, 6)`（`_groundrootSpread = 5`），锚点周围 5×5（±2）
const GROUND_ROOT_SPREAD: i32 = 5;
const GROUND_ROOT_RADIUS: i32 = 2;
/// C# `_nearMultiplier = 4`：近战（目标 2 格内）时根刺周期 ×4
const NEAR_MULTIPLIER: u64 = 4;
/// C# `Spawned()`：`_rootSpawnTime = now + 5s`、`_groundRootSpawnTime = now + 15s`
const ROOT_FIRST_TICKS: u64 = 50;
const GROUND_ROOT_FIRST_TICKS: u64 = 150;

/// #2857：C# `TreeQueen.SpawnRoots`（`TreeQueen.cs:148-182`）单个玩家的根生成（纯函数 + 注入 rng，便于单测）：
/// `count` 根、`distance` 环带（`player ± Random(-distance, distance+1)`）；
/// `Random(3)==0` 时直接落在玩家脚下并置 `hit`，该根**成功生成后 break**（`ValidPoint` 失败时只 continue，不 break）；
/// `start = Random(2000)`；`Value = Random(Random(MinMC, MaxMC))`（嵌套 roll，可为 0）。
///
/// 返回 `(x, y, value, start_delay_ms)`。
pub(crate) fn root_spawns(
    rng: &mut fastrand::Rng,
    px: i32,
    py: i32,
    count: i32,
    distance: i32,
    min_mc: i32,
    max_mc: i32,
    is_walkable: impl Fn(i32, i32) -> bool,
) -> Vec<(i32, i32, i32, u64)> {
    let mut out = Vec::new();
    let mut hit = false;
    for _ in 0..count {
        let (lx, ly) = if rng.i32(0..3) == 0 {
            hit = true;
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
        let upper = if max_mc > min_mc {
            rng.i32(min_mc..max_mc)
        } else {
            min_mc
        };
        let value = rng.i32(0..upper.max(1));
        let start = rng.i64(0..2000) as u64;
        out.push((lx, ly, value, start));
        if hit {
            break;
        }
    }
    out
}

/// #2857：C# `SpawnMassRoots`（`:200-236`）/`SpawnGroundRoots`（`:248-283`）共用的面积几何——
/// 以 `(ax, ay)` 为锚点的 `(2r+1)²` 格，跳过**怪自身所在格**与不可走格（C# `ValidPoint`）。
pub(crate) fn root_area_cells(
    ax: i32,
    ay: i32,
    radius: i32,
    boss: (i32, i32),
    is_walkable: impl Fn(i32, i32) -> bool,
) -> Vec<(i32, i32)> {
    // #2859：面积几何收敛到 helpers::area_cells 单一来源
    crate::actors::world::ai::helpers::area_cells(ax, ay, radius, Some(boss), is_walkable)
}

pub struct TreeQueenBehavior {
    root_spawn_tick: u64,
    ground_root_spawn_tick: u64,
    not_near: bool,
    spawned: bool,
}

impl Default for TreeQueenBehavior {
    fn default() -> Self {
        Self::new()
    }
}

impl TreeQueenBehavior {
    pub fn new() -> Self {
        Self {
            root_spawn_tick: 0,
            ground_root_spawn_tick: 0,
            not_near: true,
            spawned: false,
        }
    }
}

impl MonsterBehavior for TreeQueenBehavior {
    fn can_move(&self) -> bool {
        false
    }
    fn can_regen(&self) -> bool {
        false
    }
    fn on_poison(&mut self, _poison: Poison) -> bool {
        false
    } // 免疫毒

    fn on_spawned(&mut self, _monster: &mut MonsterState) {
        // C# TreeQueen.cs:289-296：5s 后撒根，15s 后撒地根
        self.spawned = true;
    }

    fn process_tick(&mut self, monster: &mut MonsterState, ctx: &mut AiCtx) {
        if !self.spawned {
            self.root_spawn_tick = ctx.tick_count + ROOT_FIRST_TICKS; // 5s
            self.ground_root_spawn_tick = ctx.tick_count + GROUND_ROOT_FIRST_TICKS; // 15s
            self.spawned = true;
        }

        // C# `ProcessTarget` 首行：`if (CurrentMap.Players.Count == 0) return;`
        let players_on_map = ctx
            .players
            .iter()
            .filter(|p| p.map_index == monster.map_index && p.hp > 0)
            .count();
        if players_on_map == 0 {
            return;
        }

        // 检测目标是否近战（2 格内 = C# `Attack` 里的 `ranged` 反义）。立即 copy 出来释放借用。
        let near_player = ctx
            .nearest_target(monster.x, monster.y, 2, monster.map_index)
            .copied();
        self.not_near = near_player.is_none();
        let near_mult = if self.not_near { 1 } else { NEAR_MULTIPLIER };

        // Root 定时器（C# TreeQueen.cs:298-310）
        if ctx.tick_count >= self.root_spawn_tick {
            if fastrand::i32(0..4) > 0 {
                // 3/4：单根（SpawnRoots）—— 对**每个**玩家各撒 1..4 根（C# :142-183）
                self.spawn_roots(monster, ctx);
            } else {
                // 1/4：群根（SpawnMassRoots）—— 随机玩家脚下 7×7（C# :186-237）
                self.spawn_mass_roots(monster, ctx);
            }
            let next = fastrand::i32(1..=3) as u64 * 10; // 1-3s = 10-30 ticks
            self.root_spawn_tick = ctx.tick_count + next * near_mult;
        }

        // GroundRoot 定时器（C# TreeQueen.cs:311-318）
        if ctx.tick_count >= self.ground_root_spawn_tick {
            // 每个玩家一个锚点（锚点 = 怪自身 ± Random(-5,6)），锚点周围 5×5（C# :239-286）
            self.spawn_ground_roots(monster, ctx);
            // C# `Envir.Random.Next(2, 3)` ⇒ 恒为 2 秒
            self.ground_root_spawn_tick = ctx.tick_count + 20 * near_mult;
        }

        // 近战攻击（C# TreeQueen.cs:54-100）：玩家 2 格内才攻击
        if near_player.is_some() {
            if ctx.tick_count >= monster.next_attack_tick {
                monster.next_attack_tick = ctx.tick_count + monster.ai_profile.attack_cooldown;
                let damage =
                    crate::combat::attack::get_attack_power(monster.min_dmg, monster.max_dmg, 0);
                // C# `if (damage == 0) return;`——本次不造成伤害（冷却已推进，与 C# 先设 ActionTime/AttackTime 一致）
                if damage == 0 {
                    return;
                }
                let is_fire = fastrand::i32(0..2) > 0; // C# `Random.Next(2) > 0` = 1/2
                if is_fire {
                    // FireBombardment：自身 3 格 AOE（C# `FindAllTargets(3, CurrentLocation)` 全员命中，MACAgility）
                    ctx.out_attacks
                        .push(crate::actors::world::ai::AttackAction::Aoe {
                            attacker_oid: monster.object_id,
                            center_x: monster.x,
                            center_y: monster.y,
                            radius: 3,
                            damage,
                            spell_id: 0,
                        });
                } else {
                    // PushAttack：C# `FindAllTargets(1, CurrentLocation)` ⇒ **只推不打**（`CompleteAttack` 的
                    // pushAttack 分支不调用 `Attacked`），方向 = 怪→目标，距离 5。动画用 `Type=1` 的
                    // ObjectAttack 广播（`Cells` 动作 + 空格集合 ⇒ 只发动画、不产生伤害）。
                    ctx.out_attacks
                        .push(crate::actors::world::ai::AttackAction::Cells {
                            attacker_oid: monster.object_id,
                            center_x: monster.x,
                            center_y: monster.y,
                            cells: Vec::new(),
                            damage: 0,
                            spell_id: 0,
                            attack_type: 1,
                        });
                    let mut ring: Vec<(i32, i32)> = Vec::with_capacity(9);
                    for dy in -1..=1 {
                        for dx in -1..=1 {
                            ring.push((monster.x + dx, monster.y + dy));
                        }
                    }
                    let pushed: Vec<(u64, i32, i32)> = ctx
                        .find_all_targets_in_cells(&ring, monster.map_index)
                        .iter()
                        .map(|p| (p.session_id, p.x, p.y))
                        .collect();
                    for (session_id, px, py) in pushed {
                        ctx.out_pushes.push(crate::actors::world::ai::PushPlayer {
                            session_id,
                            dir: crate::actors::world::ai::direction_towards(
                                monster.x, monster.y, px, py,
                            ),
                            distance: 5,
                        });
                    }
                }
            }
        }
    }
}

impl TreeQueenBehavior {
    /// C# `SpawnRoots`（`TreeQueen.cs:135-184`）——`count`/`distance` **每次调用只 roll 一次**，
    /// 对地图上**每个**玩家各生成 `count` 根（落点、延迟、伤害见 `root_spawns`）。
    /// 视觉：C# `TreeQueenRoot` 不在 `SpellObject.GetInfo` 的广播名单里（`SpellObject.cs:507-531`），
    /// 故 `show = false`（不发 `S.ObjectSpell`）。
    fn spawn_roots(&mut self, monster: &MonsterState, ctx: &mut AiCtx) {
        let mut rng = fastrand::Rng::new();
        let count = rng.i32(1..ROOT_COUNT);
        let distance = rng.i32(ROOT_SPREAD_MIN..ROOT_SPREAD_MAX);
        for p in ctx
            .players
            .iter()
            .filter(|p| p.map_index == monster.map_index && p.hp > 0)
        {
            let spawns = root_spawns(
                &mut rng,
                p.x,
                p.y,
                count,
                distance,
                monster.min_mc,
                monster.max_mc,
                |x, y| (ctx.is_walkable)(x, y),
            );
            for (x, y, value, start_delay_ms) in spawns {
                ctx.out_spell_fields
                    .push(crate::actors::world::ai::SpellFieldSpawn {
                        spell: Spell::TreeQueenRoot,
                        x,
                        y,
                        value,
                        duration_ms: 1500,
                        tick_ms: 2000,
                        caster_oid: monster.object_id,
                        caster_session: 0,
                        cells: Vec::new(),
                        show: false,
                        start_delay_ms,
                    });
            }
        }
    }

    /// C# `SpawnMassRoots`（`:186-237`）——随机一名地图玩家脚下 7×7（49 格，跳怪自身格与不可走格），
    /// `start = 500`、`TickSpeed = 1000`、`ExpireTime = 1500 + start`、`Show` 仅锚点格。
    fn spawn_mass_roots(&mut self, monster: &MonsterState, ctx: &mut AiCtx) {
        let players: Vec<(i32, i32)> = ctx
            .players
            .iter()
            .filter(|p| p.map_index == monster.map_index && p.hp > 0)
            .map(|p| (p.x, p.y))
            .collect();
        if players.is_empty() {
            return;
        }
        let mut rng = fastrand::Rng::new();
        let (ax, ay) = players[rng.usize(0..players.len())];
        let cells = root_area_cells(ax, ay, MASS_ROOT_RADIUS, (monster.x, monster.y), |x, y| {
            (ctx.is_walkable)(x, y)
        });
        if cells.is_empty() {
            return;
        }
        let value =
            crate::combat::attack::get_attack_power(monster.min_mc, monster.max_mc, monster.luck);
        ctx.out_spell_fields
            .push(crate::actors::world::ai::SpellFieldSpawn {
                spell: Spell::TreeQueenMassRoots,
                x: ax,
                y: ay,
                value,
                duration_ms: 1500,
                tick_ms: 1000,
                caster_oid: monster.object_id,
                caster_session: 0,
                cells,
                show: true,
                start_delay_ms: 500,
            });
    }

    /// C# `SpawnGroundRoots`（`:239-286`）——**每个玩家**一个锚点（锚点 = 怪自身 ± `Random(-5,6)`，
    /// 与玩家位置无关），锚点周围 5×5（25 格，跳怪自身格与不可走格）；`start = Random(4000)`、
    /// `TickSpeed = 1000`、`ExpireTime = 900 + start`、`Show` 仅锚点格。
    fn spawn_ground_roots(&mut self, monster: &MonsterState, ctx: &mut AiCtx) {
        let mut rng = fastrand::Rng::new();
        let player_count = ctx
            .players
            .iter()
            .filter(|p| p.map_index == monster.map_index && p.hp > 0)
            .count();
        for _ in 0..player_count {
            let ax = monster.x + rng.i32(-GROUND_ROOT_SPREAD..=GROUND_ROOT_SPREAD);
            let ay = monster.y + rng.i32(-GROUND_ROOT_SPREAD..=GROUND_ROOT_SPREAD);
            let cells = root_area_cells(
                ax,
                ay,
                GROUND_ROOT_RADIUS,
                (monster.x, monster.y),
                |x, y| (ctx.is_walkable)(x, y),
            );
            if cells.is_empty() {
                continue;
            }
            let value = crate::combat::attack::get_attack_power(
                monster.min_dmg,
                monster.max_dmg,
                monster.luck,
            );
            let start_delay_ms = rng.i64(0..4000) as u64;
            ctx.out_spell_fields
                .push(crate::actors::world::ai::SpellFieldSpawn {
                    spell: Spell::TreeQueenGroundRoots,
                    x: ax,
                    y: ay,
                    value,
                    duration_ms: 900,
                    tick_ms: 1000,
                    caster_oid: monster.object_id,
                    caster_session: 0,
                    cells,
                    show: true,
                    start_delay_ms,
                });
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// #2857：C# `SpawnRoots`（`TreeQueen.cs:139-140`）——`count ∈ [1,5)`、`distance ∈ [5,15)`
    #[test]
    fn root_count_and_spread_match_csharp() {
        let mut rng = fastrand::Rng::with_seed(5);
        let mut max_distance = i32::MIN;
        for _ in 0..300 {
            let count = rng.i32(1..ROOT_COUNT);
            let distance = rng.i32(ROOT_SPREAD_MIN..ROOT_SPREAD_MAX);
            assert!((1..5).contains(&count), "count ∈ [1,5)");
            assert!((5..15).contains(&distance), "distance ∈ [5,15)");
            max_distance = max_distance.max(distance);
        }
        assert!(max_distance > 5, "多次抽样应覆盖环带");
    }

    /// #2857：根落点几何——环带内（含 1/3 落玩家脚下）、`ValidPoint` 过滤、
    /// `Random(3)==0` 的"脚下根"生成后即 break（后续不再生成）
    #[test]
    fn root_spawns_geometry_matches_csharp() {
        let mut rng = fastrand::Rng::with_seed(11);
        let spawns = root_spawns(&mut rng, 100, 100, 4, 10, 10, 30, |_, _| true);
        assert!(spawns.len() <= 4);
        for (x, y, value, start) in &spawns {
            assert!((x - 100).abs() <= 10 && (y - 100).abs() <= 10);
            assert!(*value >= 0 && *value < 30);
            assert!(*start < 2000);
        }
        // 全图不可走 → 无落点
        assert!(root_spawns(&mut rng, 100, 100, 4, 10, 10, 30, |_, _| false).is_empty());
        // 只允许玩家脚下格可走：1/3 概率命中脚下并 break ⇒ 至多 1 个落点且必在脚下
        let mut rng = fastrand::Rng::with_seed(3);
        let only_foot = root_spawns(&mut rng, 100, 100, 4, 10, 10, 30, |x, y| {
            x == 100 && y == 100
        });
        assert!(only_foot.len() <= 1);
        assert!(only_foot.iter().all(|(x, y, _, _)| *x == 100 && *y == 100));
    }

    /// #2857：面积几何——`SpawnMassRoots` 7×7 = 49 格、`SpawnGroundRoots` 5×5 = 25 格，
    /// 两者都跳过**怪自身所在格**与不可走格
    #[test]
    fn root_area_cells_match_csharp() {
        let boss = (100, 100);

        // 怪在面积外：49 格全保留
        let mass = root_area_cells(200, 200, MASS_ROOT_RADIUS, boss, |_, _| true);
        assert_eq!(mass.len(), 49);
        assert!(mass.contains(&(197, 197)) && mass.contains(&(203, 203)));
        assert!(!mass.contains(&(196, 200)) && !mass.contains(&(204, 200)));

        // 怪自己在锚点上：49 - 1 = 48
        let mass_self = root_area_cells(100, 100, MASS_ROOT_RADIUS, boss, |_, _| true);
        assert_eq!(mass_self.len(), 48);
        assert!(!mass_self.contains(&boss));

        // 5×5 = 25（怪在外）
        let ground = root_area_cells(300, 300, GROUND_ROOT_RADIUS, boss, |_, _| true);
        assert_eq!(ground.len(), 25);
        assert!(ground.contains(&(298, 298)) && ground.contains(&(302, 302)));
        assert!(!ground.contains(&(297, 300)));

        // 不可走格被过滤
        let blocked = root_area_cells(200, 200, MASS_ROOT_RADIUS, boss, |x, y| (x + y) % 2 == 0);
        assert!(blocked.iter().all(|(x, y)| (x + y) % 2 == 0));
    }
}
