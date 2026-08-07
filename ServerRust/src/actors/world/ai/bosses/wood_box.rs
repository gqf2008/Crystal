//! WoodBox（木箱）behavior
//!
//! C# 参考：Server/MirObjects/Monsters/WoodBox.cs
//! 机制：CanMove=false；死亡 → CompleteDeath：FindAllTargets(1) AOE 伤害（ACAgility）

use crate::actors::world::MonsterState;
use crate::actors::world::ai::behavior::MonsterBehavior;
use crate::actors::world::ai::ctx::AiCtx;
use crate::actors::world::ai::helpers::*;

const VIEW_RANGE: i32 = 12;
const AOE_RADIUS: i32 = 1;

pub struct WoodBoxBehavior;

impl WoodBoxBehavior {
    pub fn new() -> Self {
        Self
    }
}

impl MonsterBehavior for WoodBoxBehavior {
    fn can_move(&self) -> bool {
        false
    }

    fn can_regen(&self) -> bool {
        false
    }

    fn process_tick(&mut self, _monster: &mut MonsterState, _ctx: &mut AiCtx) {
        // C#：无主动 AI（不可移动、不可攻击）
    }

    /// C# CompleteDeath：1 格 AOE 伤害
    fn on_die(&mut self, monster: &mut MonsterState, ctx: &mut AiCtx) {
        let damage = crate::combat::attack::get_attack_power(monster.min_dmg, monster.max_dmg, 0).max(1);
        ctx.out_attacks.push(crate::actors::world::ai::AttackAction::Aoe {
            attacker_oid: monster.object_id,
            center_x: monster.x,
            center_y: monster.y,
            radius: AOE_RADIUS,
            damage,
            spell_id: 0,
        });
    }
}
