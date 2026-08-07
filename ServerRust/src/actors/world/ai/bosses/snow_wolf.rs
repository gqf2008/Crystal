//! SnowWolf（雪狼）behavior
//!
//! C# 参考：Server/MirObjects/Monsters/SnowWolf.cs
//! 机制：80% 物理近战（DC）/ 20% 魔法近战（MC，Type=1）；
//!      魔法命中时对周围 2 格 AOE，且每个目标 1/4 减速毒 + 1/8 冰冻毒（5s，tick 2000）

use crate::actors::world::MonsterState;
use crate::actors::world::ai::behavior::MonsterBehavior;
use crate::actors::world::ai::ctx::AiCtx;
use crate::actors::world::ai::helpers::*;
use crate::combat::poison::Poison;
use mir2_shared::enums::PoisonType;

const VIEW_RANGE: i32 = 12;
const AOE_RADIUS: i32 = 2;

pub struct SnowWolfBehavior;

impl SnowWolfBehavior {
    pub fn new() -> Self {
        Self
    }
}

impl MonsterBehavior for SnowWolfBehavior {
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
            // C# Envir.Random.Next(5) > 0：80% 物理 / 20% 魔法（Type=1）
            let magic = fastrand::i32(0..5) == 0;
            if !magic {
                ctx.out_attacks.push(crate::actors::world::ai::AttackAction::Melee {
                    attacker_oid: monster.object_id,
                    target_session: target.session_id,
                    damage,
                    spell_id: 0,
                    attack_type: 0,
                });
            } else {
                // C# CompleteAttack：FindAllTargets(2) AOE + 减速/冰冻毒
                ctx.out_attacks.push(crate::actors::world::ai::AttackAction::Aoe {
                    attacker_oid: monster.object_id,
                    center_x: monster.x,
                    center_y: monster.y,
                    radius: AOE_RADIUS,
                    damage,
                    spell_id: 0,
                });
                let nearby: Vec<u64> = ctx.find_targets_in_range(monster.x, monster.y, AOE_RADIUS, monster.map_index)
                    .iter().map(|p| p.session_id).collect();
                for sid in nearby {
                    // PoisonTarget(4, 5, Slow, 2000)：1/4 概率
                    if fastrand::i32(0..4) == 0 {
                        ctx.out_poisons.push(crate::actors::world::ai::PoisonPlayer {
                            session_id: sid,
                            poison: Poison::new(PoisonType::SLOW, 5, 0, 2000),
                        });
                    }
                    // PoisonTarget(8, 5, Frozen, 2000)：1/8 概率
                    if fastrand::i32(0..8) == 0 {
                        ctx.out_poisons.push(crate::actors::world::ai::PoisonPlayer {
                            session_id: sid,
                            poison: Poison::new(PoisonType::FROZEN, 5, 0, 2000),
                        });
                    }
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
