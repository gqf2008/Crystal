//! FlameAssassin（火焰刺客）behavior
//!
//! C# 参考：Server/MirObjects/Monsters/FlameAssassin.cs（继承 RightGuard）
//! 机制：AttackRange 风筝（<6 远离，>=6 接近）；远程命中后 100% 减速毒（时长=MC，值=SC，tick 1000）

use crate::actors::world::MonsterState;
use crate::actors::world::ai::behavior::MonsterBehavior;
use crate::actors::world::ai::ctx::AiCtx;
use crate::actors::world::ai::helpers::*;
use crate::combat::poison::Poison;
use mir2_shared::enums::PoisonType;

const ATTACK_RANGE: i32 = 6;

pub struct FlameAssassinBehavior;

impl FlameAssassinBehavior {
    pub fn new() -> Self {
        Self
    }
}

impl MonsterBehavior for FlameAssassinBehavior {
    fn process_tick(&mut self, monster: &mut MonsterState, ctx: &mut AiCtx) {
        let target = match ctx.nearest_target(monster.x, monster.y, ATTACK_RANGE, monster.map_index) {
            Some(t) => *t,
            None => return,
        };
        monster.target_session = Some(target.session_id);
        let dist = max_distance(monster.x, monster.y, target.x, target.y);

        if dist <= ATTACK_RANGE && ctx.tick_count >= monster.next_attack_tick {
            monster.next_attack_tick = ctx.tick_count + monster.ai_profile.attack_cooldown;
            let damage = crate::combat::attack::get_attack_power(monster.min_dmg, monster.max_dmg, monster.luck).max(1);
            ctx.out_attacks.push(crate::actors::world::ai::AttackAction::Range {
                attacker_oid: monster.object_id,
                target_session: target.session_id,
                target_object_id: target.object_id,
                damage,
                spell_id: 0,
            });
            // C# CompleteRangeAttack：PoisonTarget(1, MC, Slow, 1000)：100%、时长=MC 攻秒数、值=SC
            let mc_power = crate::combat::attack::get_attack_power(monster.min_mac, monster.max_mac, 0).max(1);
            let sc_power = crate::combat::attack::get_attack_power(monster.min_sc, monster.max_sc, 0).max(1);
            ctx.out_poisons.push(crate::actors::world::ai::PoisonPlayer {
                session_id: target.session_id,
                poison: Poison::new(PoisonType::SLOW, mc_power as u32, sc_power, 1000),
            });
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
