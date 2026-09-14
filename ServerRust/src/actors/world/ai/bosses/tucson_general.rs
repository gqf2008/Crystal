//! TucsonGeneral（图森将军）behavior
//!
//! C# 参考：Server/MirObjects/Monsters/TucsonGeneral.cs
//! 机制：
//!   - 周期性狂暴（_RageTime，20s 冷却）：在视野范围内投放 15 颗落石法术场
//!     （TucsonGeneralRock），1/3 概率直接落在玩家身上
//!   - 近战 3/4：2/3 DC 单体；1/3 MC 践踏（AOE 3 格 + Paralysis）
//!   - 远程 3/4：3/4 SC 弹道；1/4 SC*2 强力弹道
//!
//! Attack（C# :25-126）：Rage→落石；近战/远程分支。
//! CompleteAttack（C# :128-152）：stomp→AOE3 + Paralysis。

use crate::actors::world::ai::behavior::MonsterBehavior;
use crate::actors::world::ai::ctx::AiCtx;
use crate::actors::world::ai::helpers::*;
use crate::actors::world::MonsterState;
use crate::combat::poison::Poison;
use mir2_shared::enums::PoisonType;
use mir2_shared::enums::Spell;

const VIEW_RANGE: i32 = 15;
const STOMP_RADIUS: i32 = 3;
/// 狂暴冷却（C# _RageTime = Time + 20000）
const RAGE_COOLDOWN_TICKS: u64 = 200;
/// 落石数量（C# _RockCount = 15）
const ROCK_COUNT: usize = 15;
/// C# `TucsonGeneral.cs:69`：`ExpireTime = now + 2000 + start`；对象在 `start` 生成、
/// 首跳在 `start + 1000`（`StartTime`）⇒ 相对首跳再活 1000ms
const ROCK_DURATION_MS: u64 = 1000;

/// C# `TucsonGeneral.cs:60`：落点跳过「与自身同行**或**同列」的点（`||` 语义，注意不是 `&&`）。
pub(crate) fn rock_location_allowed(rx: i32, ry: i32, boss_x: i32, boss_y: i32) -> bool {
    !(rx == boss_x || ry == boss_y)
}

/// C# `TucsonGeneral.cs:62/69/75`：`start = Random(0,5000)`（对象生成时机）+
/// `StartTime = now + 1000 + start`（首跳再晚 1 秒）⇒ 本端首跳延迟 = `start + 1000`。
pub(crate) fn rock_start_delay_ms(rng: &mut fastrand::Rng) -> u64 {
    rng.i64(0..5000) as u64 + 1000
}

pub struct TucsonGeneralBehavior {
    /// 下次狂暴 tick（C# _RageTime）
    next_rage_tick: u64,
}

impl Default for TucsonGeneralBehavior {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// #2859：C# `TucsonGeneral.cs:60`——落点跳过「与自身同行**或**同列」的点（`||`，不是 `&&`）
    #[test]
    fn rock_location_skips_same_row_or_column() {
        let boss = (100, 100);
        assert!(
            !rock_location_allowed(100, 137, boss.0, boss.1),
            "同行应跳过"
        );
        assert!(
            !rock_location_allowed(137, 100, boss.0, boss.1),
            "同列应跳过"
        );
        assert!(rock_location_allowed(101, 101, boss.0, boss.1));
        assert!(rock_location_allowed(137, 137, boss.0, boss.1));
    }

    /// #2859：C# `TucsonGeneral.cs:62/69/75`——首跳延迟 = `Random(0,5000) + 1000`；
    /// `ExpireTime = now + 2000 + start` ⇒ 本端 `(start_delay, duration)=(start+1000, 1000)`
    /// 折算出的总寿命必须等于 `start + 2000`
    #[test]
    fn rock_timing_matches_csharp() {
        let mut rng = fastrand::Rng::with_seed(9);
        for _ in 0..200 {
            let delay = rock_start_delay_ms(&mut rng);
            assert!((1000..6000).contains(&delay), "首跳延迟 ∈ [1000, 6000)");
            let (expires_ms, _) =
                crate::actors::world::spell::delayed_spell_timing(delay, ROCK_DURATION_MS, 1000);
            assert_eq!(expires_ms - delay, 1000, "相对首跳再活 1000ms");
            let start = delay - 1000;
            assert_eq!(
                expires_ms,
                start + 2000,
                "总寿命 = start + 2000（C# ExpireTime）"
            );
        }
        assert_eq!(ROCK_DURATION_MS, 1000);
        // C# `Value = Random(MinDC, MaxDC)`：法术 = TucsonGeneralRock（不是 MapQuake1）
        assert_eq!(ROCK_COUNT, 15);
    }
}

impl TucsonGeneralBehavior {
    pub fn new() -> Self {
        Self { next_rage_tick: 0 }
    }
}

impl MonsterBehavior for TucsonGeneralBehavior {
    fn process_tick(&mut self, monster: &mut MonsterState, ctx: &mut AiCtx) {
        let target = match ctx.nearest_target(monster.x, monster.y, VIEW_RANGE, monster.map_index) {
            Some(t) => *t,
            None => return,
        };
        monster.target_session = Some(target.session_id);
        let dist = max_distance(monster.x, monster.y, target.x, target.y);

        // ---- 狂暴：落石雨（周期触发）----
        if ctx.tick_count >= self.next_rage_tick {
            self.next_rage_tick = ctx.tick_count + RAGE_COOLDOWN_TICKS;
            // 收集目标用于 1/3 落点
            let targets: Vec<crate::actors::world::ai::PlayerSnap> = ctx
                .find_targets_in_range(monster.x, monster.y, VIEW_RANGE, monster.map_index)
                .into_iter()
                .copied()
                .collect();
            // 落石循环统一用同一个 rng（C# 全程 `Envir.Random`）
            let mut rng = fastrand::Rng::new();
            for _ in 0..ROCK_COUNT {
                let (rx, ry) = if rng.i32(0..3) == 0 && !targets.is_empty() {
                    // 1/3 概率落在随机玩家身上
                    let t = targets[rng.usize(0..targets.len())];
                    (t.x, t.y)
                } else {
                    // 视野范围内随机点（C# CurrentLocation ± ViewRange）
                    (
                        monster.x + rng.i32(-VIEW_RANGE..=VIEW_RANGE),
                        monster.y + rng.i32(-VIEW_RANGE..=VIEW_RANGE),
                    )
                };
                // C# `TucsonGeneral.cs:60`：**与自身同行或同列**的点跳过（注意是 `||` 不是 `&&`）
                if !rock_location_allowed(rx, ry, monster.x, monster.y) {
                    continue;
                }
                // C# `Value = Random(MinDC, MaxDC)`（可 roll 到 0 ⇒ 该落石不造成伤害，由法术场
                // 的 `Value == 0` 短路处理）
                let value =
                    crate::combat::attack::get_attack_power(monster.min_dmg, monster.max_dmg, 0);
                let start_delay_ms = rock_start_delay_ms(&mut rng);
                ctx.out_spell_fields
                    .push(crate::actors::world::ai::SpellFieldSpawn {
                        // #2859：C# 用的是 `Spell.TucsonGeneralRock`（此前误写成 MapQuake1，
                        // 导致防御类型（AC vs MAC）与客户端视觉都不对）
                        spell: Spell::TucsonGeneralRock,
                        x: rx,
                        y: ry,
                        value,
                        duration_ms: ROCK_DURATION_MS,
                        tick_ms: 1000,
                        caster_oid: monster.object_id,
                        caster_session: 0,
                        cells: Vec::new(),
                        // C# 未设置 `Show`（默认 false）⇒ 不广播 `S.ObjectSpell`
                        show: false,
                        start_delay_ms,
                    });
            }
            monster.next_attack_tick = ctx.tick_count + 80;
            return;
        }

        // ---- 攻击分支 ----
        if dist <= VIEW_RANGE && ctx.tick_count >= monster.next_attack_tick {
            monster.next_attack_tick = ctx.tick_count + monster.ai_profile.attack_cooldown;
            let melee = dist <= 2;

            if melee && fastrand::i32(0..4) > 0 {
                if fastrand::i32(0..3) > 0 {
                    // Type0 DC 单体
                    let damage = crate::combat::attack::get_attack_power(
                        monster.min_dmg,
                        monster.max_dmg,
                        0,
                    )
                    .max(1);
                    ctx.out_attacks
                        .push(crate::actors::world::ai::AttackAction::Melee {
                            attacker_oid: monster.object_id,
                            target_session: target.session_id,
                            damage,
                            spell_id: 0,
                            attack_type: 0,
                        });
                } else {
                    // Type1 MC 践踏 AOE 3 + Paralysis
                    let damage =
                        crate::combat::attack::get_attack_power(monster.min_mc, monster.max_mc, 0)
                            .max(1);
                    let hits: Vec<crate::actors::world::ai::PlayerSnap> = ctx
                        .find_targets_in_range(
                            monster.x,
                            monster.y,
                            STOMP_RADIUS,
                            monster.map_index,
                        )
                        .into_iter()
                        .copied()
                        .collect();
                    for h in hits {
                        ctx.out_attacks
                            .push(crate::actors::world::ai::AttackAction::Melee {
                                attacker_oid: monster.object_id,
                                target_session: h.session_id,
                                damage,
                                spell_id: 0,
                                attack_type: 1,
                            });
                        // C# PoisonTarget 1/3
                        if fastrand::i32(0..3) == 0 {
                            ctx.out_poisons
                                .push(crate::actors::world::ai::PoisonPlayer {
                                    session_id: h.session_id,
                                    poison: Poison::new(
                                        PoisonType::PARALYSIS,
                                        5,
                                        poison_sc_value(monster),
                                        1000,
                                    ),
                                });
                        }
                    }
                }
            } else if fastrand::i32(0..4) > 0 {
                // Type1 SC 弹道（TucsonGeneral.cs:111）
                let damage =
                    crate::combat::attack::get_attack_power(monster.min_sc, monster.max_sc, 0)
                        .max(1);
                ctx.out_attacks
                    .push(crate::actors::world::ai::AttackAction::Range {
                        attacker_oid: monster.object_id,
                        target_session: target.session_id,
                        target_object_id: target.object_id,
                        damage,
                        spell_id: 0,
                    });
            } else {
                // Type2 SC*2 强力弹道（TucsonGeneral.cs:119）
                let damage =
                    crate::combat::attack::get_attack_power(monster.min_sc, monster.max_sc * 2, 0)
                        .max(1);
                ctx.out_attacks
                    .push(crate::actors::world::ai::AttackAction::Range {
                        attacker_oid: monster.object_id,
                        target_session: target.session_id,
                        target_object_id: target.object_id,
                        damage,
                        spell_id: 0,
                    });
            }
            return;
        }

        // 追击
        if ctx.tick_count >= monster.next_move_tick {
            let (nx, ny, dir) = step_toward(monster.x, monster.y, target.x, target.y);
            ctx.out_moves.push((monster.object_id, nx, ny, dir));
            monster.next_move_tick = ctx.tick_count + monster.ai_profile.move_interval;
            monster.ai_state = crate::actors::world::MonsterAiState::Chase;
        }
    }
}
