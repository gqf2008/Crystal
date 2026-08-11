//! GuardianRock（守护之石）behavior
//!
//! C# 参考：Server/MirObjects/Monsters/GuardianRock.cs
//! 机制：CanMove=false；视野内远程攻击（ObjectRangeAttack）+ PullAttack 把玩家拉向岩石。

use crate::actors::world::MonsterState;
use crate::actors::world::ai::behavior::MonsterBehavior;
use crate::actors::world::ai::ctx::AiCtx;
use crate::actors::world::ai::helpers::*;

pub struct GuardianRockBehavior;

impl GuardianRockBehavior {
    pub fn new() -> Self {
        Self
    }
}

impl MonsterBehavior for GuardianRockBehavior {
    fn can_move(&self) -> bool {
        false
    }

    fn can_regen(&self) -> bool {
        false
    }

    fn process_tick(&mut self, monster: &mut MonsterState, ctx: &mut AiCtx) {
        // C# InAttackRange = Info.ViewRange（用 aggro_range 近似）
        let view = monster.ai_profile.aggro_range.max(3);
        let target = match ctx.nearest_target(monster.x, monster.y, view, monster.map_index) {
            Some(t) => *t,
            None => return,
        };
        monster.target_session = Some(target.session_id);
        let dist = max_distance(monster.x, monster.y, target.x, target.y);
        if dist <= view && ctx.tick_count >= monster.next_attack_tick {
            monster.next_attack_tick = ctx.tick_count + monster.ai_profile.attack_cooldown;
            // C# CompleteAttack：ObjectRangeAttack 远程伤害
            let damage = crate::combat::attack::get_attack_power(monster.min_dmg, monster.max_dmg, monster.luck).max(1);
            ctx.out_attacks.push(crate::actors::world::ai::AttackAction::Range {
                attacker_oid: monster.object_id,
                target_session: target.session_id,
                target_object_id: target.object_id,
                damage,
                spell_id: 0,
            });
            // C# PullAttack：把目标向岩石拉（pushdir = 目标→岩石方向），距离 = maxdist-1 截断 1..4
            let pull_dist = (dist - 1).clamp(1, 4);
            let dir = direction_towards(target.x, target.y, monster.x, monster.y);
            ctx.out_pushes.push(crate::actors::world::ai::PushPlayer {
                session_id: target.session_id,
                dir,
                distance: pull_dist,
            });
        }
    }
}
