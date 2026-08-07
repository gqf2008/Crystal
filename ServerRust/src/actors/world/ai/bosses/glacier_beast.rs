//! GlacierBeast（冰川兽）behavior
//!
//! C# 参考：Server/MirObjects/Monsters/GlacierBeast.cs（继承 CrazyManworm.cs）
//! 机制：
//!   - 继承 CrazyManworm：2/3 物理近战（DC，attackType=0）/ 1/3 魔法近战（MC，attackType=1）
//!   - GlacierBeast CompleteAttack：attackType==1 时 1/3 减速毒（5s，tick 2000）

use crate::actors::world::MonsterState;
use crate::actors::world::ai::behavior::MonsterBehavior;
use crate::actors::world::ai::ctx::AiCtx;
use crate::actors::world::ai::helpers::*;
use crate::combat::poison::Poison;
use mir2_shared::enums::PoisonType;

const VIEW_RANGE: i32 = 12;

pub struct GlacierBeastBehavior;

impl GlacierBeastBehavior {
    pub fn new() -> Self {
        Self
    }
}

impl MonsterBehavior for GlacierBeastBehavior {
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
            // C# CrazyManworm：2/3 物理（attackType=0）/ 1/3 魔法（attackType=1）
            let magic = fastrand::i32(0..3) == 0;
            ctx.out_attacks.push(crate::actors::world::ai::AttackAction::Melee {
                attacker_oid: monster.object_id,
                target_session: target.session_id,
                damage,
                spell_id: 0,
                attack_type: if magic { 1 } else { 0 },
            });
            // C# GlacierBeast CompleteAttack：attackType==1 → PoisonTarget(3, 5, Slow, 2000)：1/3、5s
            if magic && fastrand::i32(0..3) == 0 {
                ctx.out_poisons.push(crate::actors::world::ai::PoisonPlayer {
                    session_id: target.session_id,
                    poison: Poison::new(PoisonType::SLOW, 5, 0, 2000),
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
