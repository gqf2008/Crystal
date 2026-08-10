//! SepArcher（圣战弓手）behavior（简化）
//!
//! C# 参考：Server/MirObjects/Monsters/SepArcher.cs
//! 机制：远程投射；dist<=2 且 1/3：BackStep 后跳 3 格（用远离 1 步近似）

use crate::actors::world::MonsterState;
use crate::actors::world::ai::behavior::MonsterBehavior;
use crate::actors::world::ai::ctx::AiCtx;
use crate::actors::world::ai::helpers::*;

const VIEW_RANGE: i32 = 12;
/// C# AttackRange = 6
const ATTACK_RANGE: i32 = 6;

pub struct SepArcherBehavior;

impl SepArcherBehavior {
    pub fn new() -> Self {
        Self
    }
}

impl MonsterBehavior for SepArcherBehavior {
    fn process_tick(&mut self, monster: &mut MonsterState, ctx: &mut AiCtx) {
        let target = match ctx.nearest_target(monster.x, monster.y, VIEW_RANGE, monster.map_index) {
            Some(t) => *t,
            None => return,
        };
        monster.target_session = Some(target.session_id);
        let dist = max_distance(monster.x, monster.y, target.x, target.y);

        if dist <= ATTACK_RANGE && ctx.tick_count >= monster.next_attack_tick {
            monster.next_attack_tick = ctx.tick_count + monster.ai_profile.attack_cooldown;
            let damage = crate::combat::attack::get_attack_power(monster.min_dmg, monster.max_dmg, monster.luck).max(1);
            // C# dist<=2 且 1/3：BackStep
            if dist <= 2 && fastrand::i32(0..3) == 0 {
                let (nx, ny, dir) = step_away(monster.x, monster.y, target.x, target.y);
                ctx.out_moves.push((monster.object_id, nx, ny, dir));
                return;
            }
            // C# 4/5 DoubleShot（两次投射）/ 1/5 StraightShot（一次投射）
            if fastrand::i32(0..5) > 0 {
                ctx.out_attacks.push(crate::actors::world::ai::AttackAction::Range {
                    attacker_oid: monster.object_id,
                    target_session: target.session_id,
                    target_object_id: target.object_id,
                    damage,
                    spell_id: 0,
                });
                ctx.out_attacks.push(crate::actors::world::ai::AttackAction::Range {
                    attacker_oid: monster.object_id,
                    target_session: target.session_id,
                    target_object_id: target.object_id,
                    damage,
                    spell_id: 1,
                });
            } else {
                ctx.out_attacks.push(crate::actors::world::ai::AttackAction::Range {
                    attacker_oid: monster.object_id,
                    target_session: target.session_id,
                    target_object_id: target.object_id,
                    damage,
                    spell_id: 0,
                });
            }
            return;
        }

        // C# ProcessTarget：出攻击范围才 MoveTo（不风筝）
        if dist > ATTACK_RANGE && ctx.tick_count >= monster.next_move_tick {
            let (nx, ny, dir) = step_toward(monster.x, monster.y, target.x, target.y);
            ctx.out_moves.push((monster.object_id, nx, ny, dir));
            monster.next_move_tick = ctx.tick_count + monster.ai_profile.move_interval;
            monster.ai_state = crate::actors::world::MonsterAiState::Chase;
        }
    }
}
