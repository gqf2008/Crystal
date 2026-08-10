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

use mir2_shared::enums::Spell;
use crate::actors::world::MonsterState;
use crate::combat::poison::Poison;
use mir2_shared::enums::PoisonType;
use crate::actors::world::ai::behavior::MonsterBehavior;
use crate::actors::world::ai::ctx::AiCtx;
use crate::actors::world::ai::helpers::*;

const VIEW_RANGE: i32 = 15;
const STOMP_RADIUS: i32 = 3;
/// 狂暴冷却（C# _RageTime = Time + 20000）
const RAGE_COOLDOWN_TICKS: u64 = 200;
/// 落石数量（C# _RockCount = 15）
const ROCK_COUNT: usize = 15;

pub struct TucsonGeneralBehavior {
    /// 下次狂暴 tick（C# _RageTime）
    next_rage_tick: u64,
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
            let targets: Vec<crate::actors::world::ai::PlayerSnap> =
                ctx.find_targets_in_range(monster.x, monster.y, VIEW_RANGE, monster.map_index)
                    .into_iter().copied().collect();
            for _ in 0..ROCK_COUNT {
                let (rx, ry) = if fastrand::i32(0..3) == 0 && !targets.is_empty() {
                    // 1/3 概率落在随机玩家身上
                    let t = targets[fastrand::usize(0..targets.len())];
                    (t.x, t.y)
                } else {
                    // 视野范围内随机点（C# CurrentLocation ± ViewRange）
                    (
                        monster.x + fastrand::i32(-VIEW_RANGE..=VIEW_RANGE),
                        monster.y + fastrand::i32(-VIEW_RANGE..=VIEW_RANGE),
                    )
                };
                let value = crate::combat::attack::get_attack_power(monster.min_dmg, monster.max_dmg, 0).max(1);
                ctx.out_spell_fields.push(crate::actors::world::ai::SpellFieldSpawn {
                    spell: Spell::MapQuake1,
                    x: rx,
                    y: ry,
                    value,
                    duration_ms: 2000,
                    tick_ms: 1000,
                    caster_oid: monster.object_id,
                    caster_session: 0,
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
                    let damage = crate::combat::attack::get_attack_power(monster.min_dmg, monster.max_dmg, 0).max(1);
                    ctx.out_attacks.push(crate::actors::world::ai::AttackAction::Melee {
                        attacker_oid: monster.object_id,
                        target_session: target.session_id,
                        damage,
                        spell_id: 0,
                        attack_type: 0,
                    });
                } else {
                    // Type1 MC 践踏 AOE 3 + Paralysis
                    let damage = crate::combat::attack::get_attack_power(monster.min_mac, monster.max_mac, 0).max(1);
                    let hits: Vec<crate::actors::world::ai::PlayerSnap> =
                        ctx.find_targets_in_range(monster.x, monster.y, STOMP_RADIUS, monster.map_index)
                            .into_iter().copied().collect();
                    for h in hits {
                        ctx.out_attacks.push(crate::actors::world::ai::AttackAction::Melee {
                            attacker_oid: monster.object_id,
                            target_session: h.session_id,
                            damage,
                            spell_id: 0,
                            attack_type: 1,
                        });
                        // C# PoisonTarget 1/3
                            if fastrand::i32(0..3) == 0 {
                            ctx.out_poisons.push(crate::actors::world::ai::PoisonPlayer {
                                session_id: h.session_id,
                                poison: Poison::new(PoisonType::PARALYSIS, 5, poison_sc_value(monster), 1000),
                            });
                            }
                    }
                }
            } else if fastrand::i32(0..4) > 0 {
                // Type1 SC 弹道
                let damage = crate::combat::attack::get_attack_power(monster.min_dmg, monster.max_dmg, 0).max(1);
                ctx.out_attacks.push(crate::actors::world::ai::AttackAction::Range {
                    attacker_oid: monster.object_id,
                    target_session: target.session_id,
                    target_object_id: target.object_id,
                    damage,
                    spell_id: 0,
                });
            } else {
                // Type2 SC*2 强力弹道
                let damage = crate::combat::attack::get_attack_power(monster.min_dmg, monster.max_dmg * 2, 0).max(1);
                ctx.out_attacks.push(crate::actors::world::ai::AttackAction::Range {
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
