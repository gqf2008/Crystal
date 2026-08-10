//! AssassinBird（刺客鸟）behavior
//!
//! C# 参考：Server/MirObjects/Monsters/AssassinBird.cs
//! 机制：近战（dist<=1）：8/9：3/4 普攻 DC / 1/4 魔法 MC；1/9：Type=2 SinglePushAttack（推挤 3，等级门控）

use crate::actors::world::MonsterState;
use crate::actors::world::ai::behavior::MonsterBehavior;
use crate::actors::world::ai::ctx::AiCtx;
use crate::actors::world::ai::helpers::*;

const VIEW_RANGE: i32 = 12;

pub struct AssassinBirdBehavior;

impl AssassinBirdBehavior {
    pub fn new() -> Self {
        Self
    }
}

impl MonsterBehavior for AssassinBirdBehavior {
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
            let dir = direction_towards(monster.x, monster.y, target.x, target.y);
            // C# Random.Next(9) > 0：8/9 普通 / 1/9 推挤
            if fastrand::i32(0..9) > 0 {
                // C# Random.Next(4) > 0：3/4 普攻 / 1/4 魔法
                let magic = fastrand::i32(0..4) == 0;
                ctx.out_attacks.push(crate::actors::world::ai::AttackAction::Melee {
                    attacker_oid: monster.object_id,
                    target_session: target.session_id,
                    damage,
                    spell_id: 0,
                    attack_type: if magic { 1 } else { 0 },
                });
            } else {
                ctx.out_attacks.push(crate::actors::world::ai::AttackAction::Melee {
                    attacker_oid: monster.object_id,
                    target_session: target.session_id,
                    damage,
                    spell_id: 0,
                    attack_type: 2,
                });
                // C# SinglePushAttack：目标等级<=怪+5 才推 3 格（MonsterObject.cs:3842）
                if (target.level as i32) <= monster.level + 5 {
                    ctx.out_pushes.push(crate::actors::world::ai::PushPlayer {
                        session_id: target.session_id,
                        dir,
                        distance: 3,
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
