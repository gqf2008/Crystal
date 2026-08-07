//! SeedingsGeneral（幼苗将军）behavior
//!
//! C# 参考：Server/MirObjects/Monsters/SeedingsGeneral.cs
//! 机制：
//!   - AttackRange=2
//!   - 近战 4/5：3/4 DC 血攻（Type0）；1/4 MC 绿色溅射（Type1，MACAgility）
//!   - 远程 4/5：MC 回声喊（Type0，单体 MACAgility + Slow）
//!     1/5：MC 践踏（Type1，AOE 2 格 + Frozen）
//!
//! Attack（C# :27-93）：近战/远程分支 + 4 种攻击类型。
//! CompleteRangeAttack（C# :96-122）：echo→Slow；stomp→AOE+Frozen。

use crate::actors::world::MonsterState;
use crate::combat::poison::Poison;
use mir2_shared::enums::PoisonType;
use crate::actors::world::ai::behavior::MonsterBehavior;
use crate::actors::world::ai::ctx::AiCtx;
use crate::actors::world::ai::helpers::*;

const VIEW_RANGE: i32 = 15;
const ATTACK_RANGE: i32 = 2;
const MELEE_RANGE: i32 = 1;
const STOMP_RADIUS: i32 = 2;

pub struct SeedingsGeneralBehavior;

impl SeedingsGeneralBehavior {
    pub fn new() -> Self {
        Self
    }
}

impl MonsterBehavior for SeedingsGeneralBehavior {
    fn process_tick(&mut self, monster: &mut MonsterState, ctx: &mut AiCtx) {
        let target = match ctx.nearest_target(monster.x, monster.y, VIEW_RANGE, monster.map_index) {
            Some(t) => *t,
            None => return,
        };
        monster.target_session = Some(target.session_id);
        let dist = max_distance(monster.x, monster.y, target.x, target.y);

        if dist <= ATTACK_RANGE && ctx.tick_count >= monster.next_attack_tick {
            monster.next_attack_tick = ctx.tick_count + monster.ai_profile.attack_cooldown;
            let melee = dist <= MELEE_RANGE;

            if melee && fastrand::i32(0..5) > 0 {
                // 近战 4/5：3/4 DC 血攻 / 1/4 MC 绿溅
                if fastrand::i32(0..4) > 0 {
                    // Type0 DC 血攻
                    let damage = crate::combat::attack::get_attack_power(monster.min_dmg, monster.max_dmg, 0).max(1);
                    ctx.out_attacks.push(crate::actors::world::ai::AttackAction::Melee {
                        attacker_oid: monster.object_id,
                        target_session: target.session_id,
                        damage,
                        spell_id: 0,
                        attack_type: 0,
                    });
                } else {
                    // Type1 MC 绿色溅射（MACAgility）
                    let damage = crate::combat::attack::get_attack_power(monster.min_mac, monster.max_mac, 0).max(1);
                    ctx.out_attacks.push(crate::actors::world::ai::AttackAction::Melee {
                        attacker_oid: monster.object_id,
                        target_session: target.session_id,
                        damage,
                        spell_id: 0,
                        attack_type: 1,
                    });
                }
            } else {
                // 远程 4/5：MC 回声喊 / 1/5：MC 践踏
                if fastrand::i32(0..5) > 0 {
                    // Type0 echo 单体 + Slow
                    let damage = crate::combat::attack::get_attack_power(monster.min_mac, monster.max_mac, 0).max(1);
                    ctx.out_attacks.push(crate::actors::world::ai::AttackAction::Range {
                        attacker_oid: monster.object_id,
                        target_session: target.session_id,
                        target_object_id: target.object_id,
                        damage,
                        spell_id: 0,
                    });
                    // C# PoisonTarget 1/5
                        if fastrand::i32(0..5) == 0 {
                        ctx.out_poisons.push(crate::actors::world::ai::PoisonPlayer {
                            session_id: target.session_id,
                            poison: Poison::new(PoisonType::SLOW, 5, damage, 1000),
                        });
                        }
                } else {
                    // Type1 stomp AOE 2 格 + Frozen
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
                        // C# PoisonTarget 1/5
                            if fastrand::i32(0..5) == 0 {
                            ctx.out_poisons.push(crate::actors::world::ai::PoisonPlayer {
                                session_id: h.session_id,
                                poison: Poison::new(PoisonType::FROZEN, 5, damage, 1000),
                            });
                            }
                    }
                }
            }
            return;
        }

        // 追击
        if dist > ATTACK_RANGE && ctx.tick_count >= monster.next_move_tick {
            let (nx, ny, dir) = step_toward(monster.x, monster.y, target.x, target.y);
            ctx.out_moves.push((monster.object_id, nx, ny, dir));
            monster.next_move_tick = ctx.tick_count + monster.ai_profile.move_interval;
            monster.ai_state = crate::actors::world::MonsterAiState::Chase;
        }
    }
}
