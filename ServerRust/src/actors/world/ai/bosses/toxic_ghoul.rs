//! ToxicGhoul（毒尸，AI 28）behavior
//!
//! C# 参考：Server/MirObjects/Monsters/ToxicGhoul.cs
//! 机制：
//!   - 近战（MACAgility），命中后 1/8 施加绿毒（5s，值=SP，tick 2000）
//!   - Die（仅 Info.Effect==1）：1 格内 AOE 伤害（ACAgility）+ 1/5 绿毒（5s，值=SP，tick 2000）

use crate::actors::world::MonsterState;
use crate::actors::world::ai::behavior::MonsterBehavior;
use crate::actors::world::ai::ctx::AiCtx;
use crate::actors::world::ai::helpers::*;

const VIEW_RANGE: i32 = 8;
const DEATH_RADIUS: i32 = 1;

pub struct ToxicGhoulBehavior;

impl ToxicGhoulBehavior {
    pub fn new() -> Self {
        Self
    }
}

impl MonsterBehavior for ToxicGhoulBehavior {
    fn process_tick(&mut self, monster: &mut MonsterState, ctx: &mut AiCtx) {
        let target = match ctx.nearest_target(monster.x, monster.y, VIEW_RANGE, monster.map_index) {
            Some(t) => *t,
            None => return,
        };
        monster.target_session = Some(target.session_id);
        let dist = max_distance(monster.x, monster.y, target.x, target.y);

        if dist <= 1 && ctx.tick_count >= monster.next_attack_tick {
            monster.next_attack_tick = ctx.tick_count + monster.ai_profile.attack_cooldown;
            let damage = crate::combat::attack::get_attack_power(monster.min_dmg, monster.max_dmg, 0).max(1);
            ctx.out_attacks.push(crate::actors::world::ai::AttackAction::Melee {
                attacker_oid: monster.object_id,
                target_session: target.session_id,
                damage,
                spell_id: 0,
                attack_type: 0,
            });
            // C# CompleteAttack：PoisonTarget(8, 5, Green, 2000)：1/8 概率、5s、值=SP
            if fastrand::i32(0..8) == 0 {
                ctx.out_poisons.push(crate::actors::world::ai::PoisonPlayer {
                    session_id: target.session_id,
                    poison: crate::combat::poison::Poison::new(
                        mir2_shared::enums::PoisonType::GREEN, 5, damage, 2000),
                });
            }
            return;
        }

        // 追击
        if ctx.tick_count >= monster.next_move_tick {
            let (nx, ny, dir) = step_toward(monster.x, monster.y, target.x, target.y);
            ctx.out_moves.push((monster.object_id, nx, ny, dir));
            monster.next_move_tick = ctx.tick_count + monster.ai_profile.move_interval;
            monster.ai_state = crate::actors::world::MonsterAiState::Chase;
        }
    }

    /// C# Die/CompleteDeath（仅 Info.Effect==1）：1 格内 AOE 伤害 + 1/5 绿毒
    fn on_die(&mut self, monster: &mut MonsterState, ctx: &mut AiCtx) {
        if monster.effect != 1 {
            return;
        }
        let damage = crate::combat::attack::get_attack_power(monster.min_dmg, monster.max_dmg, 0).max(1);
        ctx.out_attacks.push(crate::actors::world::ai::AttackAction::Aoe {
            attacker_oid: monster.object_id,
            center_x: monster.x,
            center_y: monster.y,
            radius: DEATH_RADIUS,
            damage,
            spell_id: 0,
        });
        let nearby: Vec<u64> = ctx.find_targets_in_range(monster.x, monster.y, DEATH_RADIUS, monster.map_index)
            .iter().map(|t| t.session_id).collect();
        for sid in nearby {
            if fastrand::i32(0..5) == 0 {
                ctx.out_poisons.push(crate::actors::world::ai::PoisonPlayer {
                    session_id: sid,
                    poison: crate::combat::poison::Poison::new(
                        mir2_shared::enums::PoisonType::GREEN, 5, damage, 2000),
                });
            }
        }
    }
}
