//! CaveMaggot（洞穴蛆，AI 7）behavior
//!
//! C# 参考：Server/MirObjects/Monsters/CaveMaggot.cs
//! 机制：近战（DefenceType.MACAgility），命中后 20% 施加麻痹（5s）

use crate::actors::world::MonsterState;
use crate::actors::world::ai::behavior::MonsterBehavior;
use crate::actors::world::ai::ctx::AiCtx;
use crate::actors::world::ai::helpers::*;

const VIEW_RANGE: i32 = 8;

pub struct CaveMaggotBehavior;

impl CaveMaggotBehavior {
    pub fn new() -> Self {
        Self
    }
}

impl MonsterBehavior for CaveMaggotBehavior {
    fn process_tick(&mut self, monster: &mut MonsterState, ctx: &mut AiCtx) {
        let target = match ctx.nearest_target(monster.x, monster.y, VIEW_RANGE, monster.map_index) {
            Some(t) => *t,
            None => return,
        };
        monster.target_session = Some(target.session_id);
        let dist = max_distance(monster.x, monster.y, target.x, target.y);

        if dist <= 1 && ctx.tick_count >= monster.next_attack_tick {
            monster.next_attack_tick = ctx.tick_count + monster.ai_profile.attack_cooldown;
            let damage = crate::combat::attack::get_attack_power(monster.min_dmg, monster.max_dmg, 0).max(1);
            ctx.out_attacks.push(crate::actors::world::ai::AttackAction::Melee {
                attacker_oid: monster.object_id,
                target_session: target.session_id,
                damage,
                spell_id: 0,
                attack_type: 0,
            });
            // C# CompleteAttack：PoisonTarget(20, 5, Paralysis, 1000)：1/20（5%）
            if fastrand::i32(0..20) == 0 {
                ctx.out_poisons.push(crate::actors::world::ai::PoisonPlayer {
                    session_id: target.session_id,
                    poison: crate::combat::poison::Poison::new(
                        mir2_shared::enums::PoisonType::PARALYSIS, 5, 0, 1000),
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
