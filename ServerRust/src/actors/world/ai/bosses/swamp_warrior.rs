//! SwampWarrior（沼泽战士）behavior
//!
//! C# 参考：Server/MirObjects/Monsters/SwampWarrior.cs
//! 机制：
//!   - AttackRange=6；近战（dist<=1）DC / 远程 MC（MAC，攻速+500ms）
//!   - CompleteRangeAttack：Envir.Random.Next(1) 恒为 0 → 恒走 else 分支 Red 毒
//!     PoisonTarget(8, 5, Red, 2000)：1/8、5s、tick 2000（C# 注释 50/50，实际代码恒 Red，按代码对齐）

use crate::actors::world::MonsterState;
use crate::actors::world::ai::behavior::MonsterBehavior;
use crate::actors::world::ai::ctx::AiCtx;
use crate::actors::world::ai::helpers::*;
use crate::combat::poison::Poison;
use mir2_shared::enums::PoisonType;

const ATTACK_RANGE: i32 = 6;

pub struct SwampWarriorBehavior;

impl SwampWarriorBehavior {
    pub fn new() -> Self {
        Self
    }
}

impl MonsterBehavior for SwampWarriorBehavior {
    fn process_tick(&mut self, monster: &mut MonsterState, ctx: &mut AiCtx) {
        let target = match ctx.nearest_target(monster.x, monster.y, ATTACK_RANGE, monster.map_index) {
            Some(t) => *t,
            None => return,
        };
        monster.target_session = Some(target.session_id);
        let dist = max_distance(monster.x, monster.y, target.x, target.y);

        if dist <= ATTACK_RANGE && ctx.tick_count >= monster.next_attack_tick {
            let damage = crate::combat::attack::get_attack_power(monster.min_dmg, monster.max_dmg, monster.luck).max(1);
            if dist <= 1 {
                monster.next_attack_tick = ctx.tick_count + monster.ai_profile.attack_cooldown;
                ctx.out_attacks.push(crate::actors::world::ai::AttackAction::Melee {
                    attacker_oid: monster.object_id,
                    target_session: target.session_id,
                    damage,
                    spell_id: 0,
                    attack_type: 0,
                });
            } else {
                monster.next_attack_tick = ctx.tick_count + monster.ai_profile.attack_cooldown + 5;
                ctx.out_attacks.push(crate::actors::world::ai::AttackAction::Range {
                    attacker_oid: monster.object_id,
                    target_session: target.session_id,
                    target_object_id: target.object_id,
                    damage,
                    spell_id: 0,
                });
                // C# 恒 Red：PoisonTarget(8, 5, Red, 2000)：1/8、5s
                if fastrand::i32(0..8) == 0 {
                    ctx.out_poisons.push(crate::actors::world::ai::PoisonPlayer {
                        session_id: target.session_id,
                        poison: Poison::new(PoisonType::RED, 5, damage, 2000),
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
