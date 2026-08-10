//! FurbolgArcher（兽人弓手）behavior
//!
//! C# 参考：Server/MirObjects/Monsters/FurbolgArcher.cs
//! 机制：
//!   - AttackRange=6；目标<6 格时远离（风筝），>=6 格时接近
//!   - dist<=2 且 1/5 概率：JumpBack(2) 后跳（反向 2 格 ObjectBackStep）
//!   - 80% 普通投射（DC）/ 20% Type=1 强化投射（DC*1.5）

use crate::actors::world::MonsterState;
use crate::actors::world::ai::behavior::MonsterBehavior;
use crate::actors::world::ai::ctx::AiCtx;
use crate::actors::world::ai::helpers::*;

const ATTACK_RANGE: i32 = 6;

pub struct FurbolgArcherBehavior;

impl FurbolgArcherBehavior {
    pub fn new() -> Self {
        Self
    }
}

impl MonsterBehavior for FurbolgArcherBehavior {
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
            // C# dist<=2 且 1/5：JumpBack(2)（反向 2 格；out_backsteps 应用时校验 walkable + 广播 ObjectBackStep）
            if dist <= 2 && fastrand::i32(0..5) == 0 {
                let dir = (direction_towards(monster.x, monster.y, target.x, target.y) as i32 + 4).rem_euclid(8) as u8;
                ctx.out_backsteps.push((monster.object_id, dir, 2));
                return;
            }
            // C# 80% 普通投射 / 20% Type=1 强化投射（DC*1.5）
            let powered = fastrand::i32(0..5) == 0;
            let dmg = if powered { (damage as f32 * 1.5) as i32 } else { damage };
            ctx.out_attacks.push(crate::actors::world::ai::AttackAction::Range {
                attacker_oid: monster.object_id,
                target_session: target.session_id,
                target_object_id: target.object_id,
                damage: dmg.max(1),
                spell_id: 0,
            });
            return;
        }

        if ctx.tick_count >= monster.next_move_tick {
            // C# 风筝：>=6 格接近，<6 格远离
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
