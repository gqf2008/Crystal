//! SepTaoist（圣战道士）behavior（简化）
//!
//! C# 参考：Server/MirObjects/Monsters/SepTaoist.cs
//! 机制：远程；绿毒/红毒交替（duration=SP+rand(1..3)*7，值=SP/15+4，tick 2000）；
//!      1/8 诅咒减速（5s）；否则魂火（MACAgility）
//! 简化：目标毒状态不可知，用行为内计数器交替绿/红

use crate::actors::world::MonsterState;
use crate::actors::world::ai::behavior::MonsterBehavior;
use crate::actors::world::ai::ctx::AiCtx;
use crate::actors::world::ai::helpers::*;
use crate::combat::poison::Poison;
use mir2_shared::enums::PoisonType;

const VIEW_RANGE: i32 = 12;

pub struct SepTaoistBehavior {
    poison_cycle: u8,
}

impl SepTaoistBehavior {
    pub fn new() -> Self {
        Self { poison_cycle: 0 }
    }
}

impl MonsterBehavior for SepTaoistBehavior {
    fn process_tick(&mut self, monster: &mut MonsterState, ctx: &mut AiCtx) {
        let target = match ctx.nearest_target(monster.x, monster.y, VIEW_RANGE, monster.map_index) {
            Some(t) => *t,
            None => return,
        };
        monster.target_session = Some(target.session_id);
        let dist = max_distance(monster.x, monster.y, target.x, target.y);

        if dist <= VIEW_RANGE && ctx.tick_count >= monster.next_attack_tick {
            monster.next_attack_tick = ctx.tick_count + monster.ai_profile.attack_cooldown;
            let damage = crate::combat::attack::get_attack_power(monster.min_dmg, monster.max_dmg, monster.luck).max(1);
            // C#：无绿无红 → 绿毒；绿 → 红；红 → 绿（用计数器近似）
            if self.poison_cycle == 0 {
                self.poison_cycle = 1;
                let dur = damage + fastrand::i32(1..4) * 7;
                ctx.out_poisons.push(crate::actors::world::ai::PoisonPlayer {
                    session_id: target.session_id,
                    poison: Poison::new(PoisonType::GREEN, dur as u32, damage / 15 + 4, 2000),
                });
                return;
            }
            if self.poison_cycle == 1 {
                self.poison_cycle = 2;
                let dur = damage + fastrand::i32(1..4) * 7;
                ctx.out_poisons.push(crate::actors::world::ai::PoisonPlayer {
                    session_id: target.session_id,
                    poison: Poison::new(PoisonType::RED, dur as u32, 0, 2000),
                });
                return;
            }
            self.poison_cycle = 0;
            // C# 双毒后 1/8 诅咒减速，否则魂火
            if fastrand::i32(0..8) == 0 {
                ctx.out_poisons.push(crate::actors::world::ai::PoisonPlayer {
                    session_id: target.session_id,
                    poison: Poison::new(PoisonType::SLOW, 5, 0, 1000),
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

        if ctx.tick_count >= monster.next_move_tick {
            let (nx, ny, dir) = step_toward(monster.x, monster.y, target.x, target.y);
            ctx.out_moves.push((monster.object_id, nx, ny, dir));
            monster.next_move_tick = ctx.tick_count + monster.ai_profile.move_interval;
            monster.ai_state = crate::actors::world::MonsterAiState::Chase;
        }
    }
}
