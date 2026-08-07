//! HellCannibal（地狱食人花）behavior
//!
//! C# 参考：Server/MirObjects/Monsters/HellCannibal.cs
//! 机制：3/4 物理近战（DC）/ 1/4 毒分支（damage=0）：
//!      毒分支 FindAllTargets(2)，每个目标 100% 红毒（duration=random(0..SP/2)、值=SP、tick 1000）

use crate::actors::world::MonsterState;
use crate::actors::world::ai::behavior::MonsterBehavior;
use crate::actors::world::ai::ctx::AiCtx;
use crate::actors::world::ai::helpers::*;
use crate::combat::poison::Poison;
use mir2_shared::enums::PoisonType;

const VIEW_RANGE: i32 = 12;
const AOE_RADIUS: i32 = 2;

pub struct HellCannibalBehavior;

impl HellCannibalBehavior {
    pub fn new() -> Self {
        Self
    }
}

impl MonsterBehavior for HellCannibalBehavior {
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
            // C# Envir.Random.Next(4) > 0：3/4 物理 / 1/4 毒分支
            if fastrand::i32(0..4) > 0 {
                ctx.out_attacks.push(crate::actors::world::ai::AttackAction::Melee {
                    attacker_oid: monster.object_id,
                    target_session: target.session_id,
                    damage,
                    spell_id: 0,
                    attack_type: 0,
                });
            } else {
                // C# 毒分支 damage=0：FindAllTargets(2) + 100% 红毒（duration=random(0..SP/2)，值=SP）
                ctx.out_attacks.push(crate::actors::world::ai::AttackAction::Aoe {
                    attacker_oid: monster.object_id,
                    center_x: monster.x,
                    center_y: monster.y,
                    radius: AOE_RADIUS,
                    damage: 0,
                    spell_id: 0,
                });
                let nearby: Vec<u64> = ctx.find_targets_in_range(monster.x, monster.y, AOE_RADIUS, monster.map_index)
                    .iter().map(|p| p.session_id).collect();
                for sid in nearby {
                    let duration = fastrand::i32(0..(damage / 2).max(1));
                    ctx.out_poisons.push(crate::actors::world::ai::PoisonPlayer {
                        session_id: sid,
                        poison: Poison::new(PoisonType::RED, duration.max(0) as u32, damage, 1000),
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
