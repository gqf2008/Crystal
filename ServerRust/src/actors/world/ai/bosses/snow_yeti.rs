//! SnowYeti（雪猿）behavior
//!
//! C# 参考：Server/MirObjects/Monsters/SnowYeti.cs
//! 机制：
//!   - 近战（dist<=1）：双段近战（500ms + 1500ms 各一次伤害）
//!   - 不在近战范围：1/5 概率远程（MAC）；4/5 概率移动接近
//!   - 远程命中后 1/3 冰冻毒（5s，tick 1000）
//!   - 1/5 概率直接远程（替代近战）

use crate::actors::world::MonsterState;
use crate::actors::world::ai::behavior::MonsterBehavior;
use crate::actors::world::ai::ctx::AiCtx;
use crate::actors::world::ai::helpers::*;
use crate::combat::poison::Poison;
use mir2_shared::enums::PoisonType;

const VIEW_RANGE: i32 = 12;
const ATTACK_RANGE: i32 = 9;

pub struct SnowYetiBehavior;

impl SnowYetiBehavior {
    pub fn new() -> Self {
        Self
    }
}

impl MonsterBehavior for SnowYetiBehavior {
    fn process_tick(&mut self, monster: &mut MonsterState, ctx: &mut AiCtx) {
        let target = match ctx.nearest_target(monster.x, monster.y, ATTACK_RANGE, monster.map_index) {
            Some(t) => *t,
            None => return,
        };
        monster.target_session = Some(target.session_id);
        let dist = max_distance(monster.x, monster.y, target.x, target.y);

        // C# ProcessTarget：不在近战范围 → 1/5 远程，否则移动
        if dist > 1 {
            if ctx.tick_count >= monster.next_attack_tick && fastrand::i32(0..5) == 0 {
                monster.next_attack_tick = ctx.tick_count + monster.ai_profile.attack_cooldown + 10;
                let damage = crate::combat::attack::get_attack_power(monster.min_dmg, monster.max_dmg, monster.luck).max(1);
                ctx.out_attacks.push(crate::actors::world::ai::AttackAction::Range {
                    attacker_oid: monster.object_id,
                    target_session: target.session_id,
                    target_object_id: target.object_id,
                    damage,
                    spell_id: 0,
                });
                // C# CompleteRangeAttack：1/3 冰冻毒（5s，tick 1000）
                if fastrand::i32(0..3) == 0 {
                    ctx.out_poisons.push(crate::actors::world::ai::PoisonPlayer {
                        session_id: target.session_id,
                        poison: Poison::new(PoisonType::FROZEN, 5, 0, 1000),
                    });
                }
                return;
            }
            if ctx.tick_count >= monster.next_move_tick {
                let (nx, ny, dir) = step_toward(monster.x, monster.y, target.x, target.y);
                ctx.out_moves.push((monster.object_id, nx, ny, dir));
                monster.next_move_tick = ctx.tick_count + monster.ai_profile.move_interval;
                monster.ai_state = crate::actors::world::MonsterAiState::Chase;
            }
            return;
        }

        // 近战范围：4/5 双段近战 / 1/5 远程
        if ctx.tick_count >= monster.next_attack_tick {
            monster.next_attack_tick = ctx.tick_count + monster.ai_profile.attack_cooldown;
            let damage = crate::combat::attack::get_attack_power(monster.min_dmg, monster.max_dmg, monster.luck).max(1);
            if fastrand::i32(0..5) > 0 {
                // C# Attack：500ms + 1500ms 两次伤害
                for _ in 0..2 {
                    ctx.out_attacks.push(crate::actors::world::ai::AttackAction::Melee {
                        attacker_oid: monster.object_id,
                        target_session: target.session_id,
                        damage,
                        spell_id: 0,
                        attack_type: 0,
                    });
                }
            } else {
                ctx.out_attacks.push(crate::actors::world::ai::AttackAction::Range {
                    attacker_oid: monster.object_id,
                    target_session: target.session_id,
                    target_object_id: target.object_id,
                    damage,
                    spell_id: 0,
                });
                if fastrand::i32(0..3) == 0 {
                    ctx.out_poisons.push(crate::actors::world::ai::PoisonPlayer {
                        session_id: target.session_id,
                        poison: Poison::new(PoisonType::FROZEN, 5, 0, 1000),
                    });
                }
            }
        }
    }
}
