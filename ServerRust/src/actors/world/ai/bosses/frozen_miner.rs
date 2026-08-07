//! FrozenMiner（冰霜矿工）behavior
//!
//! C# 参考：Server/MirObjects/Monsters/FrozenMiner.cs
//! 机制：近战；目标>1 且 1/2，或 1/8 → Type=1 AOE 1（0.8x 伤害）；否则单体近战

use crate::actors::world::MonsterState;
use crate::actors::world::ai::behavior::MonsterBehavior;
use crate::actors::world::ai::ctx::AiCtx;
use crate::actors::world::ai::helpers::*;

const VIEW_RANGE: i32 = 12;
const AOE_RADIUS: i32 = 1;

pub struct FrozenMinerBehavior;

impl FrozenMinerBehavior {
    pub fn new() -> Self {
        Self
    }
}

impl MonsterBehavior for FrozenMinerBehavior {
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
            let nearby = ctx.find_targets_in_range(monster.x, monster.y, AOE_RADIUS, monster.map_index);
            // C# (targets.Count>1 && Random.Next(2)==0) || Random.Next(8)==0 → AOE
            let aoe = (nearby.len() > 1 && fastrand::i32(0..2) == 0) || fastrand::i32(0..8) == 0;
            if aoe {
                ctx.out_attacks.push(crate::actors::world::ai::AttackAction::Aoe {
                    attacker_oid: monster.object_id,
                    center_x: monster.x,
                    center_y: monster.y,
                    radius: AOE_RADIUS,
                    damage: ((damage as f32 * 0.8) as i32).max(1),
                    spell_id: 0,
                });
            } else {
                ctx.out_attacks.push(crate::actors::world::ai::AttackAction::Melee {
                    attacker_oid: monster.object_id,
                    target_session: target.session_id,
                    damage,
                    spell_id: 0,
                    attack_type: 0,
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
