//! DarkOmaKing（暗黑奥玛之王）behavior
//!
//! C# 参考：Server/MirObjects/Monsters/DarkOmaKing.cs
//! 机制：可移动、AttackRange=6、双独立定时器驱动：
//!   - _OrbTime (20s)：召唤 2 个 PowerBead（在 8 距离内随机点）
//!   - _MassThunderTime (10s + 0-5s 抖动)：MassThunder AOE（自身 5 格 AOE，MAC 伤害）
//! 攻击：
//!   - 近战 (距离<=3)：3/4 普攻 DC / 1/4 FullmoonAttack 三连击（16 格 + 推 1）+ 前方 3 格 DarkOmaKingNuke 法术场
//!   - 远程 (>3)：1/3 概率弹道远程攻击（MAC）
//! 死亡：清理 SlaveList（PowerBead）

use crate::actors::world::ai::behavior::MonsterBehavior;
use crate::actors::world::ai::ctx::AiCtx;
use crate::actors::world::ai::helpers::*;
use crate::actors::world::MonsterState;
use mir2_shared::enums::Spell;

/// 攻击视野范围（C# AttackRange=6，但 ProcessTarget 用 ViewRange 寻敌）
const VIEW_RANGE: i32 = 20;
/// C# `DarkOmaKing.AttackRange = 6`——`InAttackRange()` 的判定半径（Orb/MassThunder 只在攻击距离内触发）
const ATTACK_RANGE: i32 = 6;
/// C# Nuke 法术场参数（`DarkOmaKing.cs:114-131`）：`start = 3000`、`ExpireTime = 900 + start`、`TickSpeed = 1000`
const NUKE_START_MS: u64 = 3000;
const NUKE_DURATION_MS: u64 = 900;
const NUKE_TICK_MS: u64 = 1000;

/// C# `DarkOmaKing.InAttackRange()`（`:28-31`）——同图且切比雪夫距离 ≤ `AttackRange(6)`。
/// Orb/MassThunder 定时器写在 `Attack()` 内部，故也受此门控。
pub(crate) fn in_attack_range(dist: i32) -> bool {
    dist <= ATTACK_RANGE
}
/// 近战判定距离（C# InRange(CurrentLocation, Target, 3)）
const MELEE_RANGE: i32 = 3;
/// Orb（PowerBead）召唤周期：20s = 200 ticks（C# _OrbTime）
const ORB_INTERVAL_TICKS: u64 = 200;
/// MassThunder 基础周期：10s = 100 ticks（C# _MassThunderTime）
const MASS_THUNDER_BASE_TICKS: u64 = 100;
/// MassThunder 随机抖动上限：5s = 50 ticks
const MASS_THUNDER_JITTER_TICKS: u64 = 50;
/// MassThunder AOE 半径（C# FindAllTargets(5, CurrentLocation)）
const MASS_THUNDER_RADIUS: i32 = 5;
/// PowerBead 召唤点距自身的最大距离（C# distance=8）
const ORB_SPAWN_DISTANCE: i32 = 8;
/// 每次召唤 PowerBead 数量（C# count=2）
const ORB_SPAWN_COUNT: usize = 2;

pub struct DarkOmaKingBehavior {
    /// 下次召唤 PowerBead 的 tick（对齐 C# _OrbTime）
    next_bead_tick: u64,
    /// 下次 MassThunder 的 tick（对齐 C# _MassThunderTime）
    next_thunder_tick: u64,
}

impl Default for DarkOmaKingBehavior {
    fn default() -> Self {
        Self::new()
    }
}

impl DarkOmaKingBehavior {
    pub fn new() -> Self {
        Self {
            // C# 构造函数：_MassThunderTime = Envir.Time + 10000; _OrbTime = Envir.Time + 20000
            // on_spawned 会用真实 tick_count 重置
            next_bead_tick: 0,
            next_thunder_tick: 0,
        }
    }
}

impl MonsterBehavior for DarkOmaKingBehavior {
    fn on_spawned(&mut self, _monster: &mut MonsterState) {
        // 占位：真实 tick_count 在首次 process_tick 时懒初始化（on_spawned 拿不到 ctx）
        self.next_bead_tick = 0;
        self.next_thunder_tick = 0;
    }

    fn process_tick(&mut self, monster: &mut MonsterState, ctx: &mut AiCtx) {
        // 懒初始化定时器（对齐 C# 构造函数：Thunder +10s, Orb +20s）
        if self.next_thunder_tick == 0 {
            self.next_thunder_tick = ctx.tick_count + MASS_THUNDER_BASE_TICKS;
        }
        if self.next_bead_tick == 0 {
            self.next_bead_tick = ctx.tick_count + ORB_INTERVAL_TICKS;
        }

        // 无目标时不行动（C# ProcessTarget：Target==null 直接 return）
        let target = match ctx.nearest_target(monster.x, monster.y, VIEW_RANGE, monster.map_index) {
            Some(t) => *t,
            None => return,
        };
        monster.target_session = Some(target.session_id);

        let dist = max_distance(monster.x, monster.y, target.x, target.y);

        // #2857：C# 的 Orb/MassThunder 定时器写在 `Attack()` **内部**，而 `Attack()` 只会在
        // `InAttackRange()`（距离 ≤ 6）且 `CanAttack` 成立时被调用——超出攻击距离时先走近，不落雷/不召唤。
        if !in_attack_range(dist) {
            if ctx.tick_count >= monster.next_move_tick {
                let (nx, ny, dir) = step_toward(monster.x, monster.y, target.x, target.y);
                ctx.out_moves.push((monster.object_id, nx, ny, dir));
                monster.next_move_tick = ctx.tick_count + monster.ai_profile.move_interval;
                monster.ai_state = crate::actors::world::MonsterAiState::Chase;
            }
            return;
        }

        // C# `CanAttack` 门控（`AttackTime`/`ActionTime` 都须已过）
        if ctx.tick_count < monster.next_attack_tick {
            return;
        }

        // ---- 定时器驱动：PowerBead 召唤（C# DarkOmaKing.cs:44-68）----
        if ctx.tick_count >= self.next_bead_tick {
            self.next_bead_tick = ctx.tick_count + ORB_INTERVAL_TICKS;
            // C# count=2，每个 bead 在 ±distance=8 内随机点，避开自身和目标位置
            for _ in 0..ORB_SPAWN_COUNT {
                // 4 次尝试（C# attempts=4）
                for _ in 0..4 {
                    let dx = fastrand::i32(-ORB_SPAWN_DISTANCE..=ORB_SPAWN_DISTANCE);
                    let dy = fastrand::i32(-ORB_SPAWN_DISTANCE..=ORB_SPAWN_DISTANCE);
                    let sx = monster.x + dx;
                    let sy = monster.y + dy;
                    // 避开自身和目标（C# location == CurrentLocation || == Target 则 continue）
                    if (sx == monster.x && sy == monster.y) || (sx == target.x && sy == target.y) {
                        continue;
                    }
                    ctx.out_summons.push(crate::actors::world::ai::BossSummon {
                        monster_name: "PowerBead".to_string(),
                        x: sx,
                        y: sy,
                        is_slave: true, // 加入 slave_list，Boss 死亡时清理
                        summoner_oid: Some(monster.object_id),
                    });
                    break;
                }
            }
        }

        // ---- 定时器驱动：MassThunder AOE（C# DarkOmaKing.cs:70-85）----
        if ctx.tick_count >= self.next_thunder_tick {
            // C# 10s + Random(0,5000)
            let jitter = fastrand::u64(0..MASS_THUNDER_JITTER_TICKS);
            self.next_thunder_tick = ctx.tick_count + MASS_THUNDER_BASE_TICKS + jitter;
            // C# MassThunder 分支在 `return` 前已设 `ActionTime = Envir.Time + AttackSpeed + 300`
            // （虽然跳过了末尾的 `AttackTime` 更新，但 `CanAttack` 要求两个计时器都过 ⇒ 有效冷却 = +AttackSpeed+300）
            monster.next_attack_tick = ctx.tick_count + monster.ai_profile.attack_cooldown + 3;

            // MAC 伤害 AOE（C# GetAttackPower(MinMC, MaxMC)）
            let damage = crate::combat::attack::get_attack_power(
                monster.min_mc,
                monster.max_mc,
                monster.luck,
            );
            // C# `if (damage == 0) return;`（冷却已推进，本次不落雷）
            if damage == 0 {
                return;
            }
            ctx.out_attacks
                .push(crate::actors::world::ai::AttackAction::Aoe {
                    attacker_oid: monster.object_id,
                    center_x: monster.x,
                    center_y: monster.y,
                    radius: MASS_THUNDER_RADIUS,
                    damage,
                    spell_id: 0,
                });
            return; // C# MassThunder 分支后直接 return（ActionTime 已推迟）
        }

        // ---- 攻击 / 追击（C# Attack + ProcessTarget）----
        // C# `ranged = CurrentLocation == Target.CurrentLocation || !InRange(CurrentLocation, Target, 3)`
        // ⇒ **同格也算远程**（distance == 0 走远程分支）
        if dist > 0 && dist <= MELEE_RANGE {
            // C# 每个分支先设 `ActionTime = Envir.Time + AttackSpeed + 300`（近战/远程）后再判伤害
            monster.next_attack_tick = ctx.tick_count + monster.ai_profile.attack_cooldown + 3;
            // C# DarkOmaKing.cs:87-133：ranged=false 时
            if fastrand::i32(0..4) > 0 {
                // 3/4：普攻 DC（Type=0）
                let damage = crate::combat::attack::get_attack_power(
                    monster.min_dmg,
                    monster.max_dmg,
                    monster.luck,
                );
                // C# `if (damage == 0) return;`——本次不出伤（冷却已推进）
                if damage > 0 {
                    ctx.out_attacks
                        .push(crate::actors::world::ai::AttackAction::Melee {
                            attacker_oid: monster.object_id,
                            target_session: target.session_id,
                            damage,
                            spell_id: 0,
                            attack_type: 0,
                        });
                }
            } else {
                // 1/4：FullmoonAttack 三连击 + DarkOmaKingNuke 法术场（Type=1）
                let damage = crate::combat::attack::get_attack_power(
                    monster.min_dmg,
                    monster.max_dmg,
                    monster.luck,
                );
                // C# `ActionTime = Envir.Time + AttackSpeed + 3400`（Nuke 分支冷却更长）
                monster.next_attack_tick = ctx.tick_count + monster.ai_profile.attack_cooldown + 34;
                if damage <= 0 {
                    return;
                }
                // C# FullmoonAttack(damage, delay, ACAgility, pushDistance=1, distance=2)：16 格（8 方向 × 2 圈）
                let dir = direction_towards(monster.x, monster.y, target.x, target.y);
                monster.direction = dir;
                let cells = eight_dir_rings(monster.x, monster.y, 2);
                // C#：延迟 500/1700/2500ms 分 3 次结算（DelayedAction DelayedType.Damage）
                for delay_ms in [500u64, 1700, 2500] {
                    ctx.out_delayed_attacks
                        .push(crate::actors::world::ai::DelayedAttack {
                            delay_ticks: delay_ms / 100,
                            center_x: monster.x,
                            center_y: monster.y,
                            cells: cells.clone(),
                            damage,
                            attacker_oid: monster.object_id,
                            map_index: monster.map_index,
                        });
                }
                // C# FullmoonAttack 每次调用 pushDistance=1 → 3 次各推首个命中目标 1 格
                let hit: Vec<u64> = ctx
                    .find_targets_in_cells(&cells, monster.map_index)
                    .iter()
                    .map(|p| p.session_id)
                    .collect();
                if let Some(&first) = hit.first() {
                    for _ in 0..3 {
                        ctx.out_pushes.push(crate::actors::world::ai::PushPlayer {
                            session_id: first,
                            dir,
                            distance: 1,
                        });
                    }
                }
                // 前方 3 格投放 DarkOmaKingNuke 法术场（C# DarkOmaKing.cs:114-132）
                let dir = direction_towards(monster.x, monster.y, target.x, target.y) as usize;
                let nuke_x = monster.x + DIR_DX[dir % 8] * 3;
                let nuke_y = monster.y + DIR_DY[dir % 8] * 3;
                ctx.out_spell_fields
                    .push(crate::actors::world::ai::SpellFieldSpawn {
                        spell: Spell::DarkOmaKingNuke,
                        x: nuke_x,
                        y: nuke_y,
                        value: monster.max_dmg, // C# Value = Stats[Stat.MaxDC]
                        duration_ms: NUKE_DURATION_MS,
                        tick_ms: NUKE_TICK_MS,
                        caster_oid: monster.object_id,
                        caster_session: 0,
                        cells: Vec::new(),
                        show: true,
                        // C# `start = 3000`（`DelayedAction(DelayedType.Spawn, Envir.Time + 3000, ob)`）
                        start_delay_ms: NUKE_START_MS,
                    });
            }
        } else {
            // 远程（C# DarkOmaKing.cs:134-148）：1/3 概率弹道 MAC 攻击
            monster.next_attack_tick = ctx.tick_count + monster.ai_profile.attack_cooldown + 3;
            if fastrand::i32(0..3) == 0 {
                let damage = crate::combat::attack::get_attack_power(
                    monster.min_mc,
                    monster.max_mc,
                    monster.luck,
                );
                // C# `if (damage == 0) return;`
                if damage > 0 {
                    ctx.out_attacks
                        .push(crate::actors::world::ai::AttackAction::Range {
                            attacker_oid: monster.object_id,
                            target_session: target.session_id,
                            target_object_id: target.object_id,
                            damage,
                            spell_id: 0,
                        });
                }
            }
        }
    }

    fn on_die(&mut self, _monster: &mut MonsterState, _ctx: &mut AiCtx) {
        // C# Die：Kill SlaveList（PowerBead）。由调用方通过 is_slave=true 标记统一清理。
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// #2857：C# `DarkOmaKing.AttackRange = 6`（`:13-19`）——Orb/MassThunder 定时器在 `Attack()` 内，
    /// 只有距离 ≤ 6（`InAttackRange`）才会触发；超出距离只走近。
    #[test]
    fn attack_range_gate_matches_csharp() {
        assert!(in_attack_range(0));
        assert!(in_attack_range(6));
        assert!(!in_attack_range(7));
        assert!(!in_attack_range(20));
    }

    /// #2857：C# Nuke 法术场（`:114-131`）`start = 3000`、`ExpireTime = now + 900 + start`、`TickSpeed = 1000`
    /// ——折算成「总寿命 + 首跳偏移」后应为 `(3900, 2000)`。
    #[test]
    fn nuke_timing_matches_csharp() {
        assert_eq!(NUKE_START_MS, 3000);
        assert_eq!(NUKE_DURATION_MS, 900);
        assert_eq!(NUKE_TICK_MS, 1000);
        let (expires_ms, last_tick_shift_ms) = crate::actors::world::spell::delayed_spell_timing(
            NUKE_START_MS,
            NUKE_DURATION_MS,
            NUKE_TICK_MS,
        );
        assert_eq!((expires_ms, last_tick_shift_ms), (3900, 2000));
    }
}
