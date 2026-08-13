//! ArcherGuard（弓箭守卫）behavior
//!
//! C# 参考：Server/MirObjects/Monsters/ArcherGuard.cs（继承 Guard）
//! 机制：CanMove=false；始终远程投射（对怪 int.Max，此处仅玩家）

use crate::actors::world::ai::behavior::MonsterBehavior;
use crate::actors::world::ai::ctx::AiCtx;
use crate::actors::world::ai::helpers::*;
use crate::actors::world::MonsterState;

const VIEW_RANGE: i32 = 12;

pub struct ArcherGuardBehavior;

impl Default for ArcherGuardBehavior {
    fn default() -> Self {
        Self::new()
    }
}

impl ArcherGuardBehavior {
    pub fn new() -> Self {
        Self
    }
}

impl MonsterBehavior for ArcherGuardBehavior {
    fn can_move(&self) -> bool {
        false
    }

    fn process_tick(&mut self, monster: &mut MonsterState, ctx: &mut AiCtx) {
        // #2436：C# ArcherGuard（AI 57 = TownArcher）只攻击红名玩家（PKPoints>=200，FindTarget 跳过清白）
        let target = match ctx.nearest_target(monster.x, monster.y, VIEW_RANGE, monster.map_index) {
            Some(t) if is_red_name(t.pk_points) => *t,
            _ => return,
        };
        monster.target_session = Some(target.session_id);
        let dist = max_distance(monster.x, monster.y, target.x, target.y);

        if dist <= VIEW_RANGE && ctx.tick_count >= monster.next_attack_tick {
            monster.next_attack_tick = ctx.tick_count + monster.ai_profile.attack_cooldown;
            let damage = crate::combat::attack::get_attack_power(
                monster.min_dmg,
                monster.max_dmg,
                monster.luck,
            )
            .max(1);
            ctx.out_attacks
                .push(crate::actors::world::ai::AttackAction::Range {
                    attacker_oid: monster.object_id,
                    target_session: target.session_id,
                    target_object_id: target.object_id,
                    damage,
                    spell_id: 0,
                });
        }
    }
}
