//! Turtlegrass（龟草）behavior
//!
//! C# 参考：Server/MirObjects/Monsters/Turtlegrass.cs（继承 ZumaMonster）
//! 机制：InAttackRange 2 格十字/对角；3/4 base.Attack（DC）/ 1/4 SinglePushAttack（伤害+推挤）

use crate::actors::world::MonsterState;
use crate::actors::world::ai::behavior::MonsterBehavior;
use crate::actors::world::ai::ctx::AiCtx;
use crate::actors::world::ai::helpers::*;

const VIEW_RANGE: i32 = 12;

pub struct TurtlegrassBehavior;

impl TurtlegrassBehavior {
    pub fn new() -> Self {
        Self
    }
}

impl MonsterBehavior for TurtlegrassBehavior {
    fn process_tick(&mut self, monster: &mut MonsterState, ctx: &mut AiCtx) {
        let target = match ctx.nearest_target(monster.x, monster.y, VIEW_RANGE, monster.map_index) {
            Some(t) => *t,
            None => return,
        };
        monster.target_session = Some(target.session_id);
        let dx = (target.x - monster.x).abs();
        let dy = (target.y - monster.y).abs();
        let in_range = dx <= 2 && dy <= 2 && ((dx <= 1 && dy <= 1) || (dx == dy || dx % 2 == dy % 2));

        if in_range && ctx.tick_count >= monster.next_attack_tick {
            monster.next_attack_tick = ctx.tick_count + monster.ai_profile.attack_cooldown;
            let damage = crate::combat::attack::get_attack_power(monster.min_dmg, monster.max_dmg, monster.luck).max(1);
            let dir = direction_towards(monster.x, monster.y, target.x, target.y);
            // C# Random.Next(4) > 0：3/4 base / 1/4 SinglePush
            if fastrand::i32(0..4) > 0 {
                ctx.out_attacks.push(crate::actors::world::ai::AttackAction::Melee {
                    attacker_oid: monster.object_id,
                    target_session: target.session_id,
                    damage,
                    spell_id: 0,
                    attack_type: 0,
                });
            } else {
                ctx.out_attacks.push(crate::actors::world::ai::AttackAction::Melee {
                    attacker_oid: monster.object_id,
                    target_session: target.session_id,
                    damage,
                    spell_id: 0,
                    attack_type: 1,
                });
                ctx.out_pushes.push(crate::actors::world::ai::PushPlayer {
                    session_id: target.session_id,
                    dir,
                    distance: 1,
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
