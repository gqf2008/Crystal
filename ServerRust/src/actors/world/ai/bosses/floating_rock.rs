//! FloatingRock（浮石）behavior
//!
//! C# 参考：Server/MirObjects/Monsters/FloatingRock.cs
//! 机制：CanMove=false；Die：FindAllTargets(3) AOE（AC）
//! 说明：召唤克隆依赖任意怪克隆原语，暂不实现

use crate::actors::world::MonsterState;
use crate::actors::world::ai::behavior::MonsterBehavior;
use crate::actors::world::ai::ctx::AiCtx;
use crate::actors::world::ai::helpers::*;

const VIEW_RANGE: i32 = 12;
const DEATH_RADIUS: i32 = 3;

pub struct FloatingRockBehavior;

impl FloatingRockBehavior {
    pub fn new() -> Self {
        Self
    }
}

impl MonsterBehavior for FloatingRockBehavior {
    fn can_move(&self) -> bool {
        false
    }

    fn can_regen(&self) -> bool {
        false
    }

    fn process_tick(&mut self, _monster: &mut MonsterState, _ctx: &mut AiCtx) {
        // C# ProcessTarget：仅召唤克隆（暂不实现），无攻击
    }

    /// C# Die：AOE 3 伤害（AC）
    fn on_die(&mut self, monster: &mut MonsterState, ctx: &mut AiCtx) {
        let damage = crate::combat::attack::get_attack_power(monster.min_dmg, monster.max_dmg, 0).max(1);
        ctx.out_attacks.push(crate::actors::world::ai::AttackAction::Aoe {
            attacker_oid: monster.object_id,
            center_x: monster.x,
            center_y: monster.y,
            radius: DEATH_RADIUS,
            damage,
            spell_id: 0,
        });
    }
}
