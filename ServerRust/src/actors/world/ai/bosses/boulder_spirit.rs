//! BoulderSpirit（巨石之灵）behavior
//!
//! C# 参考：Server/MirObjects/Monsters/BoulderSpirit.cs
//! 机制：CanMove=false、CanAttack=false、CanRegen=false；
//!      ProcessAI：视野内有目标 → Die；
//!      CompleteDeath：FindAllTargets(Info.ViewRange) AOE 伤害（ACAgility）

use crate::actors::world::MonsterState;
use crate::actors::world::ai::behavior::MonsterBehavior;
use crate::actors::world::ai::ctx::AiCtx;
use crate::actors::world::ai::helpers::*;

const VIEW_RANGE: i32 = 12;

pub struct BoulderSpiritBehavior;

impl BoulderSpiritBehavior {
    pub fn new() -> Self {
        Self
    }
}

impl MonsterBehavior for BoulderSpiritBehavior {
    fn can_move(&self) -> bool {
        false
    }

    fn can_regen(&self) -> bool {
        false
    }

    fn process_tick(&mut self, monster: &mut MonsterState, ctx: &mut AiCtx) {
        // C# ProcessAI：视野内有目标 → Die
        let nearby = ctx.find_targets_in_range(monster.x, monster.y, VIEW_RANGE, monster.map_index);
        if !nearby.is_empty() {
            monster.hp = 0;
        }
    }

    /// C# CompleteDeath：视野 AOE 伤害
    fn on_die(&mut self, monster: &mut MonsterState, ctx: &mut AiCtx) {
        let damage = crate::combat::attack::get_attack_power(monster.min_dmg, monster.max_dmg, 0).max(1);
        ctx.out_attacks.push(crate::actors::world::ai::AttackAction::Aoe {
            attacker_oid: monster.object_id,
            center_x: monster.x,
            center_y: monster.y,
            radius: VIEW_RANGE,
            damage,
            spell_id: 0,
        });
    }
}
