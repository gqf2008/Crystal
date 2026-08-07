//! TucsonWarrior（图森战士）behavior
//!
//! C# 参考：Server/MirObjects/Monsters/TucsonWarrior.cs
//! 机制：InAttackRange 2 格十字/对角；dist<=1 且 4/5 Halfmoon（AOE1 近似，DC）/
//!      否则 SmashAttack(1)：目标中心半径 1 AOE（MC）

use crate::actors::world::MonsterState;
use crate::actors::world::ai::behavior::MonsterBehavior;
use crate::actors::world::ai::ctx::AiCtx;
use crate::actors::world::ai::helpers::*;

const VIEW_RANGE: i32 = 12;
const AOE_RADIUS: i32 = 1;

pub struct TucsonWarriorBehavior;

impl TucsonWarriorBehavior {
    pub fn new() -> Self {
        Self
    }
}

impl MonsterBehavior for TucsonWarriorBehavior {
    fn process_tick(&mut self, monster: &mut MonsterState, ctx: &mut AiCtx) {
        let target = match ctx.nearest_target(monster.x, monster.y, VIEW_RANGE, monster.map_index) {
            Some(t) => *t,
            None => return,
        };
        monster.target_session = Some(target.session_id);
        let dx = (target.x - monster.x).abs();
        let dy = (target.y - monster.y).abs();
        let dist = max_distance(monster.x, monster.y, target.x, target.y);
        let in_range = dx <= 2 && dy <= 2 && ((dx <= 1 && dy <= 1) || (dx == dy || dx % 2 == dy % 2));

        if in_range && ctx.tick_count >= monster.next_attack_tick {
            monster.next_attack_tick = ctx.tick_count + monster.ai_profile.attack_cooldown;
            let damage = crate::combat::attack::get_attack_power(monster.min_dmg, monster.max_dmg, monster.luck).max(1);
            // C# !range && Random.Next(5) > 0：近战 4/5 Halfmoon / 1/5 Smash
            if dist <= 1 && fastrand::i32(0..5) > 0 {
                ctx.out_attacks.push(crate::actors::world::ai::AttackAction::Aoe {
                    attacker_oid: monster.object_id,
                    center_x: monster.x,
                    center_y: monster.y,
                    radius: AOE_RADIUS,
                    damage,
                    spell_id: 0,
                });
            } else {
                // C# SmashAttack(1)：目标中心半径 1
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
