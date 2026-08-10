//! DarkBeast（暗兽）behavior
//!
//! C# 参考：Server/MirObjects/Monsters/DarkBeast.cs
//! 机制：80% 物理近战（DC）/ 20% 魔法近战（MC，Type=1）；
//!      命中后若 Info.Effect==1：1/5 概率 出血毒（5s，值=SP，tick 1000）

use crate::actors::world::MonsterState;
use crate::actors::world::ai::behavior::MonsterBehavior;
use crate::actors::world::ai::ctx::AiCtx;
use crate::actors::world::ai::helpers::*;
use crate::combat::poison::Poison;
use mir2_shared::enums::PoisonType;

const VIEW_RANGE: i32 = 12;

pub struct DarkBeastBehavior;

impl DarkBeastBehavior {
    pub fn new() -> Self {
        Self
    }
}

impl MonsterBehavior for DarkBeastBehavior {
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
            ctx.out_attacks.push(crate::actors::world::ai::AttackAction::Melee {
                attacker_oid: monster.object_id,
                target_session: target.session_id,
                damage,
                spell_id: 0,
                attack_type: if magic { 1 } else { 0 },
            });
            // C# CompleteAttack：poison && Info.Effect==1 → PoisonTarget(5, 5, Bleeding)
            if magic && monster.effect == 1 && fastrand::i32(0..5) == 0 {
                ctx.out_poisons.push(crate::actors::world::ai::PoisonPlayer {
                    session_id: target.session_id,
                    poison: Poison::new(PoisonType::BLEEDING, 5, poison_sc_value(monster), 1000),
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
