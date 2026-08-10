//! HellPirate（地狱海盗）behavior
//!
//! C# 参考：Server/MirObjects/Monsters/HellPirate.cs
//! 机制：近战（dist<=1）：2/3 base.Attack（DC）/ 1/3 Fullmoon（8 格）

use crate::actors::world::MonsterState;
use crate::actors::world::ai::behavior::MonsterBehavior;
use crate::actors::world::ai::ctx::AiCtx;
use crate::actors::world::ai::helpers::*;

const VIEW_RANGE: i32 = 12;

pub struct HellPirateBehavior;

impl HellPirateBehavior {
    pub fn new() -> Self {
        Self
    }
}

impl MonsterBehavior for HellPirateBehavior {
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
            // C# Random.Next(3) > 0：2/3 普攻 / 1/3 Fullmoon
            if fastrand::i32(0..3) > 0 {
                ctx.out_attacks.push(crate::actors::world::ai::AttackAction::Melee {
                    attacker_oid: monster.object_id,
                    target_session: target.session_id,
                    damage,
                    spell_id: 0,
                    attack_type: 0,
                });
            } else {
                // C# FullmoonAttack(damage)：8 格（8 方向 × 距离 1，无中心）
                let dir = direction_towards(monster.x, monster.y, target.x, target.y);
                monster.direction = dir;
                ctx.out_attacks.push(crate::actors::world::ai::AttackAction::Arc {
                    attacker_oid: monster.object_id,
                    center_x: monster.x,
                    center_y: monster.y,
                    direction: dir,
                    count: 8,
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
