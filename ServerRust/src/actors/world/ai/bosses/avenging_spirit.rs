//! AvengingSpirit（复仇之魂）behavior
//!
//! C# 参考：Server/MirObjects/Monsters/AvengingSpirit.cs（继承 AxeSkeleton）
//! 机制：近战（dist<=1）：1/3 SinglePushAttack（伤害+推挤 3，等级门控）/ 2/3 普攻；
//!      远程：RangeDamage（MC，MACAgility）+ 命中 1/7 绿毒（5s，tick 1000）

use crate::actors::world::MonsterState;
use crate::actors::world::ai::behavior::MonsterBehavior;
use crate::actors::world::ai::ctx::AiCtx;
use crate::actors::world::ai::helpers::*;
use crate::combat::poison::Poison;
use mir2_shared::enums::PoisonType;

const ATTACK_RANGE: i32 = 6;

pub struct AvengingSpiritBehavior;

impl AvengingSpiritBehavior {
    pub fn new() -> Self {
        Self
    }
}

impl MonsterBehavior for AvengingSpiritBehavior {
    fn process_tick(&mut self, monster: &mut MonsterState, ctx: &mut AiCtx) {
        let target = match ctx.nearest_target(monster.x, monster.y, ATTACK_RANGE, monster.map_index) {
            Some(t) => *t,
            None => return,
        };
        monster.target_session = Some(target.session_id);
        let dist = max_distance(monster.x, monster.y, target.x, target.y);

        if dist <= ATTACK_RANGE && ctx.tick_count >= monster.next_attack_tick {
            monster.next_attack_tick = ctx.tick_count + monster.ai_profile.attack_cooldown;
            let damage = crate::combat::attack::get_attack_power(monster.min_dmg, monster.max_dmg, monster.luck).max(1);
            let dir = direction_towards(monster.x, monster.y, target.x, target.y);
            if dist <= 1 {
                // C# Random.Next(3) == 0：1/3 SinglePush / 2/3 普攻
                if fastrand::i32(0..3) == 0 {
                    ctx.out_attacks.push(crate::actors::world::ai::AttackAction::Melee {
                        attacker_oid: monster.object_id,
                        target_session: target.session_id,
                        damage,
                        spell_id: 0,
                        attack_type: 1,
                    });
                    // C# SinglePushAttack：目标等级<=怪+5 才推 3 格（MonsterObject.cs:3842）
                    if (target.level as i32) <= monster.level + 5 {
                        ctx.out_pushes.push(crate::actors::world::ai::PushPlayer {
                            session_id: target.session_id,
                            dir,
                            distance: 3,
                        });
                    }
                } else {
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
                // C# CompleteRangeAttack：1/7 绿毒（5s，tick 1000）
                if fastrand::i32(0..7) == 0 {
                    ctx.out_poisons.push(crate::actors::world::ai::PoisonPlayer {
                        session_id: target.session_id,
                        poison: Poison::new(PoisonType::GREEN, 5, poison_sc_value(monster), 1000),
                    });
                }
            }
            return;
        }

        if ctx.tick_count >= monster.next_move_tick {
            let (nx, ny, dir) = step_toward(monster.x, monster.y, target.x, target.y);
            ctx.out_moves.push((monster.object_id, nx, ny, dir));
            monster.next_move_tick = ctx.tick_count + monster.ai_profile.move_interval;
            monster.ai_state = crate::actors::world::MonsterAiState::Chase;
        }
    }
}
