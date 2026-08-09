//! DragonStatue（龙雕像）behavior
//!
//! C# 参考：Server/MirObjects/Monsters/DragonStatue.cs
//! 机制：CanMove=false；攻击：FindAllTargets(2, 目标) AOE（MAC）
//! #1399：睡眠——死亡后保留尸体（免疫、不索敌不攻击），15 分钟（9000 tick）后满血苏醒 + ObjectShow

use crate::actors::world::ai::behavior::MonsterBehavior;
use crate::actors::world::ai::ctx::AiCtx;
use crate::actors::world::ai::helpers::*;
use crate::actors::world::MonsterState;

const VIEW_RANGE: i32 = 12;
const AOE_RADIUS: i32 = 2;
/// #1399：C# WakeDelay = 15 * 60 * 1000ms → 9000 tick（100ms/tick）
const SLEEP_TICKS: u64 = 15 * 60 * 10;

pub struct DragonStatueBehavior {
    /// 是否睡眠中（C# Sleeping；死亡后入睡，15 分钟满血苏醒）
    sleeping: bool,
    /// 苏醒 tick
    wake_up_tick: u64,
    /// 死亡是否已广播（首次发 ObjectDied）
    death_announced: bool,
}

impl DragonStatueBehavior {
    pub fn new() -> Self {
        Self {
            sleeping: false,
            wake_up_tick: 0,
            death_announced: false,
        }
    }
}

impl MonsterBehavior for DragonStatueBehavior {
    fn can_move(&self) -> bool {
        false
    }

    /// #1399：睡眠期免疫（C# Attacked 返 0）
    fn is_attackable(&self) -> bool {
        !self.sleeping
    }

    fn on_attacked(&mut self, damage: i32) -> i32 {
        if self.sleeping {
            0
        } else {
            damage
        }
    }

    /// #1399：死亡后保留尸体等待苏醒
    fn keep_corpse_for_revive(&self) -> bool {
        self.sleeping
    }

    /// #1399：标记死亡已广播；返回是否首次
    fn mark_death_announced(&mut self) -> bool {
        let first = !self.death_announced;
        self.death_announced = true;
        first
    }

    fn process_tick(&mut self, monster: &mut MonsterState, ctx: &mut AiCtx) {
        // #1399：入睡（C# Die → Sleeping=true）；死亡保留尸体（tick.rs 死亡处理）
        if monster.hp <= 0 && !self.sleeping {
            self.sleeping = true;
            self.wake_up_tick = ctx.tick_count + SLEEP_TICKS;
            self.death_announced = false;
            return;
        }
        // #1399：苏醒（C# ProcessAI：到时 → Sleeping=false + HP=MaxHP）
        if self.sleeping {
            if monster.hp > 0 || ctx.tick_count >= self.wake_up_tick {
                self.sleeping = false;
                self.wake_up_tick = 0;
                monster.hp = monster.max_hp;
                ctx.out_show_hide.push((monster.object_id, true));
            }
            return;
        }
        let target = match ctx.nearest_target(monster.x, monster.y, VIEW_RANGE, monster.map_index) {
            Some(t) => *t,
            None => return,
        };
        monster.target_session = Some(target.session_id);

        if ctx.tick_count >= monster.next_attack_tick {
            monster.next_attack_tick = ctx.tick_count + monster.ai_profile.attack_cooldown;
            let damage = crate::combat::attack::get_attack_power(
                monster.min_dmg,
                monster.max_dmg,
                monster.luck,
            )
            .max(1);
            // C# CompleteAttack：FindAllTargets(2, Target.CurrentLocation)
            ctx.out_attacks
                .push(crate::actors::world::ai::AttackAction::Aoe {
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn sleep_ticks_matches_csharp_wake_delay() {
        // C# WakeDelay = 15 * (1000 * 60) ms；100ms/tick → 9000
        assert_eq!(SLEEP_TICKS, 9000);
    }
}
