//! DemonGuard（恶魔守卫）behavior
//!
//! C# 参考：Server/MirObjects/Monsters/DemonGuard.cs（继承 ZumaMonster）
//! 机制：2/3 物理近战（DC，ACAgility）/ 1/3 魔法近战（MC，ACAgility）
//! 说明：C# 复活机制（Revive 1-2 次，25% 递减）依赖重生逻辑，暂不实现

use crate::actors::world::MonsterState;
use crate::actors::world::ai::behavior::MonsterBehavior;
use crate::actors::world::ai::ctx::AiCtx;
use crate::actors::world::ai::helpers::*;

const VIEW_RANGE: i32 = 12;

pub struct DemonGuardBehavior;

impl DemonGuardBehavior {
    pub fn new() -> Self {
        Self
    }
}

impl MonsterBehavior for DemonGuardBehavior {
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
            // C# Random.Next(3) > 0：2/3 物理 / 1/3 魔法
            let magic = fastrand::i32(0..3) == 0;
            ctx.out_attacks.push(crate::actors::world::ai::AttackAction::Melee {
                attacker_oid: monster.object_id,
                target_session: target.session_id,
                damage,
                spell_id: 0,
                attack_type: if magic { 1 } else { 0 },
            });
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
