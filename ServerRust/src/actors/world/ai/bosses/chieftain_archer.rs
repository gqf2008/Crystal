//! ChieftainArcher（酋长弓手）behavior
//!
//! C# 参考：Server/MirObjects/Monsters/ChieftainArcher.cs
//! 机制：
//!   - AttackRange=6 远程风筝（<6 远离，>=6 接近）
//!   - 攻击：等级 0/1/2 三态（DC/MC/SC，此处 SC 用 DC 近似）；等级 2 命中后推挤 1 格

use crate::actors::world::MonsterState;
use crate::actors::world::ai::behavior::MonsterBehavior;
use crate::actors::world::ai::ctx::AiCtx;
use crate::actors::world::ai::helpers::*;

const ATTACK_RANGE: i32 = 6;

pub struct ChieftainArcherBehavior;

impl ChieftainArcherBehavior {
    pub fn new() -> Self {
        Self
    }
}

impl MonsterBehavior for ChieftainArcherBehavior {
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
            // C# level = Random(0..3)：0=DC / 1=MC / 2=SC（SC 用 DC 近似）
            let level = fastrand::i32(0..3);
            let dmg = if level == 1 { damage } else { damage };
            ctx.out_attacks.push(crate::actors::world::ai::AttackAction::Range {
                attacker_oid: monster.object_id,
                target_session: target.session_id,
                target_object_id: target.object_id,
                damage: dmg,
                spell_id: 0,
            });
            // C# CompleteRangeAttack：level==2 → 推挤 1 格
            if level == 2 {
                ctx.out_pushes.push(crate::actors::world::ai::PushPlayer {
                    session_id: target.session_id,
                    dir,
                    distance: 1,
                });
            }
            return;
        }

        if ctx.tick_count >= monster.next_move_tick {
            // C# 风筝：>=6 接近，<6 远离
            let (nx, ny, dir) = if dist >= ATTACK_RANGE {
                step_toward(monster.x, monster.y, target.x, target.y)
            } else {
                step_away(monster.x, monster.y, target.x, target.y)
            };
            ctx.out_moves.push((monster.object_id, nx, ny, dir));
            monster.next_move_tick = ctx.tick_count + monster.ai_profile.move_interval;
            monster.ai_state = crate::actors::world::MonsterAiState::Chase;
        }
    }
}
