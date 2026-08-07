//! Jar2（坛子2）behavior
//!
//! C# 参考：Server/MirObjects/Monsters/Jar2.cs（继承 Jar1）
//! 机制：静态（CanMove=false，AttackRange=6）；
//!      近战（dist<=1）且 1/3：近战 DC（MACAgility）；
//!      否则：远程 MC（MAC，攻速+500ms）+ 命中 1/5 冰冻（5s，tick 1000）

use crate::actors::world::MonsterState;
use crate::actors::world::ai::behavior::MonsterBehavior;
use crate::actors::world::ai::ctx::AiCtx;
use crate::actors::world::ai::helpers::*;
use crate::combat::poison::Poison;
use mir2_shared::enums::PoisonType;

const ATTACK_RANGE: i32 = 6;

pub struct Jar2Behavior;

impl Jar2Behavior {
    pub fn new() -> Self {
        Self
    }
}

impl MonsterBehavior for Jar2Behavior {
    fn can_move(&self) -> bool {
        false
    }

    fn can_regen(&self) -> bool {
        false
    }

    fn process_tick(&mut self, monster: &mut MonsterState, ctx: &mut AiCtx) {
        let target = match ctx.nearest_target(monster.x, monster.y, ATTACK_RANGE, monster.map_index) {
            Some(t) => *t,
            None => return,
        };
        monster.target_session = Some(target.session_id);
        let dist = max_distance(monster.x, monster.y, target.x, target.y);

        if dist <= ATTACK_RANGE && ctx.tick_count >= monster.next_attack_tick {
            let damage = crate::combat::attack::get_attack_power(monster.min_dmg, monster.max_dmg, monster.luck).max(1);
            // C# !ranged && Random.Next(3) == 0：近战 1/3
            if dist <= 1 && fastrand::i32(0..3) == 0 {
                monster.next_attack_tick = ctx.tick_count + monster.ai_profile.attack_cooldown;
                ctx.out_attacks.push(crate::actors::world::ai::AttackAction::Melee {
                    attacker_oid: monster.object_id,
                    target_session: target.session_id,
                    damage,
                    spell_id: 0,
                    attack_type: 0,
                });
            } else {
                monster.next_attack_tick = ctx.tick_count + monster.ai_profile.attack_cooldown + 5;
                ctx.out_attacks.push(crate::actors::world::ai::AttackAction::Range {
                    attacker_oid: monster.object_id,
                    target_session: target.session_id,
                    target_object_id: target.object_id,
                    damage,
                    spell_id: 0,
                });
                // C# CompleteRangeAttack：1/5 冰冻（5s，tick 1000）
                if fastrand::i32(0..5) == 0 {
                    ctx.out_poisons.push(crate::actors::world::ai::PoisonPlayer {
                        session_id: target.session_id,
                        poison: Poison::new(PoisonType::FROZEN, 5, 0, 1000),
                    });
                }
            }
        }
    }
}
