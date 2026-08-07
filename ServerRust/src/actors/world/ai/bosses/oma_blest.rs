//! OmaBlest（奥玛祝福者）behavior
//!
//! C# 参考：Server/MirObjects/Monsters/OmaBlest.cs
//! 机制：1/2 物理近战（DC）/ 1/2 AOE：FindAllTargets(1, 目标位置) 半径 1（以目标为中心）

use crate::actors::world::MonsterState;
use crate::actors::world::ai::behavior::MonsterBehavior;
use crate::actors::world::ai::ctx::AiCtx;
use crate::actors::world::ai::helpers::*;

const VIEW_RANGE: i32 = 12;
const AOE_RADIUS: i32 = 1;

pub struct OmaBlestBehavior;

impl OmaBlestBehavior {
    pub fn new() -> Self {
        Self
    }
}

impl MonsterBehavior for OmaBlestBehavior {
    fn process_tick(&mut self, monster: &mut MonsterState, ctx: &mut AiCtx) {
        let target = match ctx.nearest_target(monster.x, monster.y, VIEW_RANGE, monster.map_index) {
            Some(t) => *t,
            None => return,
        };
        monster.target_session = Some(target.session_id);
        let dist = max_distance(monster.x, monster.y, target.x, target.y);

        if dist <= 1 && ctx.tick_count >= monster.next_attack_tick {
            monster.next_attack_tick = ctx.tick_count + monster.ai_profile.attack_cooldown;
            let damage = crate::combat::attack::get_attack_power(monster.min_dmg, monster.max_dmg, monster.luck).max(1);
            // C# Envir.Random.Next(2) > 0：1/2 近战 / 1/2 AOE
            if fastrand::i32(0..2) > 0 {
                ctx.out_attacks.push(crate::actors::world::ai::AttackAction::Melee {
                    attacker_oid: monster.object_id,
                    target_session: target.session_id,
                    damage,
                    spell_id: 0,
                    attack_type: 0,
                });
            } else {
                // C# AOE：FindAllTargets(1, target.CurrentLocation)——以目标为中心
                ctx.out_attacks.push(crate::actors::world::ai::AttackAction::Aoe {
                    attacker_oid: monster.object_id,
                    center_x: target.x,
                    center_y: target.y,
                    radius: AOE_RADIUS,
                    damage,
                    spell_id: 0,
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
    }
}
