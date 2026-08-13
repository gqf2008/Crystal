//! WhiteFoxman（白狐人）behavior
//!
//! C# 参考：Server/MirObjects/Monsters/WhiteFoxman.cs
//! 机制：
//!   - AttackRange=6 远程风筝（<6 远离，>=6 接近）
//!   - 攻击 7/8：ObjectRangeAttack + RangeDamage（MACAgility，DC）
//!   - 攻击 1/8：Type=1 纯毒（CompleteAttack：levelgap=50-目标等级，Random(20)<4+levelgap → Slow 5s）

use crate::actors::world::ai::behavior::MonsterBehavior;
use crate::actors::world::ai::ctx::AiCtx;
use crate::actors::world::ai::helpers::*;
use crate::actors::world::MonsterState;
use crate::combat::poison::Poison;
use mir2_shared::enums::PoisonType;

const ATTACK_RANGE: i32 = 6;

pub struct WhiteFoxmanBehavior;

impl Default for WhiteFoxmanBehavior {
    fn default() -> Self {
        Self::new()
    }
}

impl WhiteFoxmanBehavior {
    pub fn new() -> Self {
        Self
    }
}

impl MonsterBehavior for WhiteFoxmanBehavior {
    fn process_tick(&mut self, monster: &mut MonsterState, ctx: &mut AiCtx) {
        let target = match ctx.nearest_target(monster.x, monster.y, ATTACK_RANGE, monster.map_index)
        {
            Some(t) => *t,
            None => return,
        };
        monster.target_session = Some(target.session_id);
        let dist = max_distance(monster.x, monster.y, target.x, target.y);

        if dist <= ATTACK_RANGE && ctx.tick_count >= monster.next_attack_tick {
            monster.next_attack_tick = ctx.tick_count + monster.ai_profile.attack_cooldown;
            let damage = crate::combat::attack::get_attack_power(
                monster.min_dmg,
                monster.max_dmg,
                monster.luck,
            )
            .max(1);
            // C# 7/8 弹道 DC；1/8 Type=1 纯毒（CompleteAttack 才上毒，无投射伤害）
            if fastrand::i32(0..8) != 0 {
                ctx.out_attacks
                    .push(crate::actors::world::ai::AttackAction::Range {
                        attacker_oid: monster.object_id,
                        target_session: target.session_id,
                        target_object_id: target.object_id,
                        damage,
                        spell_id: 0,
                    });
            } else {
                // C# CompleteAttack：levelgap=50-target.Level；Random.Next(20) < 4+levelgap → Slow(1,5,Slow,1000)
                let level_gap = (50 - target.level as i32).max(0);
                if fastrand::i32(0..20) < 4 + level_gap {
                    ctx.out_poisons
                        .push(crate::actors::world::ai::PoisonPlayer {
                            session_id: target.session_id,
                            poison: Poison::new(
                                PoisonType::SLOW,
                                5,
                                crate::actors::world::ai::helpers::poison_sc_value(monster),
                                1000,
                            ),
                        });
                }
            }
            return;
        }

        if ctx.tick_count >= monster.next_move_tick {
            // C# 风筝：>=6 接近，<6 远离
            let (nx, ny, dir) = if dist >= ATTACK_RANGE {
                step_toward(monster.x, monster.y, target.x, target.y)
            } else {
                step_away(monster.x, monster.y, target.x, target.y)
            };
            ctx.out_moves.push((monster.object_id, nx, ny, dir));
            monster.next_move_tick = ctx.tick_count + monster.ai_profile.move_interval;
            monster.ai_state = crate::actors::world::MonsterAiState::Chase;
        }
    }
}
