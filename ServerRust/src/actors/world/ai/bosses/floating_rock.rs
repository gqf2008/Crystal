//! FloatingRock（浮石）behavior
//!
//! C# 参考：Server/MirObjects/Monsters/FloatingRock.cs
//! 机制：CanMove=false；Die：FindAllTargets(3) AOE（AC）
//! #1396：ProcessTarget 克隆——视野内随机怪物（AI!=自身）1/3 概率，±5 随机落点生成克隆（is_slave，死亡清理）

use crate::actors::world::ai::behavior::MonsterBehavior;
use crate::actors::world::ai::ctx::AiCtx;
use crate::actors::world::ai::helpers::*;
use crate::actors::world::MonsterState;

const VIEW_RANGE: i32 = 12;
const DEATH_RADIUS: i32 = 3;
/// C# 克隆概率（Random.Next(3)==0 → 1/3）
const CLONE_CHANCE: i32 = 3;
/// C# 克隆落点范围（CurrentLocation ±5）
const CLONE_SPREAD: i32 = 5;

/// #1396：克隆随机落点（C# 4 次尝试 ±5，避开自身）
fn random_clone_pos(x: i32, y: i32) -> (i32, i32) {
    let mut nx = x;
    let mut ny = y;
    for _ in 0..4 {
        nx = x + fastrand::i32(-CLONE_SPREAD..=CLONE_SPREAD);
        ny = y + fastrand::i32(-CLONE_SPREAD..=CLONE_SPREAD);
        if (nx, ny) != (x, y) {
            break;
        }
    }
    (nx, ny)
}

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

    fn process_tick(&mut self, monster: &mut MonsterState, ctx: &mut AiCtx) {
        if ctx.tick_count < monster.next_attack_tick {
            return;
        }
        monster.next_attack_tick = ctx.tick_count + monster.ai_profile.attack_cooldown;
        // #1396：C# ProcessTarget——视野内随机怪物（Race==Monster，AI != 自身 AI）→ 1/3 概率克隆
        let candidates: Vec<crate::actors::world::ai::MonsterSnap> = ctx
            .monsters
            .iter()
            .filter(|m| m.map_index == monster.map_index)
            .filter(|m| max_distance(monster.x, monster.y, m.x, m.y) <= VIEW_RANGE)
            .filter(|m| m.monster_index != monster.monster_index)
            .copied()
            .collect();
        if candidates.is_empty() {
            return;
        }
        let target = candidates[fastrand::usize(0..candidates.len())];
        if fastrand::i32(0..CLONE_CHANCE) != 0 {
            return;
        }
        let Some(name) = ctx
            .monster_name_by_index
            .get(&target.monster_index)
            .cloned()
        else {
            return;
        };
        let (cx, cy) = random_clone_pos(monster.x, monster.y);
        ctx.out_summons.push(crate::actors::world::ai::BossSummon {
            monster_name: name,
            x: cx,
            y: cy,
            is_slave: true, // C# SlaveList：死亡时清理
            summoner_oid: Some(monster.object_id),
        });
    }

    /// C# Die：AOE 3 伤害（AC）
    fn on_die(&mut self, monster: &mut MonsterState, ctx: &mut AiCtx) {
        let damage =
            crate::combat::attack::get_attack_power(monster.min_dmg, monster.max_dmg, 0).max(1);
        ctx.out_attacks
            .push(crate::actors::world::ai::AttackAction::Aoe {
                attacker_oid: monster.object_id,
                center_x: monster.x,
                center_y: monster.y,
                radius: DEATH_RADIUS,
                damage,
                spell_id: 0,
            });
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_random_clone_pos_within_spread() {
        // C#：落点在自身 ±5 内
        for _ in 0..2000 {
            let (nx, ny) = random_clone_pos(100, 100);
            assert!((nx - 100).abs() <= CLONE_SPREAD, "x out of spread: {}", nx);
            assert!((ny - 100).abs() <= CLONE_SPREAD, "y out of spread: {}", ny);
        }
    }

    #[test]
    fn test_random_clone_pos_avoids_self() {
        let mut non_self = 0;
        for _ in 0..2000 {
            let (nx, ny) = random_clone_pos(100, 100);
            if (nx, ny) != (100, 100) {
                non_self += 1;
            }
        }
        assert!(non_self > 0, "clone pos never moved off self");
    }
}
