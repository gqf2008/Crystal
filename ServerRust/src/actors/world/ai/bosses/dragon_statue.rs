//! DragonStatue（龙雕像）behavior
//!
//! C# 参考：Server/MirObjects/Monsters/DragonStatue.cs
//! 机制：CanMove=false；攻击：FindAllTargets(2, 目标) AOE（MAC）
//! 说明：睡眠机制（死亡后 15 分钟满血苏醒）依赖重生逻辑，暂不实现

use crate::actors::world::MonsterState;
use crate::actors::world::ai::behavior::MonsterBehavior;
use crate::actors::world::ai::ctx::AiCtx;
use crate::actors::world::ai::helpers::*;

const VIEW_RANGE: i32 = 12;
const AOE_RADIUS: i32 = 2;

pub struct DragonStatueBehavior;

impl DragonStatueBehavior {
    pub fn new() -> Self {
        Self
    }
}

impl MonsterBehavior for DragonStatueBehavior {
    fn can_move(&self) -> bool {
        false
    }

    fn process_tick(&mut self, monster: &mut MonsterState, ctx: &mut AiCtx) {
        let target = match ctx.nearest_target(monster.x, monster.y, VIEW_RANGE, monster.map_index) {
            Some(t) => *t,
            None => return,
        };
        monster.target_session = Some(target.session_id);

        if ctx.tick_count >= monster.next_attack_tick {
            monster.next_attack_tick = ctx.tick_count + monster.ai_profile.attack_cooldown;
            let damage = crate::combat::attack::get_attack_power(monster.min_dmg, monster.max_dmg, monster.luck).max(1);
            // C# CompleteAttack：FindAllTargets(2, Target.CurrentLocation)
            ctx.out_attacks.push(crate::actors::world::ai::AttackAction::Aoe {
                attacker_oid: monster.object_id,
                center_x: target.x,
                center_y: target.y,
                radius: AOE_RADIUS,
                damage,
                spell_id: 0,
            });
        }
    }
}
