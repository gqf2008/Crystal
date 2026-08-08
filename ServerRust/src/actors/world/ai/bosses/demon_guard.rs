//! DemonGuard（恶魔守卫）behavior
//!
//! C# 参考：Server/MirObjects/Monsters/DemonGuard.cs（继承 ZumaMonster）
//! 机制：2/3 物理近战（DC，ACAgility）/ 1/3 魔法近战（MC，ACAgility）；
//!      复活：LifeCount=random(0..3) 次，每次复活 HP=MaxHP*(100-25*次数)/100
//! #1360：最终死亡经验按 C# Experience override 衰减（base * (100 - 25*RevivalCount) / 100）
//! #1369：C# Revive 4-20s（40-240 tick）延迟——死亡后保留尸体（不移除），到期按比例复活并 ObjectShow

use crate::actors::world::ai::behavior::MonsterBehavior;
use crate::actors::world::ai::ctx::AiCtx;
use crate::actors::world::ai::helpers::*;
use crate::actors::world::MonsterState;

const VIEW_RANGE: i32 = 12;

pub struct DemonGuardBehavior {
    revival_count: u32,
    life_count: u32,
    /// #1369：下次复活 tick（0=未排期；C# RevivalTime = (4+Random(20))*1000）
    pending_revive_tick: u64,
    /// #1369：首次死亡是否已广播 ObjectDied（避免每 tick 重复广播）
    death_announced: bool,
}

impl DemonGuardBehavior {
    pub fn new() -> Self {
        // C#：LifeCount = Envir.Random.Next(3)（0-2 次复活）
        Self {
            revival_count: 0,
            life_count: fastrand::i32(0..3) as u32,
            pending_revive_tick: 0,
            death_announced: false,
        }
    }

    /// #1369：是否处于"死亡待复活"（尸体保留）状态
    pub(crate) fn has_pending_revive(&self) -> bool {
        self.pending_revive_tick > 0
    }

    /// #1369：标记死亡已广播；返回是否为首次（tick.rs 死亡处理首次发 ObjectDied）
    pub(crate) fn mark_death_announced(&mut self) -> bool {
        let first = !self.death_announced;
        self.death_announced = true;
        first
    }
}

/// #1369：C# DemonGuard 复活 HP = MaxHP * (100 - 25*RevivalCount) / 100（最小 1）
fn revived_hp(max_hp: i32, revival_count: u32) -> i32 {
    let pct = (100 - 25 * (revival_count as i32).min(3)).max(0);
    (max_hp * pct / 100).max(1)
}

/// #1360：C# DemonGuard.Experience = Info.Experience * (100 - 25*RevivalCount) / 100
fn revived_xp(base_xp: i32, revival_count: u32) -> i32 {
    let pct = (100 - 25 * (revival_count as i32).min(3)).max(0);
    base_xp * pct / 100
}

impl MonsterBehavior for DemonGuardBehavior {
    fn process_tick(&mut self, monster: &mut MonsterState, ctx: &mut AiCtx) {
        // #1369：C# ProcessAI——死亡且 RevivalTime 到且次数未满 → Revive（延迟复活）
        if monster.hp <= 0 {
            if self.revival_count < self.life_count {
                if self.pending_revive_tick == 0 {
                    // C# Die：RevivalTime = (4 + Random(20)) * 1000 → 40~240 tick（100ms/tick）
                    self.pending_revive_tick = ctx.tick_count + 40 + fastrand::u64(0..200);
                    return;
                }
                if ctx.tick_count >= self.pending_revive_tick {
                    // C# Revive(newhp, false)：次数+1、按公式 HP、ObjectShow 现身
                    self.revival_count += 1;
                    self.pending_revive_tick = 0;
                    self.death_announced = false;
                    monster.hp = revived_hp(monster.max_hp, self.revival_count);
                    ctx.out_show_hide.push((monster.object_id, true));
                }
                return;
            }
            // #1360：最终死亡——按 C# Experience override 衰减经验（幂等；死亡管线随后读取 monster.xp）
            monster.xp = revived_xp(monster.xp, self.revival_count);
            return;
        }
        let target = match ctx.nearest_target(monster.x, monster.y, VIEW_RANGE, monster.map_index) {
            Some(t) => *t,
            None => return,
        };
        monster.target_session = Some(target.session_id);
        let dist = max_distance(monster.x, monster.y, target.x, target.y);

        if dist <= 1 && ctx.tick_count >= monster.next_attack_tick {
            monster.next_attack_tick = ctx.tick_count + monster.ai_profile.attack_cooldown;
            let damage = crate::combat::attack::get_attack_power(
                monster.min_dmg,
                monster.max_dmg,
                monster.luck,
            )
            .max(1);
            // C# Random.Next(3) > 0：2/3 物理 / 1/3 魔法
            let magic = fastrand::i32(0..3) == 0;
            ctx.out_attacks
                .push(crate::actors::world::ai::AttackAction::Melee {
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_revived_xp_matches_csharp() {
        // C#：base * (100 - 25*count) / 100（整数除法）
        assert_eq!(revived_xp(1000, 0), 1000);
        assert_eq!(revived_xp(1000, 1), 750);
        assert_eq!(revived_xp(1000, 2), 500);
        assert_eq!(revived_xp(1000, 3), 250);
        // 边界：count 超过 3 按 3 计算，且不为负
        assert_eq!(revived_xp(1000, 9), 250);
        assert_eq!(revived_xp(10, 1), 7);
    }

    #[test]
    fn test_revived_hp_matches_csharp() {
        // C#：MaxHP * (100 - 25*count) / 100，最小 1
        assert_eq!(revived_hp(4000, 0), 4000);
        assert_eq!(revived_hp(4000, 1), 3000);
        assert_eq!(revived_hp(4000, 2), 2000);
        assert_eq!(revived_hp(4000, 3), 1000);
        assert_eq!(revived_hp(4000, 9), 1000); // count>3 按 3
        assert_eq!(revived_hp(1, 3), 1); // 最小 1
    }

    #[test]
    fn test_revive_delay_range() {
        // C# RevivalTime = (4 + Random(20)) * 1000 → 40~240 tick
        for _ in 0..500 {
            let d = 40 + fastrand::u64(0..200);
            assert!((40..=239).contains(&d), "delay out of range: {}", d);
        }
    }
}
