//! OmaCannibal（奥玛食人花）behavior
//!
//! C# 参考：Server/MirObjects/Monsters/OmaCannibal.cs
//! 机制：近战（dist<=1）DC / 远程 MC（MACAgility，攻速+500ms）；
//!      远程命中后 1/5 绿毒（5s，tick 1000）

use crate::actors::world::MonsterState;
use crate::actors::world::ai::behavior::MonsterBehavior;
use crate::actors::world::ai::ctx::AiCtx;
use crate::actors::world::ai::helpers::*;
use crate::combat::poison::Poison;
use mir2_shared::enums::PoisonType;

const VIEW_RANGE: i32 = 12;

pub struct OmaCannibalBehavior;

impl OmaCannibalBehavior {
    pub fn new() -> Self {
        Self
    }
}

impl MonsterBehavior for OmaCannibalBehavior {
    fn process_tick(&mut self, monster: &mut MonsterState, ctx: &mut AiCtx) {
        let target = match ctx.nearest_target(monster.x, monster.y, VIEW_RANGE, monster.map_index) {
            Some(t) => *t,
            None => return,
        };
        monster.target_session = Some(target.session_id);
        let dist = max_distance(monster.x, monster.y, target.x, target.y);

        if dist <= VIEW_RANGE && ctx.tick_count >= monster.next_attack_tick {
            let damage = crate::combat::attack::get_attack_power(monster.min_dmg, monster.max_dmg, monster.luck).max(1);
            if dist <= 1 {
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
                // C# CompleteRangeAttack：PoisonTarget(5, 5, Green, 1000)：1/5、5s
                if fastrand::i32(0..5) == 0 {
                    ctx.out_poisons.push(crate::actors::world::ai::PoisonPlayer {
                        session_id: target.session_id,
                        poison: Poison::new(PoisonType::GREEN, 5, damage, 1000),
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
