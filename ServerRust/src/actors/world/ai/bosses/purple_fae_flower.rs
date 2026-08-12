//! PurpleFaeFlower（紫妖花）behavior
//!
//! C# 参考：Server/MirObjects/Monsters/PurpleFaeFlower.cs（继承 ZumaMonster）
//! 机制：不可移动（CanMove=false，Walk=false，ProcessRoam 空）；视距内远程射击
//!      - dist<=1：近战 DC（AC 防御）
//!      - dist>1 ：投射 DC（MAC 防御），攻速 +500ms（5 tick）
//! 注意：C# InAttackRange 要求 CanFly（不能穿墙射击）；Rust AI 无 LOS 检测，沿用通用 nearest_target

use crate::actors::world::MonsterState;
use crate::actors::world::ai::behavior::MonsterBehavior;
use crate::actors::world::ai::ctx::AiCtx;
use crate::actors::world::ai::helpers::*;

pub struct PurpleFaeFlowerBehavior;

impl PurpleFaeFlowerBehavior {
    pub fn new() -> Self {
        Self
    }
}

impl MonsterBehavior for PurpleFaeFlowerBehavior {
    fn can_move(&self) -> bool {
        false // C# CanMove=false / Walk=false / ProcessRoam 空
    }

    fn process_tick(&mut self, monster: &mut MonsterState, ctx: &mut AiCtx) {
        // C# InAttackRange = Info.ViewRange（用 MonsterAiProfile.aggro_range = info.view_range）
        let view_range = monster.ai_profile.aggro_range.max(1);
        let target = match ctx.nearest_target(monster.x, monster.y, view_range, monster.map_index) {
            Some(t) => *t,
            None => return,
        };
        monster.target_session = Some(target.session_id);
        let dist = max_distance(monster.x, monster.y, target.x, target.y);
        if dist > view_range {
            return;
        }

        if ctx.tick_count >= monster.next_attack_tick {
            // C# Attack：远程攻速 +500ms（5 tick）
            let ranged = dist > 1;
            monster.next_attack_tick = ctx.tick_count + monster.ai_profile.attack_cooldown + if ranged { 5 } else { 0 };
            let damage = crate::combat::attack::get_attack_power(monster.min_dmg, monster.max_dmg, monster.luck).max(1);
            let dir = direction_towards(monster.x, monster.y, target.x, target.y);
            monster.direction = dir;
            if ranged {
                ctx.out_attacks.push(crate::actors::world::ai::AttackAction::Range {
                    attacker_oid: monster.object_id,
                    target_session: target.session_id,
                    target_object_id: target.object_id,
                    damage,
                    spell_id: 0,
                });
            } else {
                ctx.out_attacks.push(crate::actors::world::ai::AttackAction::Melee {
                    attacker_oid: monster.object_id,
                    target_session: target.session_id,
                    damage,
                    spell_id: 0,
                    attack_type: 0,
                });
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn cannot_move() {
        // C# PurpleFaeFlower.CanMove=false
        assert!(!PurpleFaeFlowerBehavior::new().can_move());
    }
}
