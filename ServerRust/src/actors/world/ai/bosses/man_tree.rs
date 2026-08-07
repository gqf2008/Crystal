//! ManTree（树人）behavior
//!
//! C# 参考：Server/MirObjects/Monsters/ManTree.cs（继承 ZumaMonster）
//! 机制：
//!   - 7/8：3/4 普通近战（DC）/ 1/4 Halfmoon（用 AOE 半径 1 近似）
//!   - 1/8：BoulderSmash（MC）：FindAllTargets(1, 目标) AOE + 1/5 眩晕毒（5s）

use crate::actors::world::MonsterState;
use crate::actors::world::ai::behavior::MonsterBehavior;
use crate::actors::world::ai::ctx::AiCtx;
use crate::actors::world::ai::helpers::*;
use crate::combat::poison::Poison;
use mir2_shared::enums::PoisonType;

const VIEW_RANGE: i32 = 12;
const AOE_RADIUS: i32 = 1;

pub struct ManTreeBehavior;

impl ManTreeBehavior {
    pub fn new() -> Self {
        Self
    }
}

impl MonsterBehavior for ManTreeBehavior {
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
            // C# Random.Next(8) > 0：7/8 普通分支
            if fastrand::i32(0..8) > 0 {
                // C# Random.Next(4) > 0：3/4 普通近战 / 1/4 Halfmoon
                if fastrand::i32(0..4) > 0 {
                    ctx.out_attacks.push(crate::actors::world::ai::AttackAction::Melee {
                        attacker_oid: monster.object_id,
                        target_session: target.session_id,
                        damage,
                        spell_id: 0,
                        attack_type: 0,
                    });
                } else {
                    ctx.out_attacks.push(crate::actors::world::ai::AttackAction::Aoe {
                        attacker_oid: monster.object_id,
                        center_x: monster.x,
                        center_y: monster.y,
                        radius: AOE_RADIUS,
                        damage,
                        spell_id: 0,
                    });
                }
            } else {
                // C# BoulderSmash：FindAllTargets(1, 目标) + 1/5 眩晕毒（5s）
                ctx.out_attacks.push(crate::actors::world::ai::AttackAction::Aoe {
                    attacker_oid: monster.object_id,
                    center_x: target.x,
                    center_y: target.y,
                    radius: AOE_RADIUS,
                    damage,
                    spell_id: 0,
                });
                if fastrand::i32(0..5) == 0 {
                    ctx.out_poisons.push(crate::actors::world::ai::PoisonPlayer {
                        session_id: target.session_id,
                        poison: Poison::new(PoisonType::STUN, 5, 0, 1000),
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
