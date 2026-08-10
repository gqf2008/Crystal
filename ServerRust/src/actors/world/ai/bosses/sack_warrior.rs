//! SackWarrior（布袋战士）behavior
//!
//! C# 参考：Server/MirObjects/Monsters/SackWarrior.cs
//! 机制：近战；2/3 Halfmoon（4 格弧，弧内命中目标独立 1/3 出血毒）/ 1/3 魔法近战（MC）；

use crate::actors::world::MonsterState;
use crate::actors::world::ai::behavior::MonsterBehavior;
use crate::actors::world::ai::ctx::AiCtx;
use crate::actors::world::ai::helpers::*;
use crate::combat::poison::Poison;
use mir2_shared::enums::PoisonType;

const VIEW_RANGE: i32 = 12;

pub struct SackWarriorBehavior;

impl SackWarriorBehavior {
    pub fn new() -> Self {
        Self
    }
}

impl MonsterBehavior for SackWarriorBehavior {
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
            // C# Random.Next(3) > 0：2/3 Halfmoon / 1/3 魔法
            if fastrand::i32(0..3) > 0 {
                let dir = direction_towards(monster.x, monster.y, target.x, target.y);
                monster.direction = dir;
                ctx.out_attacks.push(crate::actors::world::ai::AttackAction::Arc {
                    attacker_oid: monster.object_id,
                    center_x: monster.x,
                    center_y: monster.y,
                    direction: dir,
                    count: 4,
                    damage,
                    spell_id: 0,
                    attack_type: 0,
                });
                // C# CompleteAttack：每个命中目标独立 1/3 出血毒（5s，tick 1000）
                let cells = arc_cells(monster.x, monster.y, dir, 4);
                let hit: Vec<u64> = ctx.find_targets_in_cells(&cells, monster.map_index)
                    .iter().map(|p| p.session_id).collect();
                for sid in hit {
                    if fastrand::i32(0..3) == 0 {
                        ctx.out_poisons.push(crate::actors::world::ai::PoisonPlayer {
                            session_id: sid,
                            poison: Poison::new(PoisonType::BLEEDING, 5, damage, 1000),
                        });
                    }
                }
            } else {
                ctx.out_attacks.push(crate::actors::world::ai::AttackAction::Melee {
                    attacker_oid: monster.object_id,
                    target_session: target.session_id,
                    damage,
                    spell_id: 0,
                    attack_type: 1,
                });
                // C# CompleteAttack：1/3 出血毒（5s，tick 1000）
                if fastrand::i32(0..3) == 0 {
                    ctx.out_poisons.push(crate::actors::world::ai::PoisonPlayer {
                        session_id: target.session_id,
                        poison: Poison::new(PoisonType::BLEEDING, 5, damage, 1000),
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
