//! Nadz（纳兹）behavior
//!
//! C# 参考：Server/MirObjects/Monsters/Nadz.cs
//! 机制：
//!   - InAttackRange：3 格内（C# 守卫后 (x<=3&&y<=3) 恒真，即切比雪夫距离 <=3）
//!   - 2/3 近战（C# damage=0，AC）+ 命中后 1/3 麻痹毒（5s，tick 1000）
//!   - 1/3 半月 AOE（FindAllTargets(3)，用 AOE 半径 3 近似）

use crate::actors::world::MonsterState;
use crate::actors::world::ai::behavior::MonsterBehavior;
use crate::actors::world::ai::ctx::AiCtx;
use crate::actors::world::ai::helpers::*;
use crate::combat::poison::Poison;
use mir2_shared::enums::PoisonType;

const VIEW_RANGE: i32 = 12;
const ATTACK_RANGE: i32 = 3;

pub struct NadzBehavior;

impl NadzBehavior {
    pub fn new() -> Self {
        Self
    }
}

impl MonsterBehavior for NadzBehavior {
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
            // C# Envir.Random.Next(3) > 0：2/3 近战（damage=0）/ 1/3 半月 AOE
            if fastrand::i32(0..3) > 0 {
                ctx.out_attacks.push(crate::actors::world::ai::AttackAction::Melee {
                    attacker_oid: monster.object_id,
                    target_session: target.session_id,
                    damage: 0,
                    spell_id: 0,
                    attack_type: 0,
                });
                // C# 非半月：PoisonTarget(3, 5, Paralysis)：1/3 概率、5s
                if fastrand::i32(0..3) == 0 {
                    ctx.out_poisons.push(crate::actors::world::ai::PoisonPlayer {
                        session_id: target.session_id,
                        poison: Poison::new(PoisonType::PARALYSIS, 5, 0, 1000),
                    });
                }
            } else {
                // C# Halfmoon：FindAllTargets(3)，用 AOE 半径 3 近似
                ctx.out_attacks.push(crate::actors::world::ai::AttackAction::Aoe {
                    attacker_oid: monster.object_id,
                    center_x: monster.x,
                    center_y: monster.y,
                    radius: ATTACK_RANGE,
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
